use serde_json::json;
use std::sync::Mutex;

const BASE_URL: &str = "http://127.0.0.1:3000";
static TEST_LOCK: Mutex<()> = Mutex::new(());

fn lock_test() -> std::sync::MutexGuard<'static, ()> {
    TEST_LOCK.lock().unwrap()
}

#[tokio::test]
async fn test_full_user_flow() {
    let _lock = lock_test();
    let client = reqwest::Client::new();
    
    // 1. Create user
    let username = format!("flow_{}", &uuid::Uuid::new_v4().to_string()[0..8]);
    let create_body = json!({
        "username": username,
        "password": "test"
    });

    let user_resp = client
        .post(&format!("{}/users", BASE_URL))
        .json(&create_body)
        .send()
        .await
        .expect("Failed to create user");

    assert_eq!(user_resp.status(), 201);
    let user = user_resp.json::<serde_json::Value>().await.unwrap();
    assert!(user.get("id").is_some(), "User ID missing in create response: {:?}", user);
    let user_id = user["id"].as_str().unwrap().to_string();

    // 2. Login
    let login_body = json!({
        "username": &username,
        "password": "test"
    });

    let login_resp = client
        .post(&format!("{}/login", BASE_URL))
        .json(&login_body)
        .send()
        .await
        .expect("Failed to login");

    assert_eq!(login_resp.status(), 200);
    let token_data = login_resp.json::<serde_json::Value>().await.unwrap();
    assert!(token_data.get("token").is_some(), "Token field missing in response: {:?}", token_data);
    let token = token_data["token"].as_str().unwrap().to_string();

    // 3. Create post
    let post_body = json!({
        "content": "Test post from integration test!"
    });

    let post_resp = client
        .post(&format!("{}/posts", BASE_URL))
        .header("Authorization", format!("Bearer {}", token))
        .json(&post_body)
        .send()
        .await
        .expect("Failed to create post");

    assert_eq!(post_resp.status(), 201);
    let post = post_resp.json::<serde_json::Value>().await.unwrap();
    assert_eq!(post["content"], "Test post from integration test!");
    assert_eq!(post["user_id"], user_id);
    let post_id = post["id"].as_str().unwrap().to_string();

    // 4. Edit post
    let edit_body = json!({
        "content": "Updated content from integration test!"
    });

    let edit_resp = client
        .put(&format!("{}/posts/{}", BASE_URL, post_id))
        .header("Authorization", format!("Bearer {}", token))
        .json(&edit_body)
        .send()
        .await
        .expect("Failed to edit post");

    assert_eq!(edit_resp.status(), 200);
    let edited_post = edit_resp.json::<serde_json::Value>().await.unwrap();
    assert_eq!(edited_post["content"], "Updated content from integration test!");
    assert!(edited_post["updated_at"].is_string(), "updated_at should be set after edit");
}

#[tokio::test]
async fn test_post_content_validation() {
    let _lock = lock_test();
    let client = reqwest::Client::new();
    
    // Create and login a user
    let username = format!("val_{}", &uuid::Uuid::new_v4().to_string()[0..8]);
    let create_body = json!({
        "username": username,
        "password": "test"
    });

    let user_resp = client
        .post(&format!("{}/users", BASE_URL))
        .json(&create_body)
        .send()
        .await
        .expect("Failed to create user");

    assert_eq!(user_resp.status(), 201);

    let login_body = json!({
        "username": &username,
        "password": "test"
    });

    let login_resp = client
        .post(&format!("{}/login", BASE_URL))
        .json(&login_body)
        .send()
        .await
        .expect("Failed to login");

    assert_eq!(login_resp.status(), 200);
    let token_data = login_resp.json::<serde_json::Value>().await.unwrap();
    assert!(token_data.get("token").is_some(), "Token field missing in response: {:?}", token_data);
    let token = token_data["token"]
        .as_str()
        .unwrap()
        .to_string();

    // Try empty content
    let empty_body = json!({"content": ""});

    let response = client
        .post(&format!("{}/posts", BASE_URL))
        .header("Authorization", format!("Bearer {}", token))
        .json(&empty_body)
        .send()
        .await
        .expect("Failed to make request");

    assert_eq!(response.status(), 400);

    // Try content > 5000 chars
    let long_content = "a".repeat(5001);
    let long_body = json!({"content": long_content});

    let response = client
        .post(&format!("{}/posts", BASE_URL))
        .header("Authorization", format!("Bearer {}", token))
        .json(&long_body)
        .send()
        .await
        .expect("Failed to make request");

    assert_eq!(response.status(), 400);
}

#[tokio::test]
async fn test_login_invalid_credentials() {
    let _lock = lock_test();
    let client = reqwest::Client::new();
    
    let login_body = json!({
        "username": "nonexistent_user",
        "password": "wrongpass"
    });

    let response = client
        .post(&format!("{}/login", BASE_URL))
        .json(&login_body)
        .send()
        .await
        .expect("Failed to make request");

    assert_eq!(response.status(), 401);
}

#[tokio::test]
async fn test_create_post_requires_auth() {
    let _lock = lock_test();
    let client = reqwest::Client::new();
    
    let body = json!({
        "content": "Test post without auth"
    });

    let response = client
        .post(&format!("{}/posts", BASE_URL))
        .json(&body)
        .send()
        .await
        .expect("Failed to make request");

    assert_eq!(response.status(), 401);
}

