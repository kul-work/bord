use spin_sdk::{
    http::{Request, Response, IntoResponse},
    http_component,
};

/// Proxy that injects X-Origin header and forwards requests to the main Bord app
#[http_component]
async fn handle(req: Request) -> anyhow::Result<impl IntoResponse> {
    let method = req.method().clone();
    let path = req.path_and_query().unwrap_or("/").to_string();
    let body = req.body().to_vec();
    
    // Create URL inline with the builder call
    let target_host = std::env::var("BORD_TARGET").unwrap_or_else(|_| "http://127.0.0.1:3001".to_string());
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
