pub mod error;
pub mod ffmpeg;
pub mod subtitles;
pub mod thumbnail;
pub mod transcript;
pub mod transcription;
pub mod ytdlp;

pub use error::MediaError;
pub use ffmpeg::{FfmpegExecutor, AUDIO_FADE_DURATION_SECONDS, TEMPORAL_PADDING_SECONDS};
pub use subtitles::{
    InterpolatedWord, SubtitleDialogueEvent, SubtitleGenerator, WordChunk,
    WordTimestampInterpolator,
};
pub use thumbnail::{ThumbnailGenerator, ThumbnailResult, ThumbnailService};
pub use transcript::TranscriptParser;
pub use transcription::{
    AudioExtractor, CloudWhisperClient, WhisperCppManager, WhisperService,
    WhisperTranscriptionService,
};
pub use ytdlp::{SubtitleTrack, TranscriptResult, VideoMetadata, YtDlpManager};
