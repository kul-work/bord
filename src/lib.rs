use spin_sdk::{
    http::{Request, Response, IntoResponse},
    http_component,
    key_value::Store,
};
use serde::{Serialize, Deserialize};
use uuid::Uuid;

// === Data models ===
#[derive(Serialize, Deserialize, Clone)]
struct User {
    id: String,
    username: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct Post {
    id: String,
    user_id: String,
    content: String,
    created_at: String,
}

// === Helpers ===
fn store() -> Store {
    Store::open_default().expect("KV store must exist")
}

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339()
}

// === Component entrypoint ===
#[http_component]
fn handle(req: Request) -> anyhow::Result<impl IntoResponse> {
    let path = req.path();
    let method = req.method();

    match (method.to_string().as_str(), path) {
        ("POST", "/users") => create_user(req),
        ("POST", "/posts") => create_post(req),
        ("GET", "/posts") => list_posts(),
        _ => Ok(Response::builder().status(404).body("Not found").build()),
    }
}

// === Handlers ===

fn create_user(req: Request) -> anyhow::Result<Response> {
    let store = store();
    let body = req.body();
    //eprintln!("Body: {:?}", std::str::from_utf8(body));
    let new_user: serde_json::Value = serde_json::from_slice(body)?;
    let username = new_user["username"].as_str().unwrap_or("anon");
    let id = Uuid::new_v4().to_string();

    let user = User {
        id: id.clone(),
        username: username.to_string(),
    };

    let key = format!("user:{}", id);
    store.set_json(&key, &user)?;

    Ok(Response::builder()
        .status(201)
        .header("Content-Type", "application/json")
        .body(serde_json::to_vec(&user)?)
        .build())
}

fn create_post(req: Request) -> anyhow::Result<Response> {
    let store = store();
    let body = req.body();
    let value: serde_json::Value = serde_json::from_slice(body)?;
    let user_id = value["user_id"].as_str().unwrap_or_default();
    let content = value["content"].as_str().unwrap_or_default();
    let id = Uuid::new_v4().to_string();

    let post = Post {
        id: id.clone(),
        user_id: user_id.to_string(),
        content: content.to_string(),
        created_at: now_iso(),
    };

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

fn list_posts() -> anyhow::Result<Response> {
    let store = store();
    let feed: Vec<String> = store.get_json("feed")?.unwrap_or_default();

    let mut posts = Vec::new();
    for id in feed.iter().take(20) {
        if let Some(p) = store.get_json::<Post>(&format!("post:{}", id))? {
            posts.push(p);
        }
    }

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .body(serde_json::to_vec(&posts)?)
        .build())
}
