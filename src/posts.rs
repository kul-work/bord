use spin_sdk::http::{Request, Response};
use uuid::Uuid;
use regex::Regex;
use html_escape::encode_double_quoted_attribute;
use ammonia::Builder;
use std::sync::OnceLock;
use crate::models::models::User;
use crate::models::models::Post;
use crate::core::helpers::{store, now_iso, validate_uuid};
use crate::core::query_params::{parse_query_params, get_string, get_bool_flag, get_int};
use crate::core::errors::ApiError;
use crate::auth::validate_token;
use crate::config::*;

pub fn create_post(req: Request) -> anyhow::Result<Response> {
    let user_id = match validate_token(&req) {
        Some(uid) => uid,
        None => return Ok(ApiError::Unauthorized.into()),
    };

    let store = store();
    let body = req.body();

    let value: serde_json::Value = serde_json::from_slice(body)?;
    let content = value["content"].as_str().unwrap_or_default();
    let id = Uuid::new_v4().to_string();

    // Add validation
    if content.is_empty() || content.len() > MAX_POST_LENGTH {
        return Ok(ApiError::BadRequest("Invalid content".to_string()).into());
    }

    let post = Post {
        id: id.clone(),
        user_id: user_id.to_string(),
        content: filter_post_content(content),
        created_at: now_iso(),
        updated_at: None,
    };

    // Save post object
    store.set_json(&post_key(&id), &post)?;

    // Append to global feed (store IDs in a JSON list)
    let mut feed: Vec<String> = store.get_json(FEED_KEY)?.unwrap_or_default();
    feed.insert(0, id.clone()); // prepend newest
    store.set_json(FEED_KEY, &feed)?;

    Ok(Response::builder()
        .status(201)
        .header("Content-Type", "application/json")
        .body(serde_json::to_vec(&post)?)
        .build())
}

pub fn edit_post(req: Request) -> anyhow::Result<Response> {
    let user_id = match validate_token(&req) {
        Some(uid) => uid,
        None => return Ok(ApiError::Unauthorized.into()),
    };

    let path = req.path();
    let post_id = path.split('/').last().unwrap_or("");

    if post_id.is_empty() || !validate_uuid(post_id) {
        return Ok(ApiError::BadRequest("Post ID required".to_string()).into());
    }

    let store = store();
    let post_key = post_key(post_id);

    // Check if post exists and belongs to user
    if let Some(mut post) = store.get_json::<Post>(&post_key)? {
        if post.user_id != user_id {
            return Ok(ApiError::Forbidden.into());
        }

        let value: serde_json::Value = serde_json::from_slice(req.body())?;
        let content = value["content"].as_str().unwrap_or_default();

        // Validate content
        if content.is_empty() || content.len() > MAX_POST_LENGTH {
            return Ok(ApiError::BadRequest("Invalid content".to_string()).into());
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
        Ok(ApiError::NotFound("Post not found".to_string()).into())
    }
}

fn url_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"https?://[^\s]+").expect("Regex should compile")
    })
}

