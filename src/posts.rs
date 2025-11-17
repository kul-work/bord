use spin_sdk::http::{Request, Response};
use uuid::Uuid;
use regex::Regex;
use html_escape::encode_double_quoted_attribute;
use ammonia::Builder;
use crate::models::models::User;
use crate::models::models::Post;
use crate::core::helpers::{store, now_iso, unauthorized, validate_uuid};
use crate::auth::validate_token;

pub fn create_post(req: Request) -> anyhow::Result<Response> {
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

pub fn edit_post(req: Request) -> anyhow::Result<Response> {
    let user_id = match validate_token(&req) {
        Some(uid) => uid,
        None => return Ok(unauthorized()),
    };

    let path = req.path();
    let post_id = path.split('/').last().unwrap_or("");

    if post_id.is_empty() || !validate_uuid(post_id) {
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


pub fn delete_post(req: Request) -> anyhow::Result<Response> {
     let user_id = match validate_token(&req) {
         Some(uid) => uid,
         None => return Ok(unauthorized()),
     };
 
     let path = req.path();
     let post_id = path.split('/').last().unwrap_or("");
     
     if post_id.is_empty() || !validate_uuid(post_id) {
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

pub fn list_posts(req: Request) -> anyhow::Result<Response> {
    let store = store();
    let uri = req.uri();
    
    // Check for query parameters
    let (filter_username, show_all) = if let Some(query_start) = uri.find('?') {
        let query = &uri[query_start+1..];
        let mut username = None;
        let mut all = false;
        
        for param in query.split('&') {
            if param.starts_with("user=") {
                let encoded_username = &param[5..];
                let decoded = urlencoding::decode(encoded_username)
                    .unwrap_or(std::borrow::Cow::Borrowed(encoded_username))
                    .to_string();
                username = Some(decoded);
            } else if param == "all=true" {
                all = true;
            }
        }
        (username, all)
    } else {
        (None, false)
    };
    
    // If filtering by username or showing all, no auth required
    // Otherwise, require authentication for personal posts
    let user_id = if filter_username.is_none() && !show_all {
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
    } else if show_all {
        // Get all posts from the global feed, sorted by creation date
        for id in feed.iter() {
            if let Some(p) = store.get_json::<Post>(&format!("post:{}", id))? {
                posts.push(p);
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

pub fn get_feed(req: Request) -> anyhow::Result<Response> {
    let user_id = match validate_token(&req) {
        Some(uid) => uid,
        None => return Ok(unauthorized()),
    };

    let store = store();
    
    // Get user's following list
    let followings: Vec<String> = store.get_json(&format!("followings:{}", user_id))?
        .unwrap_or_default();
    
    // Get all posts from feed
    let feed: Vec<String> = store.get_json("feed")?.unwrap_or_default();
    
    let mut posts: Vec<Post> = Vec::new();
    
    // Collect posts from user and their followings
    for post_id in feed.iter() {
        if let Some(p) = store.get_json::<Post>(&format!("post:{}", post_id))? {
            // Include if post is from someone they follow
            if followings.contains(&p.user_id) {
                posts.push(p);
            }
        }
    }
    
    // Sort by created_at in descending order (newest first)
    posts.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    
    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .body(serde_json::to_vec(&posts)?)
        .build())
}

