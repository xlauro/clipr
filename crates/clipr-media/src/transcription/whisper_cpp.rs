use clipr_domain::WordTimestamp;
use crate::error::MediaError;
use crate::transcription::whisper_trait::WhisperTranscriptionService;
use regex::Regex;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::LazyLock;
use tracing::{error, info};

pub const DEFAULT_MODEL_URL: &str =
    "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin";
pub const DEFAULT_MODEL_NAME: &str = "ggml-base.bin";
pub const MIN_MODEL_SIZE_BYTES: u64 = 100_000_000;

static TIME_STRING_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(\d{2}):(\d{2}):(\d{2})[,.](\d{3})").expect("Valid regex for whisper time string")
});

pub struct WhisperCppManager {
    custom_binary_path: Option<PathBuf>,
    models_directory: PathBuf,
}

impl Default for WhisperCppManager {
    fn default() -> Self {
        Self::new()
    }
}

impl WhisperCppManager {
    pub fn new() -> Self {
        Self {
            custom_binary_path: None,
            models_directory: Self::resolve_default_models_dir(),
        }
    }

    pub fn with_custom_binary(binary_path: impl Into<PathBuf>) -> Self {
        Self {
            custom_binary_path: Some(binary_path.into()),
            models_directory: Self::resolve_default_models_dir(),
        }
    }

    pub fn with_custom_binary_and_models(binary_path: Option<PathBuf>, models_dir: PathBuf) -> Self {
        Self {
            custom_binary_path: binary_path,
            models_directory: models_dir,
        }
    }

    pub fn resolve_whisper_binary(&self) -> Option<PathBuf> {
        if let Some(ref custom) = self.custom_binary_path
            && custom.is_file() {
                return Some(custom.clone());
            }

        // 1. ~/.clippr/bin/whisper-cli ou whisper-cpp ou main
        let home_dir = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let clippr_bin = PathBuf::from(home_dir).join(".clippr").join("bin");
        for name in &["whisper-cli", "whisper-cpp", "main"] {
            let candidate = clippr_bin.join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }

        // 2. PATH do sistema
        if let Ok(path_env) = std::env::var("PATH") {
            for dir in std::env::split_paths(&path_env) {
                for name in &["whisper-cli", "whisper-cpp", "main"] {
                    let candidate = dir.join(name);
                    if candidate.is_file() {
                        return Some(candidate);
                    }
                }
            }
        }

        None
    }

    pub fn get_models_directory(&self) -> &Path {
        &self.models_directory
    }

    pub fn get_model_path(&self) -> PathBuf {
        self.models_directory.join(DEFAULT_MODEL_NAME)
    }

    pub fn ensure_model_downloaded(&self) -> Result<PathBuf, MediaError> {
        let model_file = self.get_model_path();
        if model_file.is_file() && fs::metadata(&model_file)?.len() >= MIN_MODEL_SIZE_BYTES {
            return Ok(model_file);
        }

        fs::create_dir_all(&self.models_directory)?;
        let temp_download = self.models_directory.join(format!("{DEFAULT_MODEL_NAME}.download"));

        info!(
            "Modelo ggml-base.bin não encontrado. Iniciando download de {} para {}...",
            DEFAULT_MODEL_URL,
            temp_download.display()
        );

        let resp = reqwest::blocking::get(DEFAULT_MODEL_URL)?;
        if !resp.status().is_success() {
            return Err(MediaError::General(format!(
                "Falha no download do modelo Whisper: HTTP {}",
                resp.status()
            )));
        }

        let mut file = fs::File::create(&temp_download)?;
        let bytes = resp.bytes()?;
        file.write_all(&bytes)?;

        if fs::metadata(&temp_download)?.len() < MIN_MODEL_SIZE_BYTES {
            let _ = fs::remove_file(&temp_download);
            return Err(MediaError::General(
                "O arquivo baixado do modelo está incompleto".to_string(),
            ));
        }

        fs::rename(&temp_download, &model_file)?;
        info!("Modelo Whisper baixado com sucesso em: {}", model_file.display());
        Ok(model_file)
    }