#[tokio::test]
async fn test_bio_xss_protection() {
    let _lock = lock_test();
    let client = reqwest::Client::new();
    
    // Create user first
    let username = format!("bio_{}", &uuid::Uuid::new_v4().to_string()[0..8]);
    let create_body = json!({
        "username": username,
        "password": "test123"
    });

    let user_resp = client
        .post(&format!("{}/users", BASE_URL))
        .json(&create_body)
        .send()
        .await
        .expect("Failed to create user");

    assert_eq!(user_resp.status(), 201);
    let _user = user_resp.json::<serde_json::Value>().await.unwrap();

    // Login
    let login_body = json!({
        "username": &username,
        "password": "test123"
    });

    let login_resp = client
        .post(&format!("{}/login", BASE_URL))
        .json(&login_body)
        .send()
        .await
        .expect("Failed to login");

    assert_eq!(login_resp.status(), 200);
    let token_data = login_resp.json::<serde_json::Value>().await.unwrap();
    let token = token_data["token"].as_str().unwrap().to_string();

    // Update profile with XSS payload in bio
    let xss_payload = "<img src=x onerror='alert(\"xss\")'>";
    let update_body = json!({
        "bio": xss_payload
    });

    let update_resp = client
        .put(&format!("{}/profile", BASE_URL))
        .header("Authorization", format!("Bearer {}", token))
        .json(&update_body)
        .send()
        .await
        .expect("Failed to update profile");

    assert_eq!(update_resp.status(), 200);
    let updated_user = update_resp.json::<serde_json::Value>().await.unwrap();
    let stored_bio = updated_user["bio"].as_str().unwrap_or("");
    
    // Bio should not contain img tags or onerror attributes
    assert!(!stored_bio.contains("<img"));
    assert!(!stored_bio.contains("onerror"));
    assert!(!stored_bio.contains("alert"));
}

#[tokio::test]
async fn test_follow_unfollow_user() {
    let _lock = lock_test();
    let client = reqwest::Client::new();
    
    // Create first user
    let username1 = format!("follow1_{}", &uuid::Uuid::new_v4().to_string()[0..8]);
    let user1_body = json!({
        "username": username1,
        "password": "test"
    });
    
    let user1_resp = client
        .post(&format!("{}/users", BASE_URL))
        .json(&user1_body)
        .send()
        .await
        .expect("Failed to create user1");
    
    assert_eq!(user1_resp.status(), 201);
    let user1 = user1_resp.json::<serde_json::Value>().await.unwrap();
    let user1_id = user1["id"].as_str().unwrap().to_string();
    
    // Create second user
    let username2 = format!("follow2_{}", &uuid::Uuid::new_v4().to_string()[0..8]);
    let user2_body = json!({
        "username": username2,
        "password": "test"
    });
    
    let user2_resp = client
        .post(&format!("{}/users", BASE_URL))
        .json(&user2_body)
        .send()
        .await
        .expect("Failed to create user2");
    
    assert_eq!(user2_resp.status(), 201);
    let user2 = user2_resp.json::<serde_json::Value>().await.unwrap();
    let user2_id = user2["id"].as_str().unwrap().to_string();
    
    // Login as user1
    let login_body = json!({
        "username": &username1,
        "password": "test"
    });
    
    let login_resp = client
        .post(&format!("{}/login", BASE_URL))
        .json(&login_body)
        .send()
        .await
        .expect("Failed to login");
    
    assert_eq!(login_resp.status(), 200);
    let token_data = login_resp.json::<serde_json::Value>().await.unwrap();
    let token = token_data["token"].as_str().unwrap().to_string();
    
    // User1 follows user2
    let follow_body = json!({
        "target_user_id": user2_id
    });
    
    let follow_resp = client
        .post(&format!("{}/follow", BASE_URL))
        .header("Authorization", format!("Bearer {}", token))
        .json(&follow_body)
        .send()
        .await
        .expect("Failed to follow user");
    
    assert_eq!(follow_resp.status(), 200);
    let follow_result = follow_resp.json::<serde_json::Value>().await.unwrap();
    assert_eq!(follow_result["status"], "followed");
    
    // Check user1's followings list
    let followings_resp = client
        .get(&format!("{}/followings/{}", BASE_URL, user1_id))
        .send()
        .await
        .expect("Failed to get followings");
    
    assert_eq!(followings_resp.status(), 200);
    let followings = followings_resp.json::<Vec<String>>().await.unwrap();
    assert!(followings.contains(&user2_id), "user2_id should be in user1's followings");
    
    // User1 unfollows user2
    let unfollow_body = json!({
        "target_user_id": user2_id
    });
    
    let unfollow_resp = client
        .post(&format!("{}/unfollow", BASE_URL))
        .header("Authorization", format!("Bearer {}", token))
        .json(&unfollow_body)
        .send()
        .await
        .expect("Failed to unfollow user");
    
    assert_eq!(unfollow_resp.status(), 200);
    let unfollow_result = unfollow_resp.json::<serde_json::Value>().await.unwrap();
    assert_eq!(unfollow_result["status"], "unfollowed");
    
    // Check user1's followings list is now empty
    let followings_resp = client
        .get(&format!("{}/followings/{}", BASE_URL, user1_id))
        .send()
        .await
        .expect("Failed to get followings after unfollow");
    
    assert_eq!(followings_resp.status(), 200);
    let followings = followings_resp.json::<Vec<String>>().await.unwrap();
    assert!(!followings.contains(&user2_id), "user2_id should not be in user1's followings after unfollow");
    assert!(followings.is_empty(), "user1's followings should be empty");
}