use spin_sdk::http::{Request, Response};
use uuid::Uuid;
use crate::models::models::{User, TokenData};
use crate::config::{token_expiration_hours, USERS_LIST_KEY, TOKENS_LIST_KEY, user_key, token_key};
use crate::core::helpers::{store, verify_password, validate_uuid, now_iso, unauthorized};

pub fn login_user(req: Request) -> anyhow::Result<Response> {
    let store = store();
    let creds: serde_json::Value = serde_json::from_slice(req.body())?;
    let username = creds["username"].as_str().unwrap_or_default();
    let password = creds["password"].as_str().unwrap_or_default();

    let users: Vec<String> = store.get_json(USERS_LIST_KEY)?.unwrap_or_default();

    for id in users {
        if let Some(u) = store.get_json::<User>(&user_key(&id))? {
            if u.id.is_empty() || !validate_uuid(&u.id) {
                return Ok(unauthorized());
            }
            if u.username == username && verify_password(password, &u.password) {
                let token = Uuid::new_v4().to_string();
                let data = TokenData {
                    user_id: u.id.clone(),
                    created_at: now_iso(),
                };
                store.set_json(&token_key(&token), &data)?;
                
                // Track token in central list
                let mut tokens: Vec<String> = store.get_json(TOKENS_LIST_KEY)?.unwrap_or_default();
                tokens.push(token.clone());
                store.set_json(TOKENS_LIST_KEY, &tokens)?;

                let resp = serde_json::json!({
                    "token": token,
                    "user_id": u.id
                });
                return Ok(Response::builder()
                    .status(200)
                    .header("Content-Type", "application/json")
                    .body(serde_json::to_vec(&resp)?)
                    .build());
            }
        }
    }

    Ok(unauthorized())
}

pub fn logout_user(req: Request) -> anyhow::Result<Response> {
    let store = store();
    let auth_header = req.header("Authorization").and_then(|h| h.as_str()).unwrap_or_default();
    
    if !auth_header.starts_with("Bearer ") {
        return Ok(unauthorized());
    }
    
    let token = auth_header.strip_prefix("Bearer ").unwrap();
    let key = token_key(token);
    store.delete(&key)?;
    
    // Remove from central list
    let mut tokens: Vec<String> = store.get_json(TOKENS_LIST_KEY)?.unwrap_or_default();
    tokens.retain(|t| t != token);
    store.set_json(TOKENS_LIST_KEY, &tokens)?;
    
    let resp = serde_json::json!({
        "message": "Logged out successfully"
    });
    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .body(serde_json::to_vec(&resp)?)
        .build())
}

pub fn validate_token(req: &Request) -> Option<String> {
    let store = store();
    let auth_header = req.header("Authorization")?.as_str().unwrap_or_default();
    if !auth_header.starts_with("Bearer ") {
        return None;
    }
    let token = auth_header.strip_prefix("Bearer ").unwrap();
    let key = token_key(token);
    if let Some(data) = store.get_json::<TokenData>(&key).ok()? {
        // Check if token is expired
        if let Ok(created) = chrono::DateTime::parse_from_rfc3339(&data.created_at) {
            let now = chrono::Utc::now();
            let age_hours = (now - created.with_timezone(&chrono::Utc)).num_hours();
            if age_hours > token_expiration_hours() {
                return None;
            }
        }
        // Check if user still exists
        let user_key = user_key(&data.user_id);
        if store.get_json::<User>(&user_key).ok()?.is_none() {
            return None;
        }
        Some(data.user_id)
    } else {
        None
    }
}