    pub fn parse_whisper_cpp_json(json_content: &str) -> Result<Vec<WordTimestamp>, MediaError> {
        let trimmed = json_content.trim();
        if trimmed.is_empty() {
            return Ok(Vec::new());
        }

        let root: serde_json::Value = serde_json::from_str(trimmed)?;
        let mut result = Vec::new();

        let Some(transcription) = root.get("transcription").and_then(|t| t.as_array()) else {
            return Ok(Vec::new());
        };

        for item in transcription {
            if let Some(tokens) = item.get("tokens").and_then(|t| t.as_array())
                && !tokens.is_empty() {
                    for token in tokens {
                        let text = token.get("text").and_then(|v| v.as_str()).unwrap_or("");
                        let start = Self::parse_timestamp_or_offset(
                            token.get("timestamps").and_then(|t| t.get("from")),
                            token.get("offsets").and_then(|o| o.get("from")),
                        );
                        let end = Self::parse_timestamp_or_offset(
                            token.get("timestamps").and_then(|t| t.get("to")),
                            token.get("offsets").and_then(|o| o.get("to")),
                        );
                        let clean = text.trim();
                        if !clean.is_empty() {
                            result.push(WordTimestamp::new(clean, start, start.max(end)));
                        }
                    }
                    continue;
                }

            // Fallback para o texto do segmento se não houver tokens
            let text = item.get("text").and_then(|v| v.as_str()).unwrap_or("");
            let start = Self::parse_timestamp_or_offset(
                item.get("timestamps").and_then(|t| t.get("from")),
                item.get("offsets").and_then(|o| o.get("from")),
            );
            let end = Self::parse_timestamp_or_offset(
                item.get("timestamps").and_then(|t| t.get("to")),
                item.get("offsets").and_then(|o| o.get("to")),
            );
            let clean = text.trim();
            if !clean.is_empty() {
                result.push(WordTimestamp::new(clean, start, start.max(end)));
            }
        }

        Ok(result)
    }

    fn parse_timestamp_or_offset(
        time_node: Option<&serde_json::Value>,
        offset_node: Option<&serde_json::Value>,
    ) -> f64 {
        if let Some(t_str) = time_node.and_then(|t| t.as_str())
            && let Some(caps) = TIME_STRING_PATTERN.captures(t_str.trim()) {
                let hours: f64 = caps[1].parse().unwrap_or(0.0);
                let mins: f64 = caps[2].parse().unwrap_or(0.0);
                let secs: f64 = caps[3].parse().unwrap_or(0.0);
                let millis: f64 = caps[4].parse().unwrap_or(0.0);
                return (hours * 3600.0) + (mins * 60.0) + secs + (millis / 1000.0);
            }

        if let Some(offset) = offset_node.and_then(|o| o.as_f64()) {
            return offset / 1000.0;
        }

        0.0
    }

    fn resolve_default_models_dir() -> PathBuf {
        let home_dir = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home_dir).join(".clippr").join("models")
    }
}

impl WhisperTranscriptionService for WhisperCppManager {
    fn is_available(&self) -> bool {
        self.resolve_whisper_binary().is_some()
    }

    fn transcribe(&self, audio_file_path: &Path) -> Result<Vec<WordTimestamp>, MediaError> {
        let Some(binary) = self.resolve_whisper_binary() else {
            return Err(MediaError::WhisperUnavailable(
                "Binário whisper-cli local não encontrado no PATH ou em ~/.clippr/bin.".to_string(),
            ));
        };

        if !audio_file_path.exists() {
            return Err(MediaError::InvalidArgument(format!(
                "Arquivo de áudio não encontrado: {}",
                audio_file_path.display()
            )));
        }

        let model_file = self.ensure_model_downloaded()?;

        let temp_dir = tempfile::tempdir()?;
        let output_prefix = temp_dir.path().join("transcription");

        let mut cmd = Command::new(&binary);
        cmd.arg("-m")
            .arg(&model_file)
            .arg("-f")
            .arg(audio_file_path)
            .arg("-oj")
            .arg("-of")
            .arg(&output_prefix)
            .arg("-ml")
            .arg("1");

        info!("Executando Whisper local: {:?}", cmd);

        let output = cmd.output()?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            error!("Falha no whisper-cli local: {}", stderr);
            return Err(MediaError::General(format!(
                "Falha no whisper-cli local (exit code {}): {}",
                output.status.code().unwrap_or(-1),
                stderr
            )));
        }

        let json_output = PathBuf::from(format!("{}.json", output_prefix.display()));
        if !json_output.exists() {
            return Err(MediaError::General(format!(
                "Arquivo JSON de transcrição não gerado: {}",
                json_output.display()
            )));
        }

        let content = fs::read_to_string(&json_output)?;
        Self::parse_whisper_cpp_json(&content)
    }
}
