use thiserror::Error;

#[derive(Debug, Error)]
pub enum YouTubeError {
    #[error("YouTube Auth error: {0}")]
    Auth(#[from] YouTubeAuthError),

    #[error("YouTube API error: {0}")]
    Api(#[from] YouTubeApiError),

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("{0}")]
    Other(String),
}

#[derive(Debug, Clone, Error)]
#[error("YouTube Auth error: {message}")]
pub struct YouTubeAuthError {
    pub message: String,
}

impl YouTubeAuthError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, Error)]
#[error("YouTube API error (HTTP {status_code}): {message}")]
pub struct YouTubeApiError {
    pub status_code: u16,
    pub message: String,
}

impl YouTubeApiError {
    pub fn new(status_code: u16, message: impl Into<String>) -> Self {
        Self {
            status_code,
            message: message.into(),
        }
    }

    pub fn with_message(message: impl Into<String>) -> Self {
        Self {
            status_code: 0,
            message: message.into(),
        }
    }
}
