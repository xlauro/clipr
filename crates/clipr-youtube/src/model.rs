use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct YouTubeOAuthTokens {
    #[serde(rename = "access_token")]
    pub access_token: String,
    #[serde(rename = "refresh_token", default)]
    pub refresh_token: Option<String>,
    #[serde(rename = "expires_in")]
    pub expires_in_seconds: u64,
    #[serde(rename = "obtained_at", default = "Utc::now")]
    pub obtained_at: DateTime<Utc>,
    #[serde(rename = "scope", default)]
    pub scope: String,
}

impl YouTubeOAuthTokens {
    pub fn new(
        access_token: impl Into<String>,
        refresh_token: Option<String>,
        expires_in_seconds: u64,
        obtained_at: Option<DateTime<Utc>>,
        scope: Option<String>,
    ) -> Self {
        Self {
            access_token: access_token.into(),
            refresh_token,
            expires_in_seconds,
            obtained_at: obtained_at.unwrap_or_else(Utc::now),
            scope: scope.unwrap_or_default(),
        }
    }

    pub fn is_expired(&self) -> bool {
        self.is_expired_with_buffer(Duration::from_secs(60))
    }

    pub fn is_expired_with_buffer(&self, safety_buffer: Duration) -> bool {
        let expiration = self.obtained_at + chrono::Duration::seconds(self.expires_in_seconds as i64);
        let buffer_chrono = chrono::Duration::from_std(safety_buffer).unwrap_or_else(|_| chrono::Duration::seconds(60));
        Utc::now() + buffer_chrono > expiration
    }

    pub fn remaining_lifetime(&self) -> Duration {
        let expiration = self.obtained_at + chrono::Duration::seconds(self.expires_in_seconds as i64);
        let diff = expiration - Utc::now();
        if diff.num_milliseconds() <= 0 {
            Duration::ZERO
        } else {
            Duration::from_millis(diff.num_milliseconds() as u64)
        }
    }

    pub fn remaining_seconds(&self) -> u64 {
        self.remaining_lifetime().as_secs()
    }

    pub fn with_refreshed_token(
        &self,
        new_access_token: impl Into<String>,
        new_expires_in_seconds: u64,
        new_scope: Option<&str>,
    ) -> Self {
        let effective_scope = match new_scope {
            Some(s) if !s.trim().is_empty() => s.to_string(),
            _ => self.scope.clone(),
        };

        Self {
            access_token: new_access_token.into(),
            refresh_token: self.refresh_token.clone(),
            expires_in_seconds: new_expires_in_seconds,
            obtained_at: Utc::now(),
            scope: effective_scope,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct YouTubeChannelInfo {
    pub id: String,
    pub title: String,
    pub description: String,
    #[serde(rename = "customUrl", default)]
    pub custom_url: Option<String>,
    #[serde(rename = "avatarUrl", default)]
    pub avatar_url: Option<String>,
    #[serde(rename = "subscriberCount", default)]
    pub subscriber_count: u64,
}

impl YouTubeChannelInfo {
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        description: impl Into<String>,
        custom_url: Option<String>,
        avatar_url: Option<String>,
        subscriber_count: u64,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            description: description.into(),
            custom_url,
            avatar_url,
            subscriber_count,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum VideoPrivacy {
    #[serde(rename = "public")]
    Public,
    #[serde(rename = "unlisted")]
    Unlisted,
    #[default]
    #[serde(rename = "private")]
    Private,
}

impl VideoPrivacy {
    pub fn api_value(&self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Unlisted => "unlisted",
            Self::Private => "private",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Public => "Público",
            Self::Unlisted => "Não listado",
            Self::Private => "Privado",
        }
    }

    pub fn from_str_loose(value: &str) -> Self {
        let clean = value.trim().to_lowercase();
        match clean.as_str() {
            "public" | "público" | "publico" => Self::Public,
            "unlisted" | "não listado" | "nao listado" => Self::Unlisted,
            _ => Self::Private,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoUploadRequest {
    pub video_file: PathBuf,
    pub title: String,
    pub description: String,
    pub tags: Vec<String>,
    pub privacy: VideoPrivacy,
    pub made_for_kids: bool,
    pub custom_thumbnail: Option<PathBuf>,
}

impl VideoUploadRequest {
    pub fn new(
        video_file: impl Into<PathBuf>,
        title: impl Into<String>,
        description: impl Into<String>,
        tags: Vec<String>,
        privacy: VideoPrivacy,
    ) -> Self {
        Self {
            video_file: video_file.into(),
            title: title.into(),
            description: description.into(),
            tags,
            privacy,
            made_for_kids: false,
            custom_thumbnail: None,
        }
    }

    pub fn with_options(
        video_file: impl Into<PathBuf>,
        title: impl Into<String>,
        description: impl Into<String>,
        tags: Vec<String>,
        privacy: VideoPrivacy,
        made_for_kids: bool,
        custom_thumbnail: Option<PathBuf>,
    ) -> Self {
        Self {
            video_file: video_file.into(),
            title: title.into(),
            description: description.into(),
            tags,
            privacy,
            made_for_kids,
            custom_thumbnail,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UploadProgress {
    pub bytes_uploaded: u64,
    pub total_bytes: u64,
    pub percentage: f64,
    pub is_complete: bool,
    pub video_id: Option<String>,
}

impl UploadProgress {
    pub fn started(total_bytes: u64) -> Self {
        Self::in_progress(0, total_bytes)
    }

    pub fn in_progress(bytes_uploaded: u64, total_bytes: u64) -> Self {
        let percentage = if total_bytes > 0 {
            ((bytes_uploaded as f64 / total_bytes as f64) * 100.0).clamp(0.0, 100.0)
        } else {
            0.0
        };

        Self {
            bytes_uploaded,
            total_bytes,
            percentage,
            is_complete: false,
            video_id: None,
        }
    }

    pub fn complete(total_bytes: u64, video_id: impl Into<String>) -> Self {
        Self {
            bytes_uploaded: total_bytes,
            total_bytes,
            percentage: 100.0,
            is_complete: true,
            video_id: Some(video_id.into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UploadResult {
    pub video_id: String,
    pub video_url: String,
    pub title: String,
    pub privacy: VideoPrivacy,
}

impl UploadResult {
    pub fn new(video_id: impl Into<String>, title: impl Into<String>, privacy: VideoPrivacy) -> Self {
        let id = video_id.into();
        let url = format!("https://www.youtube.com/watch?v={id}");
        Self {
            video_id: id,
            video_url: url,
            title: title.into(),
            privacy,
        }
    }

    pub fn with_custom_url(
        video_id: impl Into<String>,
        video_url: impl Into<String>,
        title: impl Into<String>,
        privacy: VideoPrivacy,
    ) -> Self {
        Self {
            video_id: video_id.into(),
            video_url: video_url.into(),
            title: title.into(),
            privacy,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YouTubeUploadPayload {
    pub video_path: PathBuf,
    pub title: String,
    pub description: String,
    pub tags: Vec<String>,
    pub privacy_status: String,
}
