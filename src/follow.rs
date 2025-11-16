use spin_sdk::key_value::Store;

pub fn follow_user(store: &Store, follower_id: &str, following_id: &str) -> anyhow::Result<()> {
    let followings_key = format!("followings:{}", follower_id);
    let mut followings: Vec<String> = store
        .get_json(&followings_key)?
        .unwrap_or_default();
    
    if !followings.contains(&following_id.to_string()) {
        followings.push(following_id.to_string());
        store.set_json(&followings_key, &followings)?;
    }
    
    Ok(())
}

pub fn unfollow_user(store: &Store, follower_id: &str, following_id: &str) -> anyhow::Result<()> {
    let followings_key = format!("followings:{}", follower_id);
    let mut followings: Vec<String> = store
        .get_json(&followings_key)?
        .unwrap_or_default();
    
    followings.retain(|id| id != following_id);
    store.set_json(&followings_key, &followings)?;
    
    Ok(())
}

pub fn get_followings(store: &Store, user_id: &str) -> anyhow::Result<Vec<String>> {
    let followings_key = format!("followings:{}", user_id);
    let followings: Vec<String> = store
        .get_json(&followings_key)?
        .unwrap_or_default();
    
    Ok(followings)
}

pub fn get_followers(store: &Store, user_id: &str) -> anyhow::Result<Vec<String>> {
    let users: Vec<String> = store.get_json("users_list")?.unwrap_or_default();
    let mut followers = Vec::new();
    
    for id in users {
        let followings_key = format!("followings:{}", id);
        if let Ok(Some(followings)) = store.get_json::<Vec<String>>(&followings_key) {
            if followings.contains(&user_id.to_string()) {
                followers.push(id);
            }
        }
    }
    
    Ok(followers)
}
