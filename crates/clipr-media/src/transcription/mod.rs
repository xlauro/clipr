pub mod audio_extractor;
pub mod cloud_whisper;
pub mod service;
pub mod whisper_cpp;
pub mod whisper_trait;

pub use audio_extractor::AudioExtractor;
pub use cloud_whisper::CloudWhisperClient;
pub use service::WhisperService;
pub use whisper_cpp::WhisperCppManager;
pub use whisper_trait::WhisperTranscriptionService;
