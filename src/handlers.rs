use spin_sdk::http::{Request, Response};
use crate::helpers::{store, unauthorized};
use crate::auth::validate_token;
use crate::follow::{follow_user, unfollow_user, get_followings, get_followers};

pub fn handle_follow(req: Request) -> anyhow::Result<Response> {
    let user_id = match validate_token(&req) {
        Some(uid) => uid,
        None => return Ok(unauthorized()),
    };

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
    let user_id = match validate_token(&req) {
        Some(uid) => uid,
        None => return Ok(unauthorized()),
    };

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
