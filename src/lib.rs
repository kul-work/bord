use spin_sdk::{
    http::{Request, IntoResponse},
    http_component,
};

mod db;
mod models;
mod config;
mod static_server;
mod templates;
mod helpers;
mod auth;
mod users;
mod posts;
mod handlers;
mod follow;


pub use db::init_test_data;

// === Component entrypoint ===
#[http_component]
fn handle(req: Request) -> anyhow::Result<impl IntoResponse> {
    let _ = db::init_test_data(&helpers::store()); // Initialize test data on first request
    
    let path = req.path();
    let method = req.method();

    match (method.to_string().as_str(), path) {
        ("POST", "/users") => users::create_user(req),
        ("POST", "/login") => auth::login_user(req),
        ("GET", "/profile") => users::get_profile(req),
        ("PUT", "/profile") => users::update_profile(req),
        ("POST", "/posts") => posts::create_post(req),
        ("GET", "/posts") => posts::list_posts(req),
        ("GET", "/feed") => posts::get_feed(req),
        ("PUT", p) if p.starts_with("/posts/") => posts::edit_post(req),
        ("DELETE", p) if p.starts_with("/posts/") => posts::delete_post(req),
        ("POST", "/follow") => handlers::handle_follow(req),
        ("POST", "/unfollow") => handlers::handle_unfollow(req),
        ("GET", p) if p.starts_with("/followings/") => handlers::get_followings_list(p),
        ("GET", p) if p.starts_with("/followers/") => handlers::get_followers_list(p),
        ("GET", p) if p.starts_with("/users/") && p.len() > 7 => users::get_user_details(p),
        ("GET", p) if !p.contains('.') && p.len() > 1 && p != "/" => templates::render_user_profile(&req, p),
        ("GET", p) => static_server::serve_static(p),
        _ => Ok(spin_sdk::http::Response::builder().status(404).body("Not found").build()),
    }
}
