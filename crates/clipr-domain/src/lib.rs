pub mod clip;
pub mod emoji;
pub mod enums;
pub mod error;
pub mod events;
pub mod layout;
pub mod scores;
pub mod settings;
pub mod timestamp;
pub mod transcript;
pub mod video_source;

pub use clip::ClipCandidate;
pub use emoji::EmojiDictionary;
pub use enums::{
    AiProvider, EmotionTag, FramingMode, OrientationMode, ProcessingStatus, SubtitlePreset,
    ThumbnailAspectRatio,
};
pub use error::DomainError;
pub use events::{ClipprEvent, EventBus};
pub use layout::{
    VideoExportLayout, MAX_TITLE_LENGTH, SUBDIR_CORTES, SUBDIR_DADOS, SUBDIR_THUMBNAILS,
};
pub use scores::ScoreBreakdown;
pub use settings::AppSettings;
pub use timestamp::WordTimestamp;
pub use transcript::TranscriptSegment;
pub use video_source::VideoSource;

pub mod prelude {
    pub use super::{
        AiProvider, AppSettings, ClipCandidate, ClipprEvent, DomainError, EmojiDictionary,
        EmotionTag, EventBus, FramingMode, OrientationMode, ProcessingStatus, ScoreBreakdown,
        SubtitlePreset, ThumbnailAspectRatio, TranscriptSegment, VideoExportLayout, VideoSource,
        WordTimestamp, MAX_TITLE_LENGTH, SUBDIR_CORTES, SUBDIR_DADOS, SUBDIR_THUMBNAILS,
    };
}