fn filter_post_content(content: &str) -> String {
    // Sanitize HTML to remove dangerous scripts and event handlers
    let clean = Builder::default()
        .link_rel(Some("noopener noreferrer"))
        .clean(content)
        .to_string();
    
    // Convert HTTP/HTTPS URLs into clickable links with proper escaping
    url_regex().replace_all(&clean, |caps: &regex::Captures| {
        let url = &caps[0];
        let escaped_url = encode_double_quoted_attribute(url);
        format!(r#"<a href="{}" target="_blank">{}</a>"#, escaped_url, url)
    }).to_string()
}

/// Fetch all posts from the global feed
fn get_all_posts_from_feed() -> anyhow::Result<Vec<Post>> {
    let store = store();
    let feed: Vec<String> = store.get_json(FEED_KEY)?.unwrap_or_default();
    let mut posts = Vec::new();
    
    for id in feed.iter() {
        if let Some(p) = store.get_json::<Post>(&post_key(id))? {
            posts.push(p);
        }
    }
    
    Ok(posts)
}

/// Filter posts by a single user_id
fn filter_posts_by_user(user_id: &str) -> anyhow::Result<Vec<Post>> {
    let store = store();
    let feed: Vec<String> = store.get_json(FEED_KEY)?.unwrap_or_default();
    let mut posts = Vec::new();
    
    for id in feed.iter() {
        if let Some(p) = store.get_json::<Post>(&post_key(id))? {
            if p.user_id == user_id {
                posts.push(p);
            }
        }
    }
    
    Ok(posts)
}

/// Filter posts from multiple user_ids (e.g., followings)
fn filter_posts_by_users(user_ids: &[String]) -> anyhow::Result<Vec<Post>> {
    let store = store();
    let feed: Vec<String> = store.get_json(FEED_KEY)?.unwrap_or_default();
    let mut posts = Vec::new();
    
    for id in feed.iter() {
        if let Some(p) = store.get_json::<Post>(&post_key(id))? {
            if user_ids.contains(&p.user_id) {
                posts.push(p);
            }
        }
    }
    
    Ok(posts)
}

/// Look up a user by username
fn get_user_by_username(username: &str) -> anyhow::Result<Option<String>> {
    let store = store();
    let users: Vec<String> = store.get_json(USERS_LIST_KEY)?.unwrap_or_default();
    
    for id in users {
        if let Some(u) = store.get_json::<User>(&user_key(&id))? {
            if u.username == username {
                return Ok(Some(u.id));
            }
        }
    }
    
    Ok(None)
}

/// Apply pagination to a list of posts
fn paginate_posts(posts: Vec<Post>, page: usize) -> Vec<Post> {
    let start_idx = (page - 1) * POSTS_PER_PAGE;
    posts.into_iter()
        .skip(start_idx)
        .take(POSTS_PER_PAGE)
        .collect()
}


pub fn delete_post(req: Request) -> anyhow::Result<Response> {
     let user_id = match validate_token(&req) {
         Some(uid) => uid,
         None => return Ok(ApiError::Unauthorized.into()),
     };
 
     let path = req.path();
     let post_id = path.split('/').last().unwrap_or("");
     
     if post_id.is_empty() || !validate_uuid(post_id) {
         return Ok(ApiError::BadRequest("Post ID required".to_string()).into());
     }
 
     let store = store();
     let post_key = post_key(post_id);
     
     // Check if post exists and belongs to user
     if let Some(p) = store.get_json::<Post>(&post_key)? {
         if p.user_id != user_id {
             return Ok(ApiError::Forbidden.into());
         }
     
         // Delete the post
             store.delete(&post_key)?;
         
             // Remove from feed
             let mut feed: Vec<String> = store.get_json(FEED_KEY)?.unwrap_or_default();
             feed.retain(|id| id != post_id);
             store.set_json(FEED_KEY, &feed)?;
         
             Ok(Response::builder().status(204).build())
     } else {
         Ok(ApiError::NotFound("Post not found".to_string()).into())
     }
}

pub fn list_posts(req: Request) -> anyhow::Result<Response> {
    let uri = req.uri();
    
    // Parse query parameters
    let params = parse_query_params(uri);
    let filter_username = get_string(&params, "user", None);
    let show_all = get_bool_flag(&params, "all");
    let page = get_int(&params, "page", 1);
    
    // If filtering by username or showing all, no auth required
    // Otherwise, require authentication for personal posts
    let user_id = if filter_username.is_none() && !show_all {
        match validate_token(&req) {
            Some(uid) => uid,
            None => return Ok(ApiError::Unauthorized.into()),
        }
    } else {
        String::new() // Not used for filtered queries
    };

    let posts = if let Some(username) = filter_username {
        // Public query: get posts for specific username
        if let Some(uid) = get_user_by_username(&username)? {
            let user_posts = filter_posts_by_user(&uid)?;
            paginate_posts(user_posts, page)
        } else {
            Vec::new()
        }
    } else if show_all {
        // Get paginated posts from the global feed
        let all_posts = get_all_posts_from_feed()?;
        paginate_posts(all_posts, page)
    } else {
        // Authenticated query: get paginated posts for current user
        let user_posts = filter_posts_by_user(&user_id)?;
        paginate_posts(user_posts, page)
    };

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .body(serde_json::to_vec(&posts)?)
        .build())
}

pub fn get_feed(req: Request) -> anyhow::Result<Response> {
    let user_id = match validate_token(&req) {
        Some(uid) => uid,
        None => return Ok(ApiError::Unauthorized.into()),
    };

    let store = store();
    let uri = req.uri();
    
    // Parse page parameter from query string
    let params = parse_query_params(uri);
    let page = get_int(&params, "page", 1);
    
    // Get user's following list
    let followings: Vec<String> = store.get_json(&followings_key(&user_id))?
        .unwrap_or_default();
    
    // Get posts from users they follow
    let mut posts = filter_posts_by_users(&followings)?;
    
    // Sort by created_at in descending order (newest first)
    posts.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    
    // Apply pagination
    let paginated_posts = paginate_posts(posts, page);
    
    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .body(serde_json::to_vec(&paginated_posts)?)
        .build())
}

