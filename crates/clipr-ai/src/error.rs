use thiserror::Error;

#[derive(Debug, Error)]
pub enum AiError {
    #[error("Gemini API error (HTTP {status_code}): {message}")]
    Gemini {
        status_code: u16,
        message: String,
        response_body: Option<String>,
    },

    #[error("OpenAI API error (HTTP {status_code}): {message}")]
    OpenAi {
        status_code: u16,
        message: String,
        response_body: Option<String>,
    },

    #[error("API key not configured: {0}")]
    ApiKeyMissing(String),

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("JSON serialization/deserialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Parsing error: {0}")]
    Parsing(String),

    #[error("Operation unsupported: {0}")]
    Unsupported(String),

    #[error("Operation canceled")]
    Canceled,

    #[error("{0}")]
    Other(String),
}

#[derive(Debug, Clone, Error)]
#[error("Gemini API error (HTTP {status_code}): {message}")]
pub struct GeminiApiError {
    pub status_code: u16,
    pub message: String,
    pub response_body: Option<String>,
}

impl GeminiApiError {
    pub fn new(status_code: u16, message: impl Into<String>, response_body: Option<String>) -> Self {
        Self {
            status_code,
            message: message.into(),
            response_body,
        }
    }

    pub fn is_auth_error(&self) -> bool {
        self.status_code == 401
    }

    pub fn is_client_error(&self) -> bool {
        (400..500).contains(&self.status_code) && self.status_code != 429
    }

    pub fn is_rate_limit(&self) -> bool {
        self.status_code == 429
    }

    pub fn is_overload(&self) -> bool {
        self.status_code == 503
    }
}

#[derive(Debug, Clone, Error)]
#[error("OpenAI API error (HTTP {status_code}): {message}")]
pub struct OpenAiApiError {
    pub status_code: u16,
    pub message: String,
    pub response_body: Option<String>,
}

impl OpenAiApiError {
    pub fn new(status_code: u16, message: impl Into<String>, response_body: Option<String>) -> Self {
        Self {
            status_code,
            message: message.into(),
            response_body,
        }
    }

    pub fn is_auth_error(&self) -> bool {
        self.status_code == 401
    }

    pub fn is_client_error(&self) -> bool {
        (400..500).contains(&self.status_code) && self.status_code != 429
    }

    pub fn is_rate_limit(&self) -> bool {
        self.status_code == 429
    }

    pub fn is_server_error(&self) -> bool {
        (500..600).contains(&self.status_code)
    }
}

impl From<GeminiApiError> for AiError {
    fn from(err: GeminiApiError) -> Self {
        Self::Gemini {
            status_code: err.status_code,
            message: err.message,
            response_body: err.response_body,
        }
    }
}

impl From<OpenAiApiError> for AiError {
    fn from(err: OpenAiApiError) -> Self {
        Self::OpenAi {
            status_code: err.status_code,
            message: err.message,
            response_body: err.response_body,
        }
    }
}
