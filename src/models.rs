use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct User {
    pub id: String,
    pub username: String,
    pub password: String,
    pub bio: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Post {
    pub id: String,
    pub user_id: String,
    pub content: String,
    pub created_at: String,
    pub updated_at: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct TokenData {
    pub user_id: String,
    pub created_at: String,
}

#[allow(dead_code)]
pub type Followings = Vec<String>;
#[allow(dead_code)]
pub type Followers = Vec<String>;
