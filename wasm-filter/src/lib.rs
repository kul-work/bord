use spin_sdk::{
    http::{Request, Response, IntoResponse, Method},
    http_component,
};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

mod tokenizer;
mod tract_model;

#[derive(Debug, Deserialize)]
struct Config {
    #[serde(default)]
    enable_llm: bool,
    #[serde(default)]
    enable_tract: bool,
    llm: LlmConfig,
    llm_prompt: PromptConfig,
    policy: PolicyConfig,
}

#[derive(Debug, Deserialize)]
struct LlmConfig {
    address: String,
    model: String,
    #[allow(dead_code)]
    temperature: f64,
}

#[derive(Debug, Deserialize)]
struct PromptConfig {
    sentiment_analysis: String,
}

#[derive(Debug, Deserialize)]
struct PolicyConfig {
    sentiment_score_threshold: f64,
}

static CONFIG: OnceLock<Config> = OnceLock::new();

fn load_config() -> &'static Config {
    CONFIG.get_or_init(|| {
        let config_str = include_str!("../config.toml");
        toml::from_str(config_str).expect("Failed to parse config.toml")
    })
}

#[derive(Debug, Serialize)]
struct LlmRequest {
    model: String,
    prompt: String,
    stream: bool,
}

#[derive(Debug, Deserialize)]
struct LlmResponse {
    response: String,
}

#[derive(Debug, Deserialize)]
struct LlmClassification {
    sentiment_score: f64,
    has_hate_speech: bool,
    reason: String,
}

#[derive(Debug, Clone)]
struct ContentClassification {
    sentiment_score: f64, // 0.0 (negative) to 1.0 (positive)
    is_hate_speech: bool,
    #[allow(dead_code)]
    reasoning: String,
}

/// Call LLM API for sentiment analysis
async fn classify_with_llm(content: &str) -> anyhow::Result<ContentClassification> {
    let config = load_config();
    
    // Sentiment analysis prompt
    let prompt = format!("{}", config.llm_prompt.sentiment_analysis.replace("{}", content));
    
    let req_body = LlmRequest {
        model: config.llm.model.clone(),
        prompt,
        stream: false,
    };
    
    let request = Request::builder()
        .method(Method::Post)
        .uri(format!("{}/api/generate", config.llm.address))
        .header("Content-Type", "application/json")
        .body(serde_json::to_vec(&req_body)?)
        .build();
    
    match spin_sdk::http::send::<Request, Response>(request).await {
        Ok(response) => {
            let body_str = String::from_utf8_lossy(&response.body());
            
            // Parse LLM response
            if let Ok(llm_resp) = serde_json::from_str::<LlmResponse>(&body_str) {
                eprintln!("[LLM DEBUG] Raw response: {}", llm_resp.response);
                
                // Try to parse JSON from the response
                if let Ok(classification) = serde_json::from_str::<LlmClassification>(&llm_resp.response) {
                    //eprintln!("[LLM DEBUG] Parsed JSON: sentiment={}, hate_speech={}", classification.sentiment_score, classification.has_hate_speech);
                    eprintln!("[LLM] Content classified: sentiment={}, hate_speech={}", classification.sentiment_score, classification.has_hate_speech);
                    
                    Ok(ContentClassification {
                        sentiment_score: classification.sentiment_score,
                        is_hate_speech: classification.has_hate_speech,
                        reasoning: classification.reason,
                    })
                } else {
                    Err(anyhow::anyhow!("Failed to parse JSON from LLM response"))
                }
            } else {
                Err(anyhow::anyhow!("Failed to parse LLM response"))
            }
        }
        Err(e) => {
            eprintln!("[LLM ERROR] LLM call failed: {}", e);
            // Fallback: if LLM is down, allow request (graceful degradation)
            Ok(ContentClassification {
                sentiment_score: 0.5,
                is_hate_speech: false,
                reasoning: "llm_unavailable".to_string(),
            })
        }
    }
}

/// Call Tract for sentiment classification
fn classify_with_tract(content: &str) -> anyhow::Result<ContentClassification> {
    match tract_model::classify_sentiment(content) {
        Ok(sentiment_score) => {
            eprintln!("[TRACT] Sentiment score: {}", sentiment_score);
            
            // Simple heuristic: score < 0.4 = negative (potential hate speech/toxicity)
            let is_hate_speech = sentiment_score < 0.3;
            
            Ok(ContentClassification {
                sentiment_score,
                is_hate_speech,
                reasoning: "tract_inference".to_string(),
            })
        }
        Err(e) => {
            eprintln!("[TRACT ERROR] Inference failed: {}", e);
            // Fallback: allow if model fails
            Ok(ContentClassification {
                sentiment_score: 0.5,
                is_hate_speech: false,
                reasoning: "tract_error".to_string(),
            })
        }
    }
}

