use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TranscriptSegment {
    pub start_seconds: f64,
    pub end_seconds: f64,
    pub text: String,
}

impl TranscriptSegment {
    pub fn new(start_seconds: f64, end_seconds: f64, text: impl Into<String>) -> Self {
        Self {
            start_seconds,
            end_seconds,
            text: text.into(),
        }
    }

    pub fn duration(&self) -> f64 {
        (self.end_seconds - self.start_seconds).max(0.0)
    }

    pub fn formatted_time_range(&self) -> String {
        format!(
            "[{} -> {}]",
            Self::format_seconds(self.start_seconds),
            Self::format_seconds(self.end_seconds)
        )
    }

    pub fn format_seconds(seconds: f64) -> String {
        let total_sec = seconds.max(0.0) as u64;
        let hours = total_sec / 3600;
        let minutes = (total_sec % 3600) / 60;
        let secs = total_sec % 60;
        let millis = ((seconds.max(0.0) - total_sec as f64) * 1000.0).round() as u64;
        format!("{hours:02}:{minutes:02}:{secs:02}.{millis:03}")
    }
}
