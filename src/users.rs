use spin_sdk::http::{Request, Response};
use uuid::Uuid;
use ammonia::Builder;
use crate::models::models::User;
use crate::core::helpers::{store, hash_password, verify_password, unauthorized, validate_uuid};
use crate::auth::validate_token;


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
        Ok(Response::builder().status(404).body("User not found").build())
    }
}

pub fn create_user(req: Request) -> anyhow::Result<Response> {
     let store = store();
     let body = req.body();
 
     let new_user: serde_json::Value = serde_json::from_slice(body)?;
     let username = new_user["username"].as_str().unwrap_or("");
     let password = new_user["password"].as_str().unwrap_or("");
 
     if username.is_empty() {
         return Ok(Response::builder().status(400).body("Username is required").build());
     }
     if password.is_empty() {
         return Ok(Response::builder().status(400).body("Password is required").build());
     }
 
     // Sanitize username at input time
     let sanitized_username = sanitize_text(username);
 
     // Check duplicate username
     let existing_users: Vec<String> = store.get_json("users_list")?.unwrap_or_default();
     for id in &existing_users {
         if let Some(u) = store.get_json::<User>(&format!("user:{}", id))? {
             if u.username == sanitized_username {
                 return Ok(Response::builder().status(409).body("Username exists").build());
             }
         }
     }
     let id = Uuid::new_v4().to_string();
 
     let user = User {
         id: id.clone(),
         username: sanitized_username,
         password: hash_password(password),
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
        None => return Ok(unauthorized()),
    };

    get_user_by_id(&user_id)
}

pub fn get_user_details(path: &str) -> anyhow::Result<Response> {
    let user_id = path.trim_start_matches("/users/");
    
    if user_id.is_empty() || !validate_uuid(user_id) {
        return Ok(Response::builder().status(400).body("User ID required").build());
    }

    get_user_by_id(user_id)
}

pub fn update_profile(req: Request) -> anyhow::Result<Response> {
     let user_id = match validate_token(&req) {
         Some(uid) => uid,
         None => return Ok(unauthorized()),
     };
 
     let store = store();
     let user_key = format!("user:{}", user_id);
 
     if let Some(mut user) = store.get_json::<User>(&user_key)? {
         let value: serde_json::Value = serde_json::from_slice(req.body())?;
 
         // Update bio if provided
         if let Some(bio) = value["bio"].as_str() {
             if bio.len() > 500 {
                 return Ok(Response::builder().status(400).body("Bio too long (max 500 chars)").build());
             }
             // Sanitize bio at input time
             let sanitized_bio = sanitize_text(bio);
             user.bio = if sanitized_bio.is_empty() { None } else { Some(sanitized_bio) };
         }

        // Update password if provided
        if let Some(new_password) = value["new_password"].as_str() {
            if new_password.is_empty() {
                return Ok(Response::builder().status(400).body("New password cannot be empty").build());
            }
            // Verify old password if provided
             if let Some(old_password) = value["old_password"].as_str() {
                 if !verify_password(old_password, &user.password) {
                     return Ok(Response::builder().status(401).body("Invalid current password").build());
                 }
                user.password = hash_password(new_password);
            } else {
                return Ok(Response::builder().status(400).body("Current password required").build());
            }
        }

        store.set_json(&user_key, &user)?;

        Ok(Response::builder()
            .status(200)
            .header("Content-Type", "application/json")
            .body(serde_json::to_vec(&build_user_json(&user))?)
            .build())
    } else {
        Ok(Response::builder().status(404).body("User not found").build())
    }
}