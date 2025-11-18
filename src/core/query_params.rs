use std::collections::HashMap;

/// Parse query parameters from a URI string
/// 
/// Handles URL decoding and returns a HashMap of parameter key-value pairs.
/// Multiple values for the same key are not supported (only the last is kept).
///
/// # Example
/// ```
/// let params = parse_query_params("/path?user=john&page=2");
/// assert_eq!(params.get("user"), Some(&"john".to_string()));
/// assert_eq!(params.get("page"), Some(&"2".to_string()));
/// ```
pub fn parse_query_params(uri: &str) -> HashMap<String, String> {
    let mut params = HashMap::new();
    
    if let Some(query_start) = uri.find('?') {
        let query = &uri[query_start + 1..];
        for param in query.split('&') {
            if let Some(eq_idx) = param.find('=') {
                let key = &param[..eq_idx];
                let encoded_value = &param[eq_idx + 1..];
                let decoded = urlencoding::decode(encoded_value)
                    .unwrap_or(std::borrow::Cow::Borrowed(encoded_value))
                    .to_string();
                params.insert(key.to_string(), decoded);
            } else {
                // Flag parameter without value
                params.insert(param.to_string(), String::new());
            }
        }
    }
    
    params
}

/// Get a string parameter from parsed query params with optional default
pub fn get_string(params: &HashMap<String, String>, key: &str, default: Option<&str>) -> Option<String> {
    params.get(key)
        .map(|s| s.clone())
        .or_else(|| default.map(|d| d.to_string()))
}

/// Get a boolean flag parameter (e.g., ?all=true)
pub fn get_bool_flag(params: &HashMap<String, String>, key: &str) -> bool {
    params.get(key)
        .map(|v| v == "true")
        .unwrap_or(false)
}

/// Get an integer parameter with validation and default
pub fn get_int(params: &HashMap<String, String>, key: &str, default: usize) -> usize {
    params.get(key)
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(default)
        .max(1)
}
