use std::collections::HashMap;
use std::sync::OnceLock;

static VOCAB: OnceLock<Tokenizer> = OnceLock::new();

pub struct Tokenizer {
    vocab: HashMap<String, i64>,
}

impl Tokenizer {
    /// Load vocab.txt into memory
    pub fn load() -> anyhow::Result<&'static Tokenizer> {
        if let Some(tokenizer) = VOCAB.get() {
            return Ok(tokenizer);
        }
        
        let vocab_data = include_str!("../models/vocab.txt");
        let mut vocab = HashMap::new();
        
        for (idx, line) in vocab_data.lines().enumerate() {
            let token = line.trim().to_string();
            vocab.insert(token, idx as i64);
        }
        
        eprintln!("[TOKENIZER] Loaded {} tokens from vocab", vocab.len());
        
        let tokenizer = Tokenizer { vocab };
        VOCAB.set(tokenizer).map_err(|_| anyhow::anyhow!("Failed to initialize tokenizer"))?;
        
        VOCAB.get().ok_or_else(|| anyhow::anyhow!("Tokenizer not initialized"))
    }
    
    /// Basic BERT tokenization: lowercase, split, convert to IDs
    pub fn tokenize(&self, text: &str) -> Vec<i64> {
        let mut tokens = vec![];
        
        // Add [CLS] token
        if let Some(id) = self.vocab.get("[CLS]") {
            tokens.push(*id);
        }
        
        // Tokenize input (simple: split on whitespace + punctuation)
        let mut current_token = String::new();
        for ch in text.to_lowercase().chars() {
            if ch.is_whitespace() || is_punctuation(ch) {
                if !current_token.is_empty() {
                    tokens.push(self.get_token_id(&current_token));
                    current_token.clear();
                }
                // Handle punctuation
                if is_punctuation(ch) {
                    tokens.push(self.get_token_id(&ch.to_string()));
                }
            } else {
                current_token.push(ch);
            }
        }
        
        // Push last token
        if !current_token.is_empty() {
            tokens.push(self.get_token_id(&current_token));
        }
        
        // Add [SEP] token
        if let Some(id) = self.vocab.get("[SEP]") {
            tokens.push(*id);
        }
        
        // Pad to 128 tokens (DistilBERT expects fixed length)
        while tokens.len() < 128 {
            if let Some(id) = self.vocab.get("[PAD]") {
                tokens.push(*id);
            } else {
                tokens.push(0);
            }
        }
        
        // Truncate if longer than 128
        tokens.truncate(128);
        
        eprintln!("\x1b[33m[TOKENIZER] Sample:\x1b[0m {}", text);
        tokens
    }
    
    fn get_token_id(&self, token: &str) -> i64 {
        // Exact match
        if let Some(id) = self.vocab.get(token) {
            return *id;
        }
        
        // Handle subword tokens (##prefix)
        if let Some(id) = self.vocab.get(&format!("#{}", token)) {
            return *id;
        }
        
        // Unknown token fallback
        self.vocab.get("[UNK]").copied().unwrap_or(100)
    }
}

fn is_punctuation(ch: char) -> bool {
    matches!(ch, ',' | '.' | '!' | '?' | ';' | ':' | '"' | '\'' | '-')
}
