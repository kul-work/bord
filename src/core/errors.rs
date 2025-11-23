use spin_sdk::http::Response;
use std::fmt;

#[derive(Debug)]
pub enum ApiError {
    BadRequest(String),
    Unauthorized,
    Forbidden,
    NotFound(String),
    Conflict(String),
    InternalError(String),
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApiError::BadRequest(msg) => write!(f, "Bad Request: {}", msg),
            ApiError::Unauthorized => write!(f, "Unauthorized"),
            ApiError::Forbidden => write!(f, "Forbidden"),
            ApiError::NotFound(msg) => write!(f, "Not Found: {}", msg),
            ApiError::Conflict(msg) => write!(f, "Conflict: {}", msg),
            ApiError::InternalError(msg) => write!(f, "Internal Error: {}", msg),
        }
    }
}

impl From<ApiError> for Response {
    fn from(err: ApiError) -> Self {
        match err {
            ApiError::BadRequest(msg) => Response::builder()
                .status(400)
                .header("Content-Type", "application/json")
                .body(serde_json::to_vec(&serde_json::json!({"error": msg})).unwrap())
                .build(),
            ApiError::Unauthorized => Response::builder()
                .status(401)
                .header("Content-Type", "application/json")
                .body(serde_json::to_vec(&serde_json::json!({"error": "Unauthorized"})).unwrap())
                .build(),
            ApiError::Forbidden => Response::builder()
                .status(403)
                .header("Content-Type", "application/json")
                .body(serde_json::to_vec(&serde_json::json!({"error": "Forbidden"})).unwrap())
                .build(),
            ApiError::NotFound(msg) => Response::builder()
                .status(404)
                .header("Content-Type", "application/json")
                .body(serde_json::to_vec(&serde_json::json!({"error": msg})).unwrap())
                .build(),
            ApiError::Conflict(msg) => Response::builder()
                .status(409)
                .header("Content-Type", "application/json")
                .body(serde_json::to_vec(&serde_json::json!({"error": msg})).unwrap())
                .build(),
            ApiError::InternalError(msg) => Response::builder()
                .status(500)
                .header("Content-Type", "application/json")
                .body(serde_json::to_vec(&serde_json::json!({"error": msg})).unwrap())
                .build(),
        }
    }
}

impl std::error::Error for ApiError {}

// Implement conversion from anyhow::Error to ApiError for internal errors
impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        ApiError::InternalError(err.to_string())
    }
}
