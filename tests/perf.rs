use serde_json::json;
use std::time::Instant;

const BASE_URL: &str = "http://127.0.0.1:3000";
const NUM_USERS: usize = 100;
const POSTS_PER_USER: usize = 2;
const LOADED_USER_NAME: &str = "bob";
const LOADED_USER_PASSWORD: &str = "bob";
const LOADED_USER_POSTS: usize = 50;

#[ignore] 
#[tokio::test(flavor = "multi_thread")]
async fn perf_test_users_with_posts() {
    let client = reqwest::Client::new();
    let start = Instant::now();

    println!("\n=== Performance Test ===");
    println!("Creating {} users with {} posts each...", NUM_USERS, POSTS_PER_USER);

    let mut user_credentials = Vec::new();

    // Create users
    let user_creation_start = Instant::now();
    for i in 0..NUM_USERS {
        let username = format!("perf_user_{}_{}", i, uuid::Uuid::new_v4().to_string()[0..8].to_string());
        let password = "password123";

        let create_resp = client
            .post(&format!("{}/users", BASE_URL))
            .json(&json!({
                "username": username,
                "password": password
            }))
            .send()
            .await;

        if let Ok(resp) = create_resp {
            if resp.status() == 201 {
                if let Ok(user) = resp.json::<serde_json::Value>().await {
                    if let Some(user_id) = user["id"].as_str() {
                        user_credentials.push((user_id.to_string(), username, password.to_string()));
                    }
                }
            }
        }

        if (i + 1) % 100 == 0 {
            println!("  Created {}/{} users", i + 1, NUM_USERS);
        }
    }
    let user_creation_time = user_creation_start.elapsed();

    println!(
        "User creation done: {} users in {:.2}s ({:.2} users/sec)",
        user_credentials.len(),
        user_creation_time.as_secs_f64(),
        user_credentials.len() as f64 / user_creation_time.as_secs_f64()
    );

    // Create posts
    let post_creation_start = Instant::now();
    let mut posts_created = 0;
    let mut posts_failed = 0;

    for (idx, (_, username, password)) in user_credentials.iter().enumerate() {
        // Login
        let login_resp = client
            .post(&format!("{}/login", BASE_URL))
            .json(&json!({
                "username": username,
                "password": password
            }))
            .send()
            .await;

        if let Ok(resp) = login_resp {
            if resp.status() == 200 {
                if let Ok(token_data) = resp.json::<serde_json::Value>().await {
                    if let Some(token) = token_data["token"].as_str() {
                        // Create posts for this user
                        for post_num in 0..POSTS_PER_USER {
                            let content = format!(
                                "Post {} from user {} - Perf test at {}",
                                post_num + 1,
                                idx,
                                chrono::Utc::now().to_rfc3339()
                            );

                            let post_resp = client
                                .post(&format!("{}/posts", BASE_URL))
                                .header("Authorization", format!("Bearer {}", token))
                                .json(&json!({ "content": content }))
                                .send()
                                .await;

                            if let Ok(resp) = post_resp {
                                if resp.status() == 201 {
                                    posts_created += 1;
                                } else {
                                    posts_failed += 1;
                                }
                            } else {
                                posts_failed += 1;
                            }
                        }
                    }
                }
            }
        }

        if (idx + 1) % 50 == 0 {
            println!(
                "  Processed {}/{} users ({} posts created)",
                idx + 1,
                user_credentials.len(),
                posts_created
            );
        }
    }
    let post_creation_time = post_creation_start.elapsed();

    let total_time = start.elapsed();
    let total_requests = user_credentials.len() + posts_created + posts_failed;

    println!("\n=== Results ===");
    println!("Total time: {:.2}s", total_time.as_secs_f64());
    println!("User creation: {:.2}s", user_creation_time.as_secs_f64());
    println!("Post creation: {:.2}s", post_creation_time.as_secs_f64());
    println!("Users created: {}", user_credentials.len());
    println!("Posts created: {}", posts_created);
    println!("Posts failed: {}", posts_failed);
    println!("Total requests: {}", total_requests);
    println!(
        "Avg time per request: {:.2}ms",
        (total_time.as_secs_f64() * 1000.0) / total_requests as f64
    );
    println!(
        "Throughput: {:.0} requests/sec",
        total_requests as f64 / total_time.as_secs_f64()
    );
}

