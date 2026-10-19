use clipr_domain::{AppSettings, EventBus, TranscriptSegment, WordTimestamp};
use crate::error::MediaError;
use crate::subtitles::WordTimestampInterpolator;
use crate::transcription::audio_extractor::AudioExtractor;
use crate::transcription::cloud_whisper::CloudWhisperClient;
use crate::transcription::whisper_cpp::WhisperCppManager;
use crate::transcription::whisper_trait::WhisperTranscriptionService;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

pub struct WhisperService {
    cloud_client: CloudWhisperClient,
    local_manager: WhisperCppManager,
}

impl Default for WhisperService {
    fn default() -> Self {
        Self::new()
    }
}

impl WhisperService {
    pub fn new() -> Self {
        Self {
            cloud_client: CloudWhisperClient::new(),
            local_manager: WhisperCppManager::new(),
        }
    }

    pub fn from_settings(settings: &AppSettings) -> Self {
        Self {
            cloud_client: CloudWhisperClient::from_settings(settings),
            local_manager: WhisperCppManager::new(),
        }
    }

    pub fn custom(cloud_client: CloudWhisperClient, local_manager: WhisperCppManager) -> Self {
        Self {
            cloud_client,
            local_manager,
        }
    }

    pub fn get_cloud_client(&self) -> &CloudWhisperClient {
        &self.cloud_client
    }

    pub fn get_local_manager(&self) -> &WhisperCppManager {
        &self.local_manager
    }

    pub fn get_active_backend_name(&self) -> &'static str {
        if self.cloud_client.is_available() {
            if self.cloud_client.get_endpoint_url().contains("groq") {
                "CLOUD_GROQ"
            } else {
                "CLOUD_OPENAI"
            }
        } else if self.local_manager.is_available() {
            "LOCAL_WHISPER_CPP"
        } else {
            "INTERPOLATION_FALLBACK"
        }
    }

    pub fn transcribe_clip_with_fallback(
        &self,
        video_path: &Path,
        clip_start_seconds: f64,
        clip_end_seconds: f64,
        fallback_segments: &[TranscriptSegment],
    ) -> Vec<WordTimestamp> {
        let mut temp_wav: Option<PathBuf> = None;

        if self.is_available() && video_path.exists() {
            let temp_dir = PathBuf::from("temp").join("audio");
            let _ = fs::create_dir_all(&temp_dir);
            let wav_path = temp_dir.join(format!("clip_audio_{}.wav", std::time::SystemTime::now().elapsed().unwrap_or_default().as_nanos()));

            match AudioExtractor::extract_wav_16k_mono(video_path, clip_start_seconds, clip_end_seconds, &wav_path) {
                Ok(_) => {
                    temp_wav = Some(wav_path.clone());
                    match self.transcribe(&wav_path) {
                        Ok(words) if !words.is_empty() => {
                            let ok_msg = format!("[WHISPER] Transcrição para clipe concluída com sucesso: {} palavras", words.len());
                            EventBus::global().publish_log(&ok_msg, false);
                            info!("{}", ok_msg);
                            if let Some(ref p) = temp_wav {
                                let _ = fs::remove_file(p);
                            }
                            return words;
                        }
                        Ok(_) => {
                            let warn_msg = "[WHISPER] Whisper retornou lista vazia de palavras. Acionando fallback...";
                            EventBus::global().publish_log(warn_msg, false);
                            warn!("{}", warn_msg);
                        }
                        Err(e) => {
                            let warn_msg = format!("[WHISPER] Falha ao obter transcrição para o clipe: {e}. Acionando fallback gracioso...");
                            EventBus::global().publish_log(&warn_msg, false);
                            warn!("{}", warn_msg);
                        }
                    }
                }
                Err(e) => {
                    let warn_msg = format!("[WHISPER] Falha ao extrair áudio para Whisper: {e}. Acionando fallback gracioso...");
                    EventBus::global().publish_log(&warn_msg, false);
                    warn!("{}", warn_msg);
                }
            }

            if let Some(ref p) = temp_wav {
                let _ = fs::remove_file(p);
            }
        }

        let fallback_msg = format!(
            "[WHISPER] Aplicando fallback gracioso: interpolação algorítmica de timestamps para a janela [{:.2} -> {:.2}]",
            clip_start_seconds, clip_end_seconds
        );
        EventBus::global().publish_log(&fallback_msg, false);
        info!("{}", fallback_msg);
        WordTimestampInterpolator::interpolate_words_for_clip(fallback_segments, clip_start_seconds, clip_end_seconds)
    }
}

impl WhisperTranscriptionService for WhisperService {
    fn is_available(&self) -> bool {
        self.cloud_client.is_available() || self.local_manager.is_available()
    }

    fn transcribe(&self, audio_file_path: &Path) -> Result<Vec<WordTimestamp>, MediaError> {
        if !audio_file_path.exists() {
            return Err(MediaError::InvalidArgument(format!(
                "Arquivo de áudio não encontrado: {}",
                audio_file_path.display()
            )));
        }

        // 1. Tenta transcrição na Nuvem se configurado (Groq / OpenAI)
        if self.cloud_client.is_available() {
            match self.cloud_client.transcribe(audio_file_path) {
                Ok(words) if !words.is_empty() => return Ok(words),
                Ok(_) => {}
                Err(e) => {
                    warn!("Tentativa de transcrição via Cloud Whisper falhou: {}. Tentando próximo backend...", e);
                }
            }
        }

        // 2. Tenta Whisper.cpp local se disponível
        if self.local_manager.is_available() {
            match self.local_manager.transcribe(audio_file_path) {
                Ok(words) if !words.is_empty() => return Ok(words),
                Ok(_) => {}
                Err(e) => {
                    warn!("Tentativa de transcrição via Whisper.cpp local falhou: {}.", e);
                }
            }
        }

        Ok(Vec::new())
    }
}
