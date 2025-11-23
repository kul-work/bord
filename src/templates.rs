use spin_sdk::http::{Request, Response};
use rust_embed::RustEmbed;
use crate::models::models::User;
use crate::core::helpers::store;
use crate::core::errors::ApiError;
use crate::config::*;

#[derive(RustEmbed)]
#[folder = "static"]
struct Assets;

pub fn render_user_profile(_req: &Request, path: &str) -> anyhow::Result<Response> {
    let username = path.trim_start_matches('/');
    let store = store();
    
    // Find user by username
    let users: Vec<String> = store.get_json(USERS_LIST_KEY)?.unwrap_or_default();
    let mut target_user: Option<User> = None;
    
    for id in users {
        if let Some(u) = store.get_json::<User>(&user_key(&id))? {
            if u.username == username {
                target_user = Some(u);
                break;
            }
        }
    }
    
    if target_user.is_none() {
        return Ok(ApiError::NotFound("User not found".to_string()).into());
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
