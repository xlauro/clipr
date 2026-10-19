use crate::enums::ProcessingStatus;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoSource {
    pub id: Option<i64>,
    pub url: String,
    pub video_id: String,
    pub title: String,
    pub author: String,
    pub duration_seconds: f64,
    pub transcript_path: Option<String>,
    pub local_video_path: Option<String>,
    pub status: ProcessingStatus,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl VideoSource {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: Option<i64>,
        url: impl Into<String>,
        video_id: impl Into<String>,
        title: impl Into<String>,
        author: impl Into<String>,
        duration_seconds: f64,
        transcript_path: Option<String>,
        local_video_path: Option<String>,
        status: ProcessingStatus,
        error_message: Option<String>,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            url: url.into(),
            video_id: video_id.into(),
            title: title.into(),
            author: author.into(),
            duration_seconds,
            transcript_path,
            local_video_path,
            status,
            error_message,
            created_at,
        }
    }

    pub fn from_url(url: &str) -> Self {
        let video_id = Self::extract_video_id(url);
        Self {
            id: None,
            url: url.to_string(),
            video_id,
            title: "Aguardando informações...".to_string(),
            author: String::new(),
            duration_seconds: 0.0,
            transcript_path: None,
            local_video_path: None,
            status: ProcessingStatus::Pending,
            error_message: None,
            created_at: Utc::now(),
        }
    }

    pub fn with_status(&self, new_status: ProcessingStatus, new_error: Option<String>) -> Self {
        Self {
            id: self.id,
            url: self.url.clone(),
            video_id: self.video_id.clone(),
            title: self.title.clone(),
            author: self.author.clone(),
            duration_seconds: self.duration_seconds,
            transcript_path: self.transcript_path.clone(),
            local_video_path: self.local_video_path.clone(),
            status: new_status,
            error_message: new_error,
            created_at: self.created_at,
        }
    }

    pub fn is_valid_youtube_url(url: &str) -> bool {
        if url.trim().is_empty() {
            return false;
        }
        let trimmed = url.trim().to_lowercase();
        trimmed.contains("youtube.com") || trimmed.contains("youtu.be")
    }

    pub fn extract_video_id(url: &str) -> String {
        if url.trim().is_empty() {
            return String::new();
        }
        let trimmed = url.trim();

        let index_of_delimiter = |s: &str| -> Option<usize> {
            let delims = ['&', '?', '#', '/', ' '];
            let mut min_idx = None;
            for d in delims {
                if let Some(idx) = s.find(d) {
                    min_idx = Some(match min_idx {
                        Some(m) => std::cmp::min(m, idx),
                        None => idx,
                    });
                }
            }
            min_idx
        };

        if let Some(pos) = trimmed.find("v=") {
            let sub = &trimmed[pos + 2..];
            return match index_of_delimiter(sub) {
                Some(end) => sub[..end].to_string(),
                None => sub.to_string(),
            };
        } else if let Some(pos) = trimmed.find("youtu.be/") {
            let sub = &trimmed[pos + 9..];
            return match index_of_delimiter(sub) {
                Some(end) => sub[..end].to_string(),
                None => sub.to_string(),
            };
        } else if let Some(pos) = trimmed.find("shorts/") {
            let sub = &trimmed[pos + 7..];
            return match index_of_delimiter(sub) {
                Some(end) => sub[..end].to_string(),
                None => sub.to_string(),
            };
        } else if let Some(pos) = trimmed.find("live/") {
            let sub = &trimmed[pos + 5..];
            return match index_of_delimiter(sub) {
                Some(end) => sub[..end].to_string(),
                None => sub.to_string(),
            };
        }

        trimmed
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                    c
                } else {
                    '_'
                }
            })
            .collect()
    }
}
