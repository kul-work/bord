use spin_sdk::http::{Request, Response};
use spin_sdk::key_value::Store;
use crate::models::models::User;
use crate::core::helpers::{store, validate_uuid};
use crate::core::errors::ApiError;
use crate::auth::validate_token;
use crate::config::*;

pub fn follow_user(store: &Store, follower_id: &str, following_id: &str) -> anyhow::Result<()> {
    let followings_key = followings_key(follower_id);
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
    let followings_key = followings_key(follower_id);
    let mut followings: Vec<String> = store
        .get_json(&followings_key)?
        .unwrap_or_default();
    
    followings.retain(|id| id != following_id);
    store.set_json(&followings_key, &followings)?;
    
    Ok(())
}

pub fn get_followings(store: &Store, user_id: &str) -> anyhow::Result<Vec<String>> {
    let followings_key = followings_key(user_id);
    let followings: Vec<String> = store
        .get_json(&followings_key)?
        .unwrap_or_default();
    
    Ok(followings)
}

pub fn get_followers(store: &Store, user_id: &str) -> anyhow::Result<Vec<String>> {
    let users: Vec<String> = store.get_json(USERS_LIST_KEY)?.unwrap_or_default();
    let mut followers = Vec::new();
    
    for id in users {
        let followings_key = followings_key(&id);
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
    let user_id = match validate_token(&req) {
        Some(uid) => uid,
        None => return Ok(ApiError::Unauthorized.into()),
    };

    let store = store();
    let body = req.body();
    let value: serde_json::Value = serde_json::from_slice(body)?;
    let target_user_id = value["target_user_id"].as_str().unwrap_or_default();

    if target_user_id.is_empty() || !validate_uuid(target_user_id) || target_user_id == user_id {
        return Ok(ApiError::BadRequest("Invalid target user".to_string()).into());
    }

    // Verify target user exists
    let target_key = user_key(target_user_id);
    if store.get_json::<User>(&target_key)? .is_none() {
        return Ok(ApiError::NotFound("Target user not found".to_string()).into());
    }

    follow_user(&store, &user_id, target_user_id)?;

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .body(serde_json::to_vec(&serde_json::json!({"status": "followed"}))?)
        .build())
}

pub fn handle_unfollow(req: Request) -> anyhow::Result<Response> {
    let user_id = match validate_token(&req) {
        Some(uid) => uid,
        None => return Ok(ApiError::Unauthorized.into()),
    };

    let store = store();
    let body = req.body();
    let value: serde_json::Value = serde_json::from_slice(body)?;
    let target_user_id = value["target_user_id"].as_str().unwrap_or_default();

    if target_user_id.is_empty() || !validate_uuid(target_user_id) {
        return Ok(ApiError::BadRequest("Invalid target user".to_string()).into());
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
    
    if user_id.is_empty() || !validate_uuid(user_id) {
        return Ok(ApiError::BadRequest("User ID required".to_string()).into());
    }

    let store = store();
    let followings = get_followings(&store, user_id)?;
    
    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .body(serde_json::to_vec(&followings)?)
        .build())
}

pub fn get_followers_list(path: &str) -> anyhow::Result<Response> {
    let user_id = path.trim_start_matches("/followers/");
    
    if user_id.is_empty() || !validate_uuid(user_id) {
        return Ok(ApiError::BadRequest("User ID required".to_string()).into());
    }

    let store = store();
    let followers = get_followers(&store, user_id)?;
    
    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .body(serde_json::to_vec(&followers)?)
        .build())
}
