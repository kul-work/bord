use spin_sdk::http::{Request, Response};
use uuid::Uuid;
use ammonia::Builder;
use crate::models::models::{User, TokenData};
use crate::core::helpers::{store, hash_password, verify_password, validate_uuid, now_iso};
use crate::core::errors::ApiError;
use crate::auth::validate_token;
use crate::config::*;


fn sanitize_text(text: &str) -> String {
    // Sanitize to plain text only - no HTML allowed
    // Use ammonia with all tags disabled to strip HTML
    Builder::default()
        .tags(std::collections::HashSet::new())
        .clean(text)
        .to_string()
}

fn build_user_json(user: &User) -> serde_json::Value {
    serde_json::json!({
        "id": user.id,
        "username": user.username,
        "bio": user.bio.as_ref().unwrap_or(&String::new()),
    })
}

fn get_user_by_id(user_id: &str) -> anyhow::Result<Response> {
     let store = store();
     let user_key = format!("user:{}", user_id);
     
     if let Some(user) = store.get_json::<User>(&user_key)? {
         Ok(Response::builder()
             .status(200)
             .header("Content-Type", "application/json")
             .body(serde_json::to_vec(&build_user_json(&user))?)
             .build())
     } else {
        Ok(ApiError::NotFound("User not found".to_string()).into())
     }
}

pub fn create_user(req: Request) -> anyhow::Result<Response> {
     let store = store();
     let body = req.body();
 
     let new_user: serde_json::Value = serde_json::from_slice(body)?;
     let username = new_user["username"].as_str().unwrap_or("");
     let password = new_user["password"].as_str().unwrap_or("");
 
     if username.is_empty() {
         return Ok(ApiError::BadRequest("Username is required".to_string()).into());
     }
     if username.len() < MIN_USERNAME_LENGTH || username.len() > MAX_USERNAME_LENGTH {
         return Ok(ApiError::BadRequest("Username must be 3-50 characters".to_string()).into());
     }
     if password.is_empty() {
         return Ok(ApiError::BadRequest("Password is required".to_string()).into());
     }
     if password.len() < MIN_PASSWORD_LENGTH {
         return Ok(ApiError::BadRequest("Password must be at least 3 characters".to_string()).into());
     }
 
     // Sanitize username at input time
     let sanitized_username = sanitize_text(username);
 
     // Check duplicate username
     let existing_users: Vec<String> = store.get_json("users_list")?.unwrap_or_default();
     for id in &existing_users {
         if let Some(u) = store.get_json::<User>(&format!("user:{}", id))? {
             if u.username == sanitized_username {
                 return Ok(ApiError::Conflict("Username exists".to_string()).into());
             }
         }
     }
     let id = Uuid::new_v4().to_string();
 
     let user = User {
         id: id.clone(),
         username: sanitized_username,
         password: hash_password(password)?,
         bio: None,
     };
 
     let key = format!("user:{}", id);
     store.set_json(&key, &user)?;
 
     // Add to users_list
     let mut users = existing_users;
     users.push(id.clone());
     store.set_json("users_list", &users)?;
 
     Ok(Response::builder()
         .status(201)
         .header("Content-Type", "application/json")
         .body(serde_json::to_vec(&user)?)
         .build())
 }

pub fn get_profile(req: Request) -> anyhow::Result<Response> {
    let user_id = match validate_token(&req) {
        Some(uid) => uid,
        None => return Ok(ApiError::Unauthorized.into()),
    };

    get_user_by_id(&user_id)
}

pub fn get_user_details(path: &str) -> anyhow::Result<Response> {
    let user_id = path.trim_start_matches("/users/");
    
    if user_id.is_empty() || !validate_uuid(user_id) {
        return Ok(ApiError::BadRequest("User ID required".to_string()).into());
    }

    get_user_by_id(user_id)
}

pub fn update_profile(req: Request) -> anyhow::Result<Response> {
     let user_id = match validate_token(&req) {
         Some(uid) => uid,
         None => return Ok(ApiError::Unauthorized.into()),
     };
 
     let store = store();
     let user_key = format!("user:{}", user_id);
 
     if let Some(mut user) = store.get_json::<User>(&user_key)? {
         let value: serde_json::Value = serde_json::from_slice(req.body())?;
         let mut password_changed = false;
 
         // Update bio if provided
         if let Some(bio) = value["bio"].as_str() {
             if bio.len() > MAX_BIO_LENGTH {
                 return Ok(ApiError::BadRequest("Bio too long (max 500 chars)".to_string()).into());
             }
             // Sanitize bio at input time
             let sanitized_bio = sanitize_text(bio);
             user.bio = if sanitized_bio.is_empty() { None } else { Some(sanitized_bio) };
         }
 
         // Update password if provided
         if let Some(new_password) = value["new_password"].as_str() {
            if new_password.is_empty() || new_password.len() < 3 {
                return Ok(ApiError::BadRequest("Password must be 3+ characters".to_string()).into());
            }
            
            let old_password = value["old_password"].as_str()
                .ok_or_else(|| ApiError::BadRequest("Current password required".to_string()))?;
            
            if !verify_password(old_password, &user.password) {
                return Ok(ApiError::Unauthorized.into());
            }
            
            user.password = hash_password(new_password)?;
            password_changed = true;
         }
 
         store.set_json(&user_key, &user)?;
 
         // If password changed, invalidate all tokens for this user and issue a new one
         let mut response_data = build_user_json(&user);
         if password_changed {
             let all_tokens: Vec<String> = store.get_json("tokens_list")?.unwrap_or_default();
             
             // Filter out tokens for this user and delete them
             let filtered_tokens: Vec<String> = all_tokens
                 .into_iter()
                 .filter(|token| {
                     let token_key = format!("token:{}", token);
                     if let Ok(Some(token_data)) = store.get_json::<TokenData>(&token_key) {
                         if token_data.user_id == user_id {
                             // Delete token from store
                             let _ = store.delete(&token_key);
                             false // Exclude from filtered list
                         } else {
                             true // Keep token from other users
                         }
                     } else {
                         true // Keep if we can't read it
                     }
                 })
                 .collect();
             store.set_json("tokens_list", &filtered_tokens)?;
             
             // Generate new token
             let new_token = Uuid::new_v4().to_string();
             let token_data = TokenData {
                 user_id: user_id.clone(),
                 created_at: now_iso(),
             };
             store.set_json(&format!("token:{}", new_token), &token_data)?;
             
             // Add to tokens_list
             let mut updated_tokens = filtered_tokens;
             updated_tokens.push(new_token.clone());
             store.set_json("tokens_list", &updated_tokens)?;
             
             // Include new token in response
             response_data["token"] = serde_json::Value::String(new_token);
         }
 
         Ok(Response::builder()
             .status(200)
             .header("Content-Type", "application/json")
             .body(serde_json::to_vec(&response_data)?)
             .build())
     } else {
         Ok(ApiError::NotFound("User not found".to_string()).into())
     }
}