use spin_sdk::key_value::Store;
use crate::models::models::{User, Post};
use crate::core::helpers::{hash_password, now_iso as helpers_now_iso};
use uuid::Uuid;

fn now_iso() -> String {
    helpers_now_iso()
}

pub fn init_test_data(store: &Store) -> anyhow::Result<()> {
    // Check if test users already exist
     let users: Vec<String> = store.get_json("users_list")?.unwrap_or_default();
     let mut has_test = false;
     let mut has_alice = false;
     let mut has_bob = false;
     
     for id in &users {
         if let Some(u) = store.get_json::<User>(&format!("user:{}", id))? {
             if u.username == "test" {
                 has_test = true;
             }
             if u.username == "alice" {
                 has_alice = true;
             }
             if u.username == "bob" {
                 has_bob = true;
             }
         }
     }
     
     if has_test && has_alice && has_bob {
         return Ok(()); // Already initialized
     }
    
    let mut users = users;
    let mut feed: Vec<String> = store.get_json("feed")?.unwrap_or_default();
    
    // Create first test user if not exists
    if !has_test {
        let user_id = Uuid::new_v4().to_string();
        let user = User {
            id: user_id.clone(),
            username: "test".to_string(),
            password: hash_password("test")?,
            bio: Some("Test user bio".to_string()),
        };
        
        store.set_json(&format!("user:{}", user_id), &user)?;
        users.push(user_id.clone());
        
        // Create test post
        let post_id = Uuid::new_v4().to_string();
        let post = Post {
            id: post_id.clone(),
            user_id,
            content: "This is my first post on Bord!".to_string(),
            created_at: now_iso(),
            updated_at: None,
        };
        
        store.set_json(&format!("post:{}", post_id), &post)?;
        feed.insert(0, post_id);
    }
    
    // Create second test user if not exists
    if !has_alice {
        let user_id = Uuid::new_v4().to_string();
        let user = User {
            id: user_id.clone(),
            username: "alice".to_string(),
            password: hash_password("alice")?,
            bio: Some("Hello, I'm Alice!".to_string()),
        };
        
        store.set_json(&format!("user:{}", user_id), &user)?;
        users.push(user_id.clone());
        
        // Create first post for alice
        let post_id_1 = Uuid::new_v4().to_string();
        let post_1 = Post {
            id: post_id_1.clone(),
            user_id: user_id.clone(),
            content: "Welcome to my board! Excited to share thoughts here.".to_string(),
            created_at: now_iso(),
            updated_at: None,
        };
        
        store.set_json(&format!("post:{}", post_id_1), &post_1)?;
        feed.insert(0, post_id_1);
        
        // Create second post for alice
        let post_id_2 = Uuid::new_v4().to_string();
        let post_2 = Post {
            id: post_id_2.clone(),
            user_id: user_id.clone(),
            content: "Just finished an amazing project. Feeling productive today!".to_string(),
            created_at: now_iso(),
            updated_at: None,
        };
        
        store.set_json(&format!("post:{}", post_id_2), &post_2)?;
        feed.insert(0, post_id_2);
    }
    
    // Create third test user if not exists
    if !has_bob {
        let user_id = Uuid::new_v4().to_string();
        let user = User {
            id: user_id.clone(),
            username: "bob".to_string(),
            password: hash_password("bob")?,
            bio: Some("Bob's corner of the internet".to_string()),
        };
        
        store.set_json(&format!("user:{}", user_id), &user)?;
        users.push(user_id.clone());
        
        // Create post for bob
        let post_id = Uuid::new_v4().to_string();
        let post = Post {
            id: post_id.clone(),
            user_id,
            content: "Hey everyone! Just joined Bord, looking forward to connecting with you all.".to_string(),
            created_at: now_iso(),
            updated_at: None,
        };
        
        store.set_json(&format!("post:{}", post_id), &post)?;
        feed.insert(0, post_id);
    }
    
    store.set_json("users_list", &users)?;
    store.set_json("feed", &feed)?;
    
    Ok(())
    }
