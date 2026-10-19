use crate::enums::{EmotionTag, FramingMode, OrientationMode, ProcessingStatus, SubtitlePreset};
use crate::scores::ScoreBreakdown;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClipCandidate {
    pub id: Option<i64>,
    pub video_source_id: Option<i64>,
    pub title: String,
    pub hook: String,
    pub start_seconds: f64,
    pub end_seconds: f64,
    pub virality_score: i32,
    pub reasoning: String,
    pub suggested_caption: String,
    pub hashtags: Vec<String>,
    pub orientation: OrientationMode,
    pub has_subtitles: bool,
    pub has_emojis: bool,
    pub output_file_path: Option<String>,
    pub metadata_json_path: Option<String>,
    pub status: ProcessingStatus,
    pub thumbnail_timestamp: Option<f64>,
    pub thumbnail_headline: Option<String>,
    pub thumbnail_path: Option<String>,
    pub raw_frame_path: Option<String>,
    pub framing_mode: FramingMode,
    pub subtitle_preset: SubtitlePreset,
    pub emotion_tag: EmotionTag,
    pub scores: ScoreBreakdown,
}

impl ClipCandidate {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: Option<i64>,
        video_source_id: Option<i64>,
        title: impl Into<String>,
        hook: impl Into<String>,
        start_seconds: f64,
        end_seconds: f64,
        virality_score: i32,
        reasoning: impl Into<String>,
        suggested_caption: impl Into<String>,
        hashtags: Vec<String>,
        orientation: OrientationMode,
        has_subtitles: bool,
        has_emojis: bool,
        output_file_path: Option<String>,
        metadata_json_path: Option<String>,
        status: ProcessingStatus,
        thumbnail_timestamp: Option<f64>,
        thumbnail_headline: Option<String>,
        thumbnail_path: Option<String>,
        raw_frame_path: Option<String>,
        framing_mode: Option<FramingMode>,
        subtitle_preset: Option<SubtitlePreset>,
        emotion_tag: Option<EmotionTag>,
        scores: Option<ScoreBreakdown>,
    ) -> Self {
        let default_framing = if orientation == OrientationMode::Horizontal169 {
            FramingMode::Horizontal169
        } else {
            FramingMode::AutoCrop
        };
        let final_framing = framing_mode.unwrap_or(default_framing);
        let final_preset = subtitle_preset.unwrap_or(SubtitlePreset::HormoziYellow);
        let final_emotion = emotion_tag.unwrap_or(EmotionTag::AltaEnergia);
        let final_scores = scores.unwrap_or_else(|| ScoreBreakdown::from_uniform(virality_score));

        Self {
            id,
            video_source_id,
            title: title.into(),
            hook: hook.into(),
            start_seconds,
            end_seconds,
            virality_score,
            reasoning: reasoning.into(),
            suggested_caption: suggested_caption.into(),
            hashtags,
            orientation,
            has_subtitles,
            has_emojis,
            output_file_path,
            metadata_json_path,
            status,
            thumbnail_timestamp,
            thumbnail_headline,
            thumbnail_path,
            raw_frame_path,
            framing_mode: final_framing,
            subtitle_preset: final_preset,
            emotion_tag: final_emotion,
            scores: final_scores,
        }
    }

    pub fn duration_seconds(&self) -> f64 {
        (self.end_seconds - self.start_seconds).max(0.0)
    }

    pub fn has_valid_bounds(&self) -> bool {
        self.start_seconds >= 0.0 && self.end_seconds > self.start_seconds
    }

    pub fn is_viral_worthy(&self, min_score_threshold: i32) -> bool {
        self.virality_score >= min_score_threshold
    }

    pub fn is_long_clip(&self) -> bool {
        self.duration_seconds() > 90.0
    }

    pub fn is_rendered(&self) -> bool {
        self.status == ProcessingStatus::Completed
            || self
                .output_file_path
                .as_ref()
                .map(|p| !p.trim().is_empty())
                .unwrap_or(false)
    }

    pub fn clip_type_badge(&self) -> &'static str {
        if self.is_long_clip() {
            "🎬 Longo"
        } else {
            "⚡ Curto"
        }
    }

    pub fn formatted_duration(&self) -> String {
        let total_sec = self.duration_seconds().round() as u64;
        let hours = total_sec / 3600;
        let minutes = (total_sec % 3600) / 60;
        let secs = total_sec % 60;

        if hours > 0 {
            format!("{:02}h {:02}m {:02}s", hours, minutes, secs)
        } else if minutes > 0 {
            format!("{:02}m {:02}s", minutes, secs)
        } else {
            format!("{}s", secs)
        }
    }

    pub fn formatted_time_range(&self) -> String {
        let has_hours = self.start_seconds >= 3600.0 || self.end_seconds >= 3600.0;
        format!(
            "{} - {}",
            format_timestamp(self.start_seconds, has_hours),
            format_timestamp(self.end_seconds, has_hours)
        )
    }

    pub fn formatted_display_interval(&self) -> String {
        format!("{} ({})", self.formatted_time_range(), self.formatted_duration())
    }

    pub fn with_bounds(&self, new_start: f64, new_end: f64) -> Self {
        let mut clone = self.clone();
        clone.start_seconds = new_start;
        clone.end_seconds = new_end;
        clone
    }

    pub fn with_thumbnail(
        &self,
        timestamp: Option<f64>,
        headline: Option<String>,
        new_thumbnail_path: Option<String>,
        new_raw_frame_path: Option<String>,
    ) -> Self {
        let mut clone = self.clone();
        clone.thumbnail_timestamp = timestamp;
        clone.thumbnail_headline = headline;
        clone.thumbnail_path = new_thumbnail_path;
        clone.raw_frame_path = new_raw_frame_path;
        clone
    }

    pub fn with_thumbnail_paths(
        &self,
        new_thumbnail_path: Option<String>,
        new_raw_frame_path: Option<String>,
    ) -> Self {
        self.with_thumbnail(
            self.thumbnail_timestamp,
            self.thumbnail_headline.clone(),
            new_thumbnail_path,
            new_raw_frame_path,
        )
    }

    pub fn with_framing_mode(&self, new_mode: Option<FramingMode>) -> Self {
        let target_mode = new_mode.unwrap_or(FramingMode::AutoCrop);
        let mut clone = self.clone();
        clone.orientation = target_mode.to_orientation_mode();
        clone.framing_mode = target_mode;
        clone
    }

    pub fn with_subtitle_preset(&self, new_preset: Option<SubtitlePreset>) -> Self {
        let mut clone = self.clone();
        clone.subtitle_preset = new_preset.unwrap_or(SubtitlePreset::HormoziYellow);
        clone
    }

    pub fn with_emotion_tag(&self, new_tag: Option<EmotionTag>) -> Self {
        let mut clone = self.clone();
        clone.emotion_tag = new_tag.unwrap_or(EmotionTag::AltaEnergia);
        clone
    }

    pub fn with_scores(&self, new_scores: Option<ScoreBreakdown>) -> Self {
        let mut clone = self.clone();
        if let Some(target) = new_scores {
            clone.virality_score = target.calculate_weighted_virality_score();
            clone.scores = target;
        }
        clone
    }

    pub fn with_scores_and_virality(
        &self,
        new_scores: Option<ScoreBreakdown>,
        explicit_virality_score: i32,
    ) -> Self {
        let mut clone = self.clone();
        if let Some(target) = new_scores {
            clone.scores = target;
        }
        clone.virality_score = explicit_virality_score;
        clone
    }

    pub fn with_emojis(&self, new_has_emojis: bool) -> Self {
        let mut clone = self.clone();
        clone.has_emojis = new_has_emojis;
        clone
    }

    pub fn has_thumbnail(&self) -> bool {
        self.thumbnail_path
            .as_ref()
            .map(|p| !p.trim().is_empty())
            .unwrap_or(false)
    }
}

fn format_timestamp(seconds: f64, has_hours: bool) -> String {
    let total_sec = seconds.max(0.0).floor() as u64;
    let hours = total_sec / 3600;
    let minutes = (total_sec % 3600) / 60;
    let secs = total_sec % 60;

    if has_hours || hours > 0 {
        format!("{:02}:{:02}:{:02}", hours, minutes, secs)
    } else {
        format!("{:02}:{:02}", minutes, secs)
    }
}
