use spin_sdk::http::{Request, Response};
use spin_sdk::key_value::Store;
use crate::core::helpers::{store, require_auth};

pub fn follow_user(store: &Store, follower_id: &str, following_id: &str) -> anyhow::Result<()> {
    let followings_key = format!("followings:{}", follower_id);
    let mut followings: Vec<String> = store
        .get_json(&followings_key)?
        .unwrap_or_default();
    
    if !followings.contains(&following_id.to_string()) {
        followings.push(following_id.to_string());
        store.set_json(&followings_key, &followings)?;
    }
    
    Ok(())
}

pub fn unfollow_user(store: &Store, follower_id: &str, following_id: &str) -> anyhow::Result<()> {
    let followings_key = format!("followings:{}", follower_id);
    let mut followings: Vec<String> = store
        .get_json(&followings_key)?
        .unwrap_or_default();
    
    followings.retain(|id| id != following_id);
    store.set_json(&followings_key, &followings)?;
    
    Ok(())
}

pub fn get_followings(store: &Store, user_id: &str) -> anyhow::Result<Vec<String>> {
    let followings_key = format!("followings:{}", user_id);
    let followings: Vec<String> = store
        .get_json(&followings_key)?
        .unwrap_or_default();
    
    Ok(followings)
}

pub fn get_followers(store: &Store, user_id: &str) -> anyhow::Result<Vec<String>> {
    let users: Vec<String> = store.get_json("users_list")?.unwrap_or_default();
    let mut followers = Vec::new();
    
    for id in users {
        let followings_key = format!("followings:{}", id);
        if let Ok(Some(followings)) = store.get_json::<Vec<String>>(&followings_key) {
            if followings.contains(&user_id.to_string()) {
                followers.push(id);
            }
        }
    }
    
    Ok(followers)
}

// === HTTP Handlers ===

pub fn handle_follow(req: Request) -> anyhow::Result<Response> {
    let user_id = require_auth(&req)?;

    let store = store();
    let body = req.body();
    let value: serde_json::Value = serde_json::from_slice(body)?;
    let target_user_id = value["target_user_id"].as_str().unwrap_or_default();

    if target_user_id.is_empty() || target_user_id == user_id {
        return Ok(Response::builder().status(400).body("Invalid target user").build());
    }

    follow_user(&store, &user_id, target_user_id)?;

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .body(serde_json::to_vec(&serde_json::json!({"status": "followed"}))?)
        .build())
}

pub fn handle_unfollow(req: Request) -> anyhow::Result<Response> {
    let user_id = require_auth(&req)?;

    let store = store();
    let body = req.body();
    let value: serde_json::Value = serde_json::from_slice(body)?;
    let target_user_id = value["target_user_id"].as_str().unwrap_or_default();

    if target_user_id.is_empty() {
        return Ok(Response::builder().status(400).body("Invalid target user").build());
    }

    unfollow_user(&store, &user_id, target_user_id)?;

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .body(serde_json::to_vec(&serde_json::json!({"status": "unfollowed"}))?)
        .build())
}

pub fn get_followings_list(path: &str) -> anyhow::Result<Response> {
    let user_id = path.trim_start_matches("/followings/");
    
    if user_id.is_empty() {
        return Ok(Response::builder().status(400).body("User ID required").build());
    }

    let store = store();
    match get_followings(&store, user_id) {
        Ok(followings) => {
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(serde_json::to_vec(&followings)?)
                .build())
        }
        Err(_) => {
            Ok(Response::builder()
                .status(500)
                .body("Error retrieving followings")
                .build())
        }
    }
}

pub fn get_followers_list(path: &str) -> anyhow::Result<Response> {
    let user_id = path.trim_start_matches("/followers/");
    
    if user_id.is_empty() {
        return Ok(Response::builder().status(400).body("User ID required").build());
    }

    let store = store();
    match get_followers(&store, user_id) {
        Ok(followers) => {
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(serde_json::to_vec(&followers)?)
                .build())
        }
        Err(_) => {
            Ok(Response::builder()
                .status(500)
                .body("Error retrieving followers")
                .build())
        }
    }
}
