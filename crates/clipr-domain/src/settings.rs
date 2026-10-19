use crate::enums::{AiProvider, OrientationMode};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub output_directory: String,
    pub min_virality_score: i32,
    pub auto_process_above_threshold: bool,
    pub default_orientation: OrientationMode,
    pub auto_vertical_max_duration_seconds: f64,
    pub burn_subtitles: bool,
    pub cpu_threads: i32,
    pub gemini_api_key: String,
    pub gemini_model: String,
    pub ai_provider: AiProvider,
    pub openai_api_key: String,
    pub openai_model: String,
    pub youtube_client_id: String,
    pub youtube_client_secret: String,
    pub youtube_access_token: String,
    pub youtube_refresh_token: String,
    pub youtube_token_expires_at: i64,
    pub youtube_channel_title: String,
    pub youtube_channel_id: String,
    pub youtube_channel_avatar_url: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self::default_settings()
    }
}

impl AppSettings {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        output_directory: Option<String>,
        min_virality_score: i32,
        auto_process_above_threshold: bool,
        default_orientation: Option<OrientationMode>,
        auto_vertical_max_duration_seconds: f64,
        burn_subtitles: bool,
        cpu_threads: i32,
        gemini_api_key: Option<String>,
        gemini_model: Option<String>,
        ai_provider: Option<AiProvider>,
        openai_api_key: Option<String>,
        openai_model: Option<String>,
        youtube_client_id: Option<String>,
        youtube_client_secret: Option<String>,
        youtube_access_token: Option<String>,
        youtube_refresh_token: Option<String>,
        youtube_token_expires_at: i64,
        youtube_channel_title: Option<String>,
        youtube_channel_id: Option<String>,
        youtube_channel_avatar_url: Option<String>,
    ) -> Self {
        let home_dir = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let default_out = PathBuf::from(home_dir).join("ClipprExports").to_string_lossy().to_string();

        Self {
            output_directory: output_directory.unwrap_or(default_out),
            min_virality_score,
            auto_process_above_threshold,
            default_orientation: default_orientation.unwrap_or(OrientationMode::Auto),
            auto_vertical_max_duration_seconds,
            burn_subtitles,
            cpu_threads,
            gemini_api_key: gemini_api_key.unwrap_or_default(),
            gemini_model: gemini_model
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| "gemini-2.5-flash".to_string()),
            ai_provider: ai_provider.unwrap_or(AiProvider::Gemini),
            openai_api_key: openai_api_key.unwrap_or_default(),
            openai_model: openai_model
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| "gpt-4o-mini".to_string()),
            youtube_client_id: youtube_client_id.unwrap_or_default(),
            youtube_client_secret: youtube_client_secret.unwrap_or_default(),
            youtube_access_token: youtube_access_token.unwrap_or_default(),
            youtube_refresh_token: youtube_refresh_token.unwrap_or_default(),
            youtube_token_expires_at,
            youtube_channel_title: youtube_channel_title.unwrap_or_default(),
            youtube_channel_id: youtube_channel_id.unwrap_or_default(),
            youtube_channel_avatar_url: youtube_channel_avatar_url.unwrap_or_default(),
        }
    }

    pub fn default_settings() -> Self {
        let home_dir = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let default_out = PathBuf::from(home_dir).join("ClipprExports").to_string_lossy().to_string();
        let api_key = Self::resolve_default_api_key();
        let openai_key = Self::resolve_default_openai_api_key();
        let yt_client_id = Self::resolve_default_youtube_client_id();
        let yt_client_secret = Self::resolve_default_youtube_client_secret();

        Self {
            output_directory: default_out,
            min_virality_score: 90,
            auto_process_above_threshold: false,
            default_orientation: OrientationMode::Auto,
            auto_vertical_max_duration_seconds: 60.0,
            burn_subtitles: true,
            cpu_threads: 0,
            gemini_api_key: api_key,
            gemini_model: "gemini-2.5-flash".to_string(),
            ai_provider: AiProvider::Gemini,
            openai_api_key: openai_key,
            openai_model: "gpt-4o-mini".to_string(),
            youtube_client_id: yt_client_id,
            youtube_client_secret: yt_client_secret,
            youtube_access_token: String::new(),
            youtube_refresh_token: String::new(),
            youtube_token_expires_at: 0,
            youtube_channel_title: String::new(),
            youtube_channel_id: String::new(),
            youtube_channel_avatar_url: String::new(),
        }
    }

    pub fn is_youtube_connected(&self) -> bool {
        !self.youtube_refresh_token.trim().is_empty() || !self.youtube_access_token.trim().is_empty()
    }

    pub fn with_youtube_credentials(&self, client_id: &str, client_secret: &str) -> Self {
        let mut clone = self.clone();
        clone.youtube_client_id = client_id.to_string();
        clone.youtube_client_secret = client_secret.to_string();
        clone
    }

    pub fn with_youtube_tokens(
        &self,
        access_token: &str,
        refresh_token: &str,
        expires_at: i64,
        channel_title: Option<&str>,
        channel_id: Option<&str>,
        channel_avatar_url: Option<&str>,
    ) -> Self {
        let mut clone = self.clone();
        clone.youtube_access_token = access_token.to_string();
        clone.youtube_refresh_token = refresh_token.to_string();
        clone.youtube_token_expires_at = expires_at;
        if let Some(ct) = channel_title {
            clone.youtube_channel_title = ct.to_string();
        }
        if let Some(cid) = channel_id {
            clone.youtube_channel_id = cid.to_string();
        }
        if let Some(cav) = channel_avatar_url {
            clone.youtube_channel_avatar_url = cav.to_string();
        }
        clone
    }

    pub fn without_youtube_auth(&self) -> Self {
        let mut clone = self.clone();
        clone.youtube_access_token = String::new();
        clone.youtube_refresh_token = String::new();
        clone.youtube_token_expires_at = 0;
        clone.youtube_channel_title = String::new();
        clone.youtube_channel_id = String::new();
        clone.youtube_channel_avatar_url = String::new();
        clone
    }

    pub fn resolve_default_api_key() -> String {
        Self::resolve_key_from_env_or_file("GEMINI_API_KEY")
    }

    pub fn resolve_default_openai_api_key() -> String {
        Self::resolve_key_from_env_or_file("OPENAI_API_KEY")
    }

    pub fn resolve_default_youtube_client_id() -> String {
        Self::resolve_key_from_env_or_file("YOUTUBE_CLIENT_ID")
    }

    pub fn resolve_default_youtube_client_secret() -> String {
        Self::resolve_key_from_env_or_file("YOUTUBE_CLIENT_SECRET")
    }

    #[allow(clippy::collapsible_if)]
    pub fn resolve_key_from_env_or_file(var_name: &str) -> String {
        if let Ok(val) = std::env::var(var_name) {
            if !val.trim().is_empty() {
                return val.trim().to_string();
            }
        }
        Self::resolve_key_from_files(var_name)
    }

    #[allow(clippy::collapsible_if)]
    fn resolve_key_from_files(var_name: &str) -> String {
        let pattern_str = format!(
            r#"^\s*(?:export\s+)?{}\s*=\s*["']?([^"'#\s]+)"#,
            regex::escape(var_name)
        );
        let Ok(re) = Regex::new(&pattern_str) else {
            return String::new();
        };

        if let Ok(home) = std::env::var("HOME") {
            for file_name in [".zshrc", ".bashrc", ".env"] {
                let p = Path::new(&home).join(file_name);
                if p.is_file() {
                    if let Ok(file) = File::open(&p) {
                        let reader = BufReader::new(file);
                        for line in reader.lines().map_while(Result::ok) {
                            if let Some(caps) = re.captures(line.trim()) {
                                if let Some(m) = caps.get(1) {
                                    return m.as_str().trim().to_string();
                                }
                            }
                        }
                    }
                }
            }
        }

        let local_env = Path::new(".env");
        if local_env.is_file() {
            if let Ok(file) = File::open(local_env) {
                let reader = BufReader::new(file);
                for line in reader.lines().map_while(Result::ok) {
                    if let Some(caps) = re.captures(line.trim()) {
                        if let Some(m) = caps.get(1) {
                            return m.as_str().trim().to_string();
                        }
                    }
                }
            }
        }

        String::new()
    }
}
