pub fn token_expiration_hours() -> i64 {
    std::env::var("BORD_TOKEN_EXPIRATION_HOURS")
        .ok()
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(24)
}

// Content length limits
pub const MAX_POST_LENGTH: usize = 5000;
pub const MAX_BIO_LENGTH: usize = 500;

// Username constraints
pub const MIN_USERNAME_LENGTH: usize = 3;
pub const MAX_USERNAME_LENGTH: usize = 50;

// Password constraints
pub const MIN_PASSWORD_LENGTH: usize = 3;

// Pagination limits
// Must match POSTS_PER_PAGE in static/index.html
pub const POSTS_PER_PAGE: usize = 10;

