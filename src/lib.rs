use spin_sdk::{
    http::{Request, IntoResponse},
    http_component,
};

mod core;
mod models;
mod config;
mod templates;
mod auth;
mod users;
mod posts;
mod follow;

use core::db;
use core::helpers;
use core::static_server;
use core::errors::ApiError;


pub use db::{init_test_data, reset_db_data};

// === Component entrypoint ===
#[http_component]
fn handle(req: Request) -> anyhow::Result<impl IntoResponse> {
    let _ = db::init_test_data(&helpers::store()); // Initialize test data on first request
    
    let path = req.path();
    let method = req.method();

    match (method.to_string().as_str(), path) {
        #[cfg(feature = "perf")]
        ("POST", "/dev/ok") => {
            Ok(spin_sdk::http::Response::builder().status(200).body(b"ok".to_vec()).build())
        },
        #[cfg(feature = "perf")]
        ("POST", "/dev/reset") => {
            db::reset_db_data(&helpers::store())?;
            Ok(spin_sdk::http::Response::builder().status(200).body(b"DB reseted.".to_vec()).build())
        },
        ("POST", "/users") => users::create_user(req),
        ("POST", "/login") => auth::login_user(req),
        ("POST", "/logout") => auth::logout_user(req),
        ("GET", "/profile") => users::get_profile(req),
        ("PUT", "/profile") => users::update_profile(req),        
        ("POST", "/posts") => posts::create_post(req),
        ("GET", "/posts") => posts::list_posts(req),        
        ("PUT", p) if p.starts_with("/posts/") => posts::edit_post(req),
        ("DELETE", p) if p.starts_with("/posts/") => posts::delete_post(req),
        ("GET", "/feed") => posts::get_feed(req),
        ("POST", "/follow") => follow::handle_follow(req),
        ("POST", "/unfollow") => follow::handle_unfollow(req),
        ("GET", p) if p.starts_with("/followings/") => follow::get_followings_list(p),
        ("GET", p) if p.starts_with("/followers/") => follow::get_followers_list(p),
        ("GET", p) if p.starts_with("/users/") && p.len() > 7 => users::get_user_details(p),
        ("GET", p) if !p.contains('.') && p.len() > 1 && p != "/" => templates::render_user_profile(&req, p),
        ("GET", p) => static_server::serve_static(p),
        _ => Ok(ApiError::NotFound("No route found".to_string()).into()),
    }
}
