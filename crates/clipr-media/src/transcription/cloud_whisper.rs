use clipr_domain::{AppSettings, WordTimestamp};
use crate::error::MediaError;
use crate::transcription::whisper_trait::WhisperTranscriptionService;
use std::fs;
use std::path::Path;
use tracing::{error, info};

pub const GROQ_ENDPOINT: &str = "https://api.groq.com/openai/v1/audio/transcriptions";
pub const GROQ_MODEL: &str = "whisper-large-v3";
pub const OPENAI_ENDPOINT: &str = "https://api.openai.com/v1/audio/transcriptions";
pub const OPENAI_MODEL: &str = "whisper-1";

pub struct CloudWhisperClient {
    api_key: String,
    endpoint_url: String,
    model: String,
}

impl Default for CloudWhisperClient {
    fn default() -> Self {
        Self::new()
    }
}

impl CloudWhisperClient {
    pub fn new() -> Self {
        let groq_key = Self::resolve_groq_api_key();
        let openai_key = Self::resolve_openai_api_key();

        if !groq_key.is_empty() {
            Self {
                api_key: groq_key,
                endpoint_url: GROQ_ENDPOINT.to_string(),
                model: GROQ_MODEL.to_string(),
            }
        } else if !openai_key.is_empty() {
            Self {
                api_key: openai_key,
                endpoint_url: OPENAI_ENDPOINT.to_string(),
                model: OPENAI_MODEL.to_string(),
            }
        } else {
            Self {
                api_key: String::new(),
                endpoint_url: GROQ_ENDPOINT.to_string(),
                model: GROQ_MODEL.to_string(),
            }
        }
    }

    pub fn from_settings(settings: &AppSettings) -> Self {
        let groq_key = Self::resolve_groq_api_key();
        let openai_key = if !settings.openai_api_key.trim().is_empty() {
            settings.openai_api_key.trim().to_string()
        } else {
            Self::resolve_openai_api_key()
        };

        if !groq_key.is_empty() {
            Self {
                api_key: groq_key,
                endpoint_url: GROQ_ENDPOINT.to_string(),
                model: GROQ_MODEL.to_string(),
            }
        } else if !openai_key.is_empty() {
            Self {
                api_key: openai_key,
                endpoint_url: OPENAI_ENDPOINT.to_string(),
                model: OPENAI_MODEL.to_string(),
            }
        } else {
            Self {
                api_key: String::new(),
                endpoint_url: GROQ_ENDPOINT.to_string(),
                model: GROQ_MODEL.to_string(),
            }
        }
    }

    pub fn custom(api_key: impl Into<String>, endpoint_url: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            endpoint_url: endpoint_url.into(),
            model: model.into(),
        }
    }

    pub fn get_endpoint_url(&self) -> &str {
        &self.endpoint_url
    }

    pub fn get_model(&self) -> &str {
        &self.model
    }

    pub fn parse_whisper_verbose_json_response(json_body: &str) -> Result<Vec<WordTimestamp>, MediaError> {
        let trimmed = json_body.trim();
        if trimmed.is_empty() {
            return Ok(Vec::new());
        }

        let root: serde_json::Value = serde_json::from_str(trimmed)?;
        let mut result = Vec::new();

        // Prioridade 1: campo "words" no nó raiz
        if let Some(words_node) = root.get("words").and_then(|w| w.as_array())
            && !words_node.is_empty() {
                for w in words_node {
                    let word = w.get("word").and_then(|v| v.as_str()).unwrap_or("");
                    let start = w.get("start").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    let end = w.get("end").and_then(|v| v.as_f64()).unwrap_or(start);
                    if !word.trim().is_empty() {
                        result.push(WordTimestamp::new(word.trim(), start, end));
                    }
                }
                return Ok(result);
            }

        // Prioridade 2: dentro de segments[].words
        if let Some(segments_node) = root.get("segments").and_then(|s| s.as_array()) {
            for seg in segments_node {
                if let Some(seg_words) = seg.get("words").and_then(|w| w.as_array()) {
                    for w in seg_words {
                        let word = w.get("word").and_then(|v| v.as_str()).unwrap_or("");
                        let start = w.get("start").and_then(|v| v.as_f64()).unwrap_or(0.0);
                        let end = w.get("end").and_then(|v| v.as_f64()).unwrap_or(start);
                        if !word.trim().is_empty() {
                            result.push(WordTimestamp::new(word.trim(), start, end));
                        }
                    }
                }
            }
        }

        Ok(result)
    }

    pub fn resolve_groq_api_key() -> String {
        if let Ok(key) = std::env::var("GROQ_API_KEY")
            && !key.trim().is_empty() {
                return key.trim().to_string();
            }
        AppSettings::resolve_key_from_env_or_file("GROQ_API_KEY")
    }

    pub fn resolve_openai_api_key() -> String {
        AppSettings::resolve_default_openai_api_key()
    }
}

impl WhisperTranscriptionService for CloudWhisperClient {
    fn is_available(&self) -> bool {
        !self.api_key.trim().is_empty()
    }

    fn transcribe(&self, audio_file_path: &Path) -> Result<Vec<WordTimestamp>, MediaError> {
        if !self.is_available() {
            return Err(MediaError::WhisperUnavailable(
                "Nenhuma chave de API configurada para Cloud Whisper (GROQ_API_KEY ou OPENAI_API_KEY ausentes).".to_string(),
            ));
        }

        if !audio_file_path.exists() {
            return Err(MediaError::InvalidArgument(format!(
                "Arquivo de áudio não encontrado: {}",
                audio_file_path.display()
            )));
        }

        let audio_bytes = fs::read(audio_file_path)?;
        if audio_bytes.is_empty() {
            return Err(MediaError::InvalidArgument(format!(
                "Arquivo de áudio está vazio: {}",
                audio_file_path.display()
            )));
        }

        let file_name = audio_file_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let part = reqwest::blocking::multipart::Part::bytes(audio_bytes)
            .file_name(file_name)
            .mime_str("audio/wav")
            .map_err(|e| MediaError::General(e.to_string()))?;

        let form = reqwest::blocking::multipart::Form::new()
            .text("model", self.model.clone())
            .text("response_format", "verbose_json")
            .text("timestamp_granularities[]", "word")
            .part("file", part);

        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(90))
            .build()?;

        info!(
            "Enviando requisição de transcrição Whisper para {} (modelo: {})...",
            self.endpoint_url, self.model
        );

        let resp = client
            .post(&self.endpoint_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .multipart(form)
            .send()?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().unwrap_or_default();
            error!("Erro na API Whisper ({}) Status {}: {}", self.endpoint_url, status, body);
            return Err(MediaError::General(format!(
                "Falha na API Whisper (HTTP {status}): {body}"
            )));
        }

        let json_body = resp.text()?;
        Self::parse_whisper_verbose_json_response(&json_body)
    }
}
