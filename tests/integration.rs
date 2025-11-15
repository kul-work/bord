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
    let username = format!("flow_test_{}", uuid::Uuid::new_v4());
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
    let username = format!("validation_test_{}", uuid::Uuid::new_v4());
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