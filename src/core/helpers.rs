use spin_sdk::http::{Request, Response};
use spin_sdk::key_value::Store;
use argon2::{Argon2, PasswordHasher, PasswordVerifier};
use argon2::password_hash::SaltString;
use rand::rngs::OsRng;
use crate::auth::validate_token;

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
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    
    argon2
        .hash_password(password.as_bytes(), &salt)
        .expect("Failed to hash password")
        .to_string()
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    use argon2::PasswordHash;
    
    let parsed_hash = match PasswordHash::new(hash) {
        Ok(h) => h,
        Err(_) => return false,
    };
    
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
}

pub fn require_auth(req: &Request) -> anyhow::Result<String> {
    validate_token(req).ok_or_else(|| anyhow::anyhow!("Unauthorized"))
}
