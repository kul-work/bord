use crate::tokenizer::Tokenizer;
use tract_onnx::prelude::*;

/// Run inference on tokenized input
pub fn classify_sentiment(text: &str) -> anyhow::Result<f64> {
    let tokenizer = Tokenizer::load()?;
    
    // Tokenize input
    let token_ids = tokenizer.tokenize(text);
    
    // Create attention mask (1 for real tokens, 0 for padding)
    let attention_mask: Vec<i64> = token_ids
        .iter()
        .map(|&id| if id == 0 { 0 } else { 1 })
        .collect();
    
    eprintln!("[TRACT] Loading and running model...");
    
    let model_bytes = include_bytes!("../models/model.onnx");
    
    // Parse ONNX from bytes
    let graph = tract_onnx::onnx()
        .model_for_read(&mut std::io::Cursor::new(model_bytes))?;
    
    let model = graph.into_runnable()?;
    
    // Create tensors using tract macros
    eprintln!("[TRACT] Creating tensors...");
    
    // Convert to fixed-size arrays for tensor2 macro (batch_size=1)
    let mut input_array = [0i64; 128];
    let mut mask_array = [0i64; 128];
    let len = token_ids.len().min(128);
    input_array[..len].copy_from_slice(&token_ids[..len]);
    mask_array[..len].copy_from_slice(&attention_mask[..len]);
    
    eprintln!("[TRACT] Tensor shape: ({}, {})", 1, 128);
    eprintln!("[TRACT] First few input tokens: {:?}", &input_array[..10]);
    
    let input_tensor = tensor2(&[input_array]);
    let mask_tensor = tensor2(&[mask_array]);
    
    eprintln!("[TRACT] Input tensor shape: {:?}", input_tensor.shape());
    eprintln!("[TRACT] Mask tensor shape: {:?}", mask_tensor.shape());
    
    // Run model
    eprintln!("[TRACT] Running inference...");
    let outputs = model.run(tvec![
        input_tensor.into(),
        mask_tensor.into()
    ])?;
    
    // Extract logits from output
    let logits = &outputs[0];
    let logits_view = match logits.as_slice::<f32>() {
        Ok(slice) => slice,
        Err(_) => return Err(anyhow::anyhow!("Failed to get logits as f32 slice")),
    };
    
    // DistilBERT returns shape (batch_size, num_classes) where num_classes=2 (negative, positive)
    // We want the positive class score
    if logits_view.len() < 2 {
        return Err(anyhow::anyhow!("Unexpected model output shape: expected at least 2 logits"));
    }
    
    let negative_score = logits_view[0];
    let positive_score = logits_view[1];
    
    // Convert logits to probability via softmax-like approach
    let sentiment_score = sigmoid(positive_score - negative_score);
    
    eprintln!(
        "[TRACT] Inference complete: positive_logit={}, negative_logit={}, sentiment_score={}",
        positive_score, negative_score, sentiment_score
    );
    
    Ok(sentiment_score)
}

/// Simple sigmoid for probability normalization
fn sigmoid(x: f32) -> f64 {
    (1.0 / (1.0 + (-x).exp())) as f64
}
