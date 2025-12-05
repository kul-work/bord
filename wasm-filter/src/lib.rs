use spin_sdk::{
    http::{Request, Response, IntoResponse},
    http_component,
};

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
            if let Some(error_msg) = contains_forbidden_content(&content) {
                return Ok(build_error_response(&error_msg));
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
