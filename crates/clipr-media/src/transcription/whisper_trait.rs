use std::path::Path;
use clipr_domain::WordTimestamp;
use crate::error::MediaError;

pub trait WhisperTranscriptionService: Send + Sync {
    fn transcribe(&self, audio_file_path: &Path) -> Result<Vec<WordTimestamp>, MediaError>;
    fn is_available(&self) -> bool;
}
