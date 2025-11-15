use spin_sdk::{
    http::{Request, Response, IntoResponse},
    http_component,
    key_value::Store,
};
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use sha2::{Sha256, Digest};

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
fn serve_static(_path: &str) -> anyhow::Result<Response> {
    let html = include_bytes!("../static/index.html");
    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "text/html; charset=utf-8")
        .body(html.to_vec())
        .build())
}

fn serve_favicon() -> anyhow::Result<Response> {
    let favicon = include_bytes!("../static/favicon.ico");
    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "image/x-icon")
        .body(favicon.to_vec())
        .build())
}

fn serve_logo() -> anyhow::Result<Response> {
    let favicon = include_bytes!("../static/b.png");
    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "image/x-icon")
        .body(favicon.to_vec())
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
        ("POST", "/posts") => create_post(req),
        ("GET", "/posts") => list_posts(req),
        ("PUT", p) if p.starts_with("/posts/") => edit_post(req),
        ("DELETE", p) if p.starts_with("/posts/") => delete_post(req),
        ("GET", "/") | ("GET", "/index.html") => serve_static(path),
        ("GET", "/favicon.ico") => serve_favicon(),
        ("GET", "/B.png") => serve_logo(),
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
        content: content.to_string(),
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
    let user_id = match validate_token(&req) {
        Some(uid) => uid,
        None => return Ok(unauthorized()),
    };

    let store = store();
    let feed: Vec<String> = store.get_json("feed")?.unwrap_or_default();

    let mut posts = Vec::new();
    for id in feed.iter().take(20) {
        if let Some(p) = store.get_json::<Post>(&format!("post:{}", id))? {
            if p.user_id == user_id {
                posts.push(p);
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

        // Update post
        post.content = content.to_string();
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
