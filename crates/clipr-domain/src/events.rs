use crate::clip::ClipCandidate;
use crate::video_source::VideoSource;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::LazyLock;
use tokio::sync::broadcast::{self, Receiver, Sender};

static GLOBAL_EVENT_BUS: LazyLock<EventBus> = LazyLock::new(|| EventBus::new(2048));

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum ClipprEvent {
    LogMessage {
        timestamp: DateTime<Utc>,
        message: String,
        is_debug: bool,
    },
    ProgressUpdated {
        timestamp: DateTime<Utc>,
        stage: String,
        progress: f64,
        details: String,
    },
    VideoSourceUpdated {
        timestamp: DateTime<Utc>,
        video_source: VideoSource,
    },
    ClipsEvaluated {
        timestamp: DateTime<Utc>,
        video_source_id: Option<i64>,
        approved_clips: Vec<ClipCandidate>,
        discarded_count: usize,
    },
    ClipRenderCompleted {
        timestamp: DateTime<Utc>,
        clip: ClipCandidate,
    },
    ThumbnailGenerated {
        timestamp: DateTime<Utc>,
        clip: ClipCandidate,
        thumbnail_path: PathBuf,
        raw_frame_path: PathBuf,
    },
    PipelineError {
        timestamp: DateTime<Utc>,
        stage: String,
        error_message: String,
    },
}

impl ClipprEvent {
    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            Self::LogMessage { timestamp, .. } => *timestamp,
            Self::ProgressUpdated { timestamp, .. } => *timestamp,
            Self::VideoSourceUpdated { timestamp, .. } => *timestamp,
            Self::ClipsEvaluated { timestamp, .. } => *timestamp,
            Self::ClipRenderCompleted { timestamp, .. } => *timestamp,
            Self::ThumbnailGenerated { timestamp, .. } => *timestamp,
            Self::PipelineError { timestamp, .. } => *timestamp,
        }
    }

    pub fn log(message: impl Into<String>, is_debug: bool) -> Self {
        Self::LogMessage {
            timestamp: Utc::now(),
            message: message.into(),
            is_debug,
        }
    }

    pub fn progress(stage: impl Into<String>, progress: f64, details: impl Into<String>) -> Self {
        Self::ProgressUpdated {
            timestamp: Utc::now(),
            stage: stage.into(),
            progress,
            details: details.into(),
        }
    }

    pub fn progress_pipeline(progress: f64, details: impl Into<String>) -> Self {
        Self::progress("PIPELINE", progress, details)
    }

    pub fn video_source_updated(video_source: VideoSource) -> Self {
        Self::VideoSourceUpdated {
            timestamp: Utc::now(),
            video_source,
        }
    }

    pub fn video_updated(video_source: VideoSource) -> Self {
        Self::video_source_updated(video_source)
    }

    pub fn clips_evaluated(
        video_source_id: Option<i64>,
        approved_clips: Vec<ClipCandidate>,
        discarded_count: usize,
    ) -> Self {
        Self::ClipsEvaluated {
            timestamp: Utc::now(),
            video_source_id,
            approved_clips,
            discarded_count,
        }
    }

    pub fn clip_render_completed(clip: ClipCandidate) -> Self {
        Self::ClipRenderCompleted {
            timestamp: Utc::now(),
            clip,
        }
    }

    pub fn clip_rendered(clip: ClipCandidate) -> Self {
        Self::clip_render_completed(clip)
    }

    pub fn thumbnail_generated(
        clip: ClipCandidate,
        thumbnail_path: PathBuf,
        raw_frame_path: PathBuf,
    ) -> Self {
        Self::ThumbnailGenerated {
            timestamp: Utc::now(),
            clip,
            thumbnail_path,
            raw_frame_path,
        }
    }

    pub fn pipeline_error(stage: impl Into<String>, error_message: impl Into<String>) -> Self {
        Self::PipelineError {
            timestamp: Utc::now(),
            stage: stage.into(),
            error_message: error_message.into(),
        }
    }
}

/// Barramento thread-safe com Tokio broadcast channel para distribuição de eventos assíncronos.
#[derive(Clone, Debug)]
pub struct EventBus {
    sender: Sender<ClipprEvent>,
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new(2048)
    }
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    pub fn global() -> &'static EventBus {
        &GLOBAL_EVENT_BUS
    }

    pub fn subscribe(&self) -> Receiver<ClipprEvent> {
        self.sender.subscribe()
    }

    #[allow(clippy::result_large_err)]
    pub fn publish(
        &self,
        event: ClipprEvent,
    ) -> Result<usize, broadcast::error::SendError<ClipprEvent>> {
        self.sender.send(event)
    }

    pub fn publish_log(&self, message: impl Into<String>, is_debug: bool) {
        let _ = self.publish(ClipprEvent::log(message, is_debug));
    }

    pub fn publish_progress(
        &self,
        stage: impl Into<String>,
        progress: f64,
        details: impl Into<String>,
    ) {
        let _ = self.publish(ClipprEvent::progress(stage, progress, details));
    }

    pub fn publish_video_source(&self, source: VideoSource) {
        let _ = self.publish(ClipprEvent::video_source_updated(source));
    }

    pub fn publish_clips_evaluated(
        &self,
        video_source_id: Option<i64>,
        approved_clips: Vec<ClipCandidate>,
        discarded_count: usize,
    ) {
        let _ = self.publish(ClipprEvent::clips_evaluated(
            video_source_id,
            approved_clips,
            discarded_count,
        ));
    }

    pub fn publish_clip_render_completed(&self, clip: ClipCandidate) {
        let _ = self.publish(ClipprEvent::clip_render_completed(clip));
    }

    pub fn publish_thumbnail_generated(
        &self,
        clip: ClipCandidate,
        thumbnail_path: PathBuf,
        raw_frame_path: PathBuf,
    ) {
        let _ = self.publish(ClipprEvent::thumbnail_generated(
            clip,
            thumbnail_path,
            raw_frame_path,
        ));
    }

    pub fn publish_error(&self, stage: impl Into<String>, error_message: impl Into<String>) {
        let _ = self.publish(ClipprEvent::pipeline_error(stage, error_message));
    }

    pub fn receiver_count(&self) -> usize {
        self.sender.receiver_count()
    }
}
