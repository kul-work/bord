use spin_sdk::http::{Request, Response};
use uuid::Uuid;
use crate::models::User;
use crate::helpers::{store, hash_password, unauthorized};
use crate::auth::validate_token;
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "static"]
struct Assets;

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

    // Check duplicate username
    let existing_users: Vec<String> = store.get_json("users_list")?.unwrap_or_default();
    for id in &existing_users {
        if let Some(u) = store.get_json::<User>(&format!("user:{}", id))? {
            if u.username == username {
                return Ok(Response::builder().status(409).body("Username exists").build());
            }
        }
    }
    let id = Uuid::new_v4().to_string();

    let user = User {
        id: id.clone(),
        username: username.to_string(),
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
            user.bio = if bio.is_empty() { None } else { Some(bio.to_string()) };
        }

        // Update password if provided
        if let Some(new_password) = value["new_password"].as_str() {
            if new_password.is_empty() {
                return Ok(Response::builder().status(400).body("New password cannot be empty").build());
            }
            // Verify old password if provided
            if let Some(old_password) = value["old_password"].as_str() {
                if user.password != hash_password(old_password) {
                    return Ok(Response::builder().status(401).body("Invalid current password").build());
                }
                user.password = hash_password(new_password);
            } else {
                return Ok(Response::builder().status(400).body("Current password required").build());
            }
        }

        store.set_json(&user_key, &user)?;

        let profile = serde_json::json!({
            "id": user.id,
            "username": user.username,
            "bio": user.bio.unwrap_or_default(),
        });
        Ok(Response::builder()
            .status(200)
            .header("Content-Type", "application/json")
            .body(serde_json::to_vec(&profile)?)
            .build())
    } else {
        Ok(Response::builder().status(404).body("User not found").build())
    }
}

pub fn get_profile(req: Request) -> anyhow::Result<Response> {
    let user_id = match validate_token(&req) {
        Some(uid) => uid,
        None => return Ok(unauthorized()),
    };

    let store = store();
    let user_key = format!("user:{}", user_id);

    if let Some(user) = store.get_json::<User>(&user_key)? {
        let profile = serde_json::json!({
            "id": user.id,
            "username": user.username,
            "bio": user.bio.unwrap_or_default(),
        });
        Ok(Response::builder()
            .status(200)
            .header("Content-Type", "application/json")
            .body(serde_json::to_vec(&profile)?)
            .build())
    } else {
        Ok(Response::builder().status(404).body("User not found").build())
    }
}

pub fn get_user_details(path: &str) -> anyhow::Result<Response> {
    let user_id = path.trim_start_matches("/users/");
    
    if user_id.is_empty() {
        return Ok(Response::builder().status(400).body("User ID required").build());
    }

    let store = store();
    let user_key = format!("user:{}", user_id);
    
    if let Some(user) = store.get_json::<User>(&user_key)? {
        let user_data = serde_json::json!({
            "id": user.id,
            "username": user.username,
            "bio": user.bio.unwrap_or_default(),
        });
        
        Ok(Response::builder()
            .status(200)
            .header("Content-Type", "application/json")
            .body(serde_json::to_vec(&user_data)?)
            .build())
    } else {
        Ok(Response::builder().status(404).body("User not found").build())
    }
}

pub fn get_user_profile(_req: &Request, path: &str) -> anyhow::Result<Response> {
    let username = path.trim_start_matches('/');
    let store = store();
    
    // Find user by username
    let users: Vec<String> = store.get_json("users_list")?.unwrap_or_default();
    let mut target_user: Option<User> = None;
    
    for id in users {
        if let Some(u) = store.get_json::<User>(&format!("user:{}", id))? {
            if u.username == username {
                target_user = Some(u);
                break;
            }
        }
    }
    
    if target_user.is_none() {
        return Ok(Response::builder().status(404).body("User not found").build());
    }
    
    let user = target_user.unwrap();
    
    // Load profile.html template
    let template = Assets::get("profile.html")
        .ok_or_else(|| anyhow::anyhow!("Profile template not found"))?
        .data
        .to_vec();
    
    let mut html = String::from_utf8(template)?;
    
    // Replace placeholders
    let escaped_username = html_escape::encode_text(&user.username).to_string();
    let escaped_user_id = html_escape::encode_text(&user.id).to_string();
    
    html = html.replace("PROFILE_USERNAME", &escaped_username);
    html = html.replace("PROFILE_USER_ID", &escaped_user_id);
    
    // Replace bio section
    let bio_section = user.bio.as_ref()
        .map(|bio| format!(
            r#"<div class="profile-field">
                <div class="profile-field-label">Bio</div>
                <div class="profile-field-value">{}</div>
            </div>"#,
            html_escape::encode_text(bio)
        ))
        .unwrap_or_default();
    
    html = html.replace("PROFILE_BIO", &bio_section);
    
    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "text/html; charset=utf-8")
        .body(html.into_bytes())
        .build())
}