#[ignore]
#[tokio::test(flavor = "multi_thread")]
async fn perf_test_load_user_with_many_posts() {
    let client = reqwest::Client::new();
    let start = Instant::now();

    println!("\n=== Load User Performance Test ===");

    let mut user_id = None;
    let mut token = None;

    // Try to login first
    let login_resp = client
        .post(&format!("{}/login", BASE_URL))
        .json(&json!({
            "username": LOADED_USER_NAME,
            "password": LOADED_USER_PASSWORD
        }))
        .send()
        .await;

    if let Ok(resp) = login_resp {
        if resp.status() == 200 {
            if let Ok(token_data) = resp.json::<serde_json::Value>().await {
                if let Some(t) = token_data["token"].as_str() {
                    token = Some(t.to_string());
                    println!("User '{}' already exists, skipping creation", LOADED_USER_NAME);
                }
            }
        }
    }

    // If login failed, try to create user
    if token.is_none() {
        println!(
            "Creating user '{}' with {} posts...",
            LOADED_USER_NAME, LOADED_USER_POSTS
        );

        let create_resp = client
            .post(&format!("{}/users", BASE_URL))
            .json(&json!({
                "username": LOADED_USER_NAME,
                "password": LOADED_USER_PASSWORD
            }))
            .send()
            .await;

        if let Ok(resp) = create_resp {
            if resp.status() == 201 {
                if let Ok(user) = resp.json::<serde_json::Value>().await {
                    user_id = user["id"].as_str().map(|s| s.to_string());
                }
            }
        }

        // Try to login again (user might already exist)
        let login_resp = client
            .post(&format!("{}/login", BASE_URL))
            .json(&json!({
                "username": LOADED_USER_NAME,
                "password": LOADED_USER_PASSWORD
            }))
            .send()
            .await;

        if let Ok(resp) = login_resp {
            if resp.status() == 200 {
                if let Ok(token_data) = resp.json::<serde_json::Value>().await {
                    if let Some(t) = token_data["token"].as_str() {
                        token = Some(t.to_string());
                    }
                }
            }
        }

        if token.is_none() {
            println!("Failed to login or create user");
            return;
        }
    }

    let token = token.unwrap();

    // Create posts
    let post_creation_start = Instant::now();
    let mut posts_created = 0;
    let mut posts_failed = 0;

    for post_num in 0..LOADED_USER_POSTS {
        let content = format!(
            "Post {} - Load test at {}",
            post_num + 1,
            chrono::Utc::now().to_rfc3339()
        );

        let post_resp = client
            .post(&format!("{}/posts", BASE_URL))
            .header("Authorization", format!("Bearer {}", token))
            .json(&json!({ "content": content }))
            .send()
            .await;

        if let Ok(resp) = post_resp {
            if resp.status() == 201 {
                posts_created += 1;
            } else {
                posts_failed += 1;
            }
        } else {
            posts_failed += 1;
        }

        if (post_num + 1) % 10 == 0 {
            println!("  Created {}/{} posts", post_num + 1, LOADED_USER_POSTS);
        }
    }
    let post_creation_time = post_creation_start.elapsed();

    // Fetch user to load posts
    let fetch_start = Instant::now();
    let fetch_resp = client
        .get(&format!("{}/users/{}", BASE_URL, user_id.unwrap_or_default()))
        .send()
        .await;
    let fetch_time = fetch_start.elapsed();

    let total_time = start.elapsed();

    println!("\n=== Results ===");
    println!("Total time: {:.2}s", total_time.as_secs_f64());
    println!("Post creation: {:.2}s", post_creation_time.as_secs_f64());
    println!("Posts created: {}", posts_created);
    println!("Posts failed: {}", posts_failed);
    println!("User fetch time: {:.2}ms", fetch_time.as_millis());
    if let Ok(resp) = fetch_resp {
        println!("User fetch status: {}", resp.status());
    }
}