/// Check if content contains forbidden words
fn contains_forbidden_content(content: &str) -> Option<String> {
    let forbidden_words = match std::env::var("FORBIDDEN_WORDS") {
        Ok(words) => words,
        Err(_) => return None, // No forbidden words configured
    };
    
    for word in forbidden_words.split(',') {
        let word = word.trim().to_lowercase();
        if !word.is_empty() && content.to_lowercase().contains(&word) {
            eprintln!("[SPAM DETECTED] Forbidden word found: {}", word);
            return Some("Spam detected - this content won't be posted.".to_string());
        }
    }
    None
}

/// Build error response for content policy violations
fn build_error_response(message: &str) -> Response {
    let error_body = serde_json::json!({
        "error": "content_policy_violation",
        "message": message
    });
    
    Response::builder()
        .status(422)
        .header("Content-Type", "application/json")
        .body(serde_json::to_vec(&error_body).unwrap_or_default())
        .build()
}

/// Extract content from POST/PUT request body for validation
fn validate_post_content(body: &[u8]) -> Option<String> {
    if let Ok(json) = serde_json::from_slice::<serde_json::Value>(body) {
        if let Some(content) = json.get("content").and_then(|v| v.as_str()) {
            return Some(content.to_string());
        }
    }
    None
}

/// Proxy that injects X-Origin header, validates content policy, and forwards requests to the main Bord app
#[http_component]
async fn handle(req: Request) -> anyhow::Result<impl IntoResponse> {
    let method = req.method().clone();
    let path = req.path_and_query().unwrap_or("/").to_string();
    let body = req.body().to_vec();
    
    // Validate content for POST /posts and PUT /posts/* requests
    let method_str = method.to_string();
    if (method_str == "POST" && path.starts_with("/posts")) || 
       (method_str == "PUT" && path.starts_with("/posts/")) {
        if let Some(content) = validate_post_content(&body) {
            // 1. Fast check: forbidden words
            if let Some(error_msg) = contains_forbidden_content(&content) {
                return Ok(build_error_response(&error_msg));
            }
            
            // 2. ML-based sentiment/hate speech detection (configurable: LLM or Tract)
            let config = load_config();
            if config.enable_llm {
                match classify_with_llm(&content).await {
                    Ok(classification) => {
                        // Block if hate speech detected or sentiment too negative
                        if classification.is_hate_speech {
                            eprintln!("[POLICY] Blocked: hate speech detected");
                            return Ok(build_error_response("Content contains hate speech"));
                        }
                        if classification.sentiment_score < config.policy.sentiment_score_threshold {
                            eprintln!("[POLICY] Flagged: very negative sentiment ({})", classification.sentiment_score);
                            // Log but allow (you can change this to block if needed)
                        }
                    }
                    Err(e) => {
                        eprintln!("[POLICY] LLM classification failed: {}, allowing request", e);
                        // Graceful degradation: allow if LLM is down
                    }
                }
            } else if config.enable_tract {
                match classify_with_tract(&content) {
                    Ok(classification) => {
                        if classification.is_hate_speech {
                            eprintln!("[POLICY] Blocked: negative sentiment detected");
                            return Ok(build_error_response("Content sentiment too negative"));
                        }
                        if classification.sentiment_score < config.policy.sentiment_score_threshold {
                            eprintln!("[POLICY] Flagged: very negative sentiment ({})", classification.sentiment_score);
                            // Log but allow
                        }
                    }
                    Err(e) => {
                        eprintln!("[POLICY] Tract classification failed: {}, allowing request", e);
                        // Graceful degradation: allow if model fails
                    }
                }
            }
        }
    }
    
    // Create URL inline with the builder call
    let target_host = std::env::var("BORD_TARGET").expect("BORD_TARGET environment variable must be set");
    let url = format!("{}{}", target_host, path);
    
    // Build request step by step, keeping everything in scope
    let mut builder = Request::builder();
    builder.method(method);
    builder.uri(url);
    
    // Add headers from incoming request
    for (name, value) in req.headers() {
        let val_str = String::from_utf8_lossy(value.as_ref()).to_string();
        builder.header(name, val_str);
    }
    
    // Add our header
    builder.header("x-origin", "wasm-filter");
    
    // Build and send
    let req_to_send = builder.body(body).build();
    let response: Response = spin_sdk::http::send(req_to_send).await?;
    
    Ok(response)
}
