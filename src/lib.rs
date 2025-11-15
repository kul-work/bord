use spin_sdk::{
    http::{Request, Response, IntoResponse},
    http_component,
    key_value::Store,
};
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use sha2::{Sha256, Digest};
use mime_guess::from_path;
use rust_embed::RustEmbed;
use regex::Regex;
use html_escape::encode_double_quoted_attribute;
use ammonia::Builder;

#[derive(RustEmbed)]
#[folder = "static"]
struct Assets;

// === Config ===
fn token_expiration_hours() -> i64 {
    std::env::var("BORD_TOKEN_EXPIRATION_HOURS")
        .ok()
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(24)
}

// === Data models ===
#[derive(Serialize, Deserialize, Clone)]
struct User {
    id: String,
    username: String,
    password: String,
    bio: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
struct Post {
    id: String,
    user_id: String,
    content: String,
    created_at: String,
    updated_at: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct TokenData {
    user_id: String,
    created_at: String,
}

// === Helpers ===
fn store() -> Store {
    Store::open_default().expect("KV store must exist")
}

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn unauthorized() -> Response {
    Response::builder().status(401).body("Unauthorized").build()
}

fn hash_password(password: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    format!("{:x}", hasher.finalize())
}

// === Static file serving ===
fn serve_static(path: &str) -> anyhow::Result<Response> {
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

// === Database initialization ===
fn init_test_data() -> anyhow::Result<()> {
    let store = store();
    
    // Check if test user already exists
    let users: Vec<String> = store.get_json("users_list")?.unwrap_or_default();
    for id in &users {
        if let Some(u) = store.get_json::<User>(&format!("user:{}", id))? {
            if u.username == "test" {
                return Ok(()); // Already initialized
            }
        }
    }
    
    // Create test user
    let user_id = Uuid::new_v4().to_string();
    let user = User {
        id: user_id.clone(),
        username: "test".to_string(),
        password: hash_password("test"),
        bio: None,
    };
    
    store.set_json(&format!("user:{}", user_id), &user)?;
    
    let mut users = users;
    users.push(user_id.clone());
    store.set_json("users_list", &users)?;
    
    // Create test post
    let post_id = Uuid::new_v4().to_string();
    let post = Post {
        id: post_id.clone(),
        user_id,
        content: "text text text".to_string(),
        created_at: now_iso(),
        updated_at: None,
    };
    
    store.set_json(&format!("post:{}", post_id), &post)?;
    
    let mut feed: Vec<String> = store.get_json("feed")?.unwrap_or_default();
    feed.insert(0, post_id);
    store.set_json("feed", &feed)?;
    
    Ok(())
}

// === Component entrypoint ===
#[http_component]
fn handle(req: Request) -> anyhow::Result<impl IntoResponse> {
    let _ = init_test_data(); // Initialize test data on first request
    
    let path = req.path();
    let method = req.method();

    match (method.to_string().as_str(), path) {
        ("POST", "/users") => create_user(req),
        ("POST", "/login") => login_user(req),
        ("GET", "/profile") => get_profile(req),
        ("PUT", "/profile") => update_profile(req),
        ("POST", "/posts") => create_post(req),
        ("GET", "/posts") => list_posts(req),
        ("PUT", p) if p.starts_with("/posts/") => edit_post(req),
        ("DELETE", p) if p.starts_with("/posts/") => delete_post(req),
        ("GET", p) if !p.contains('.') && p.len() > 1 && p != "/" => get_user_profile(&req, p),
        ("GET", p) => serve_static(p),
        _ => Ok(Response::builder().status(404).body("Not found").build()),
    }
}

// === Handlers ===

fn create_user(req: Request) -> anyhow::Result<Response> {
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

fn login_user(req: Request) -> anyhow::Result<Response> {
    let store = store();
    let creds: serde_json::Value = serde_json::from_slice(req.body())?;
    let username = creds["username"].as_str().unwrap_or_default();
    let password = creds["password"].as_str().unwrap_or_default();

    let users: Vec<String> = store.get_json("users_list")?.unwrap_or_default();

    for id in users {
        if let Some(u) = store.get_json::<User>(&format!("user:{}", id))? {
            if u.username == username && u.password == hash_password(password) {
                let token = Uuid::new_v4().to_string();
                let data = TokenData {
                    user_id: u.id.clone(),
                    created_at: now_iso(),
                };
                store.set_json(&format!("token:{}", token), &data)?;

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

fn get_profile(req: Request) -> anyhow::Result<Response> {
    let user_id = match validate_token(&req) {
        Some(uid) => uid,
        None => return Ok(unauthorized()),
    };

    let store = store();
    let user_key = format!("user:{}", user_id);

    if let Some(user) = store.get_json::<User>(&user_key)? {
        // Return user without password
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

fn update_profile(req: Request) -> anyhow::Result<Response> {
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

        // Return user without password
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

// === Auth helper ===
fn validate_token(req: &Request) -> Option<String> {
    let store = store();
    let auth_header = req.header("Authorization")?.as_str().unwrap_or_default();
    if !auth_header.starts_with("Bearer ") {
        return None;
    }
    let token = &auth_header[7..];
    let key = format!("token:{}", token);
    if let Some(data) = store.get_json::<TokenData>(&key).ok()? {
        // Check if token is expired
        if let Ok(created) = chrono::DateTime::parse_from_rfc3339(&data.created_at) {
            let now = chrono::Utc::now();
            let age_hours = (now - created.with_timezone(&chrono::Utc)).num_hours();
            if age_hours > token_expiration_hours() {
                return None;
            }
        }
        Some(data.user_id)
    } else {
        None
    }
}

// === Post content filters ===
// Applies transformations to post content before storage.
// Add new filters here as needed (e.g., markdown, sanitization, etc.)
fn filter_post_content(content: &str) -> String {
    // Sanitize HTML to remove dangerous scripts and event handlers
    let clean = Builder::default()
        .link_rel(Some("noopener noreferrer"))
        .clean(content)
        .to_string();
    
    // Convert HTTP/HTTPS URLs into clickable links with proper escaping
    let url_pattern = Regex::new(r"https?://[^\s]+").unwrap();
    url_pattern.replace_all(&clean, |caps: &regex::Captures| {
        let url = &caps[0];
        let escaped_url = encode_double_quoted_attribute(url);
        format!(r#"<a href="{}" target="_blank">{}</a>"#, escaped_url, url)
    }).to_string()
}

fn create_post(req: Request) -> anyhow::Result<Response> {
    let user_id = match validate_token(&req) {
        Some(uid) => uid,
        None => return Ok(unauthorized()),
    };

    let store = store();
    let body = req.body();

    let value: serde_json::Value = serde_json::from_slice(body)?;
    let content = value["content"].as_str().unwrap_or_default();
    let id = Uuid::new_v4().to_string();

    let post = Post {
        id: id.clone(),
        user_id: user_id.to_string(),
        content: filter_post_content(content),
        created_at: now_iso(),
        updated_at: None,
    };

    // Add validation
    if content.is_empty() || content.len() > 5000 {
        return Ok(Response::builder().status(400).body("Invalid content").build());
    }

    // Save post object
    store.set_json(&format!("post:{}", id), &post)?;

    // Append to global feed (store IDs in a JSON list)
    let mut feed: Vec<String> = store.get_json("feed")?.unwrap_or_default();
    feed.insert(0, id.clone()); // prepend newest
    store.set_json("feed", &feed)?;

    Ok(Response::builder()
        .status(201)
        .header("Content-Type", "application/json")
        .body(serde_json::to_vec(&post)?)
        .build())
}

fn list_posts(req: Request) -> anyhow::Result<Response> {
    let store = store();
    let uri = req.uri();
    
    // Check if this is a public query (filtering by username)
    let filter_username = if let Some(query_start) = uri.find('?') {
        let query = &uri[query_start+1..];
        if query.starts_with("user=") {
            let encoded_username = &query[5..];
            // URL decode the username
            let decoded = urlencoding::decode(encoded_username)
                .unwrap_or(std::borrow::Cow::Borrowed(encoded_username))
                .to_string();
            Some(decoded)
        } else {
            None
        }
    } else {
        None
    };
    
    // If no username filter, require authentication
    let user_id = if filter_username.is_none() {
        match validate_token(&req) {
            Some(uid) => uid,
            None => return Ok(unauthorized()),
        }
    } else {
        String::new() // Not used for filtered queries
    };

    let feed: Vec<String> = store.get_json("feed")?.unwrap_or_default();

    let mut posts = Vec::new();
    
    if let Some(username) = filter_username {
        // Public query: get posts for specific username
        let users: Vec<String> = store.get_json("users_list")?.unwrap_or_default();
        let mut target_user_id: Option<String> = None;
        
        for id in users {
            if let Some(u) = store.get_json::<User>(&format!("user:{}", id))? {
                if u.username == username {
                    target_user_id = Some(u.id);
                    break;
                }
            }
        }
        
        if let Some(uid) = target_user_id {
            for id in feed.iter().take(20) {
                if let Some(p) = store.get_json::<Post>(&format!("post:{}", id))? {
                    if p.user_id == uid {
                        posts.push(p);
                    }
                }
            }
        }
    } else {
        // Authenticated query: get posts for current user
        for id in feed.iter().take(20) {
            if let Some(p) = store.get_json::<Post>(&format!("post:{}", id))? {
                if p.user_id == user_id {
                    posts.push(p);
                }
            }
        }
    }

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .body(serde_json::to_vec(&posts)?)
        .build())
}

fn edit_post(req: Request) -> anyhow::Result<Response> {
    let user_id = match validate_token(&req) {
        Some(uid) => uid,
        None => return Ok(unauthorized()),
    };

    let path = req.path();
    let post_id = path.split('/').last().unwrap_or("");

    if post_id.is_empty() {
        return Ok(Response::builder().status(400).body("Post ID required").build());
    }

    let store = store();
    let post_key = format!("post:{}", post_id);

    // Check if post exists and belongs to user
    if let Some(mut post) = store.get_json::<Post>(&post_key)? {
        if post.user_id != user_id {
            return Ok(Response::builder().status(403).body("Forbidden").build());
        }

        let value: serde_json::Value = serde_json::from_slice(req.body())?;
        let content = value["content"].as_str().unwrap_or_default();

        // Validate content
        if content.is_empty() || content.len() > 5000 {
            return Ok(Response::builder().status(400).body("Invalid content").build());
        }

        // Skip update if content didn't change
        let filtered_content = filter_post_content(content);
        if post.content == filtered_content {
            return Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(serde_json::to_vec(&post)?)
                .build());
        }

        // Update post
        post.content = filtered_content;
        post.updated_at = Some(now_iso());

        store.set_json(&post_key, &post)?;

        Ok(Response::builder()
            .status(200)
            .header("Content-Type", "application/json")
            .body(serde_json::to_vec(&post)?)
            .build())
    } else {
        Ok(Response::builder().status(404).body("Post not found").build())
    }
}

fn delete_post(req: Request) -> anyhow::Result<Response> {
     let user_id = match validate_token(&req) {
         Some(uid) => uid,
         None => return Ok(unauthorized()),
     };
 
     let path = req.path();
     let post_id = path.split('/').last().unwrap_or("");
 
     if post_id.is_empty() {
         return Ok(Response::builder().status(400).body("Post ID required").build());
     }
 
     let store = store();
     let post_key = format!("post:{}", post_id);
 
     // Check if post exists and belongs to user
     if let Some(p) = store.get_json::<Post>(&post_key)? {
         if p.user_id != user_id {
             return Ok(Response::builder().status(403).body("Forbidden").build());
         }
 
         // Delete the post
         store.delete(&post_key)?;
 
         // Remove from feed
         let mut feed: Vec<String> = store.get_json("feed")?.unwrap_or_default();
         feed.retain(|id| id != post_id);
         store.set_json("feed", &feed)?;
 
         Ok(Response::builder().status(204).body("").build())
     } else {
         Ok(Response::builder().status(404).body("Post not found").build())
     }
}

fn get_user_profile(_req: &Request, path: &str) -> anyhow::Result<Response> {
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
