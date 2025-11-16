use spin_sdk::http::Response;
use spin_sdk::key_value::Store;
use sha2::{Sha256, Digest};

pub fn store() -> Store {
    Store::open_default().expect("KV store must exist")
}

pub fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339()
}

pub fn unauthorized() -> Response {
    Response::builder().status(401).body("Unauthorized").build()
}

pub fn hash_password(password: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    format!("{:x}", hasher.finalize())
}
