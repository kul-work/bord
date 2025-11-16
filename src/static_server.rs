use spin_sdk::http::Response;
use rust_embed::RustEmbed;
use mime_guess::from_path;

#[derive(RustEmbed)]
#[folder = "static"]
struct Assets;

pub fn serve_static(path: &str) -> anyhow::Result<Response> {
    let file_path = match path {
        "/" => "index.html",
        "/index.html" => "index.html",
        _ => path.trim_start_matches('/'),
    };

    let file = Assets::get(file_path)
        .ok_or_else(|| anyhow::anyhow!("File not found"))?;

    let mime = from_path(file_path).first_or_octet_stream();

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", mime.as_ref())
        .body(file.data.to_vec())
        .build())
}
