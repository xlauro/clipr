use clipr_domain::TranscriptSegment;
use regex::Regex;
use std::sync::LazyLock;

static TIMESTAMP_LINE_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(\d{2}:\d{2}:\d{2}[.,]\d{3}|\d{2}:\d{2}[.,]\d{3})\s*-->\s*(\d{2}:\d{2}:\d{2}[.,]\d{3}|\d{2}:\d{2}[.,]\d{3})")
        .expect("Valid regex for timestamps")
});

static TAG_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"<[^>]+>").expect("Valid regex for HTML/VTT tags")
});

static DIGIT_LINE_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\d+$").expect("Valid regex for line numbers")
});

static MULTI_SPACE_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\s+").expect("Valid regex for spaces")
});

pub struct TranscriptParser;

impl TranscriptParser {
    pub fn parse_vtt(vtt_content: &str) -> Vec<TranscriptSegment> {
        let trimmed = vtt_content.trim();
        if trimmed.is_empty() {
            return Vec::new();
        }

        let mut segments = Vec::new();
        let mut current_start: Option<f64> = None;
        let mut current_end: Option<f64> = None;
        let mut current_text = String::new();

        for raw_line in vtt_content.lines() {
            let line = raw_line.trim();
            if line.is_empty()
                || line.starts_with("WEBVTT")
                || line.starts_with("NOTE")
                || line.starts_with("Kind:")
                || line.starts_with("Language:")
            {
                continue;
            }

            if let Some(caps) = TIMESTAMP_LINE_PATTERN.captures(line) {
                if let Some(start) = current_start
                    && !current_text.is_empty() {
                        let clean = Self::clean_text(&current_text);
                        if !clean.is_empty() {
                            let end = current_end.unwrap_or(start);
                            segments.push(TranscriptSegment::new(start, end, clean));
                        }
                        current_text.clear();
                    }

                current_start = Some(Self::parse_timestamp_to_seconds(&caps[1]));
                current_end = Some(Self::parse_timestamp_to_seconds(&caps[2]));
            } else if current_start.is_some() && !DIGIT_LINE_PATTERN.is_match(line) {
                if !current_text.is_empty() {
                    current_text.push(' ');
                }
                current_text.push_str(line);
            }
        }

        if let Some(start) = current_start
            && !current_text.is_empty() {
                let clean = Self::clean_text(&current_text);
                if !clean.is_empty() {
                    let end = current_end.unwrap_or(start);
                    segments.push(TranscriptSegment::new(start, end, clean));
                }
            }

        Self::deduplicate_and_merge(segments)
    }

    pub fn parse_json3(json_content: &str) -> Vec<TranscriptSegment> {
        let Ok(root) = serde_json::from_str::<serde_json::Value>(json_content) else {
            return Vec::new();
        };

        let Some(events) = root.get("events").and_then(|e| e.as_array()) else {
            return Vec::new();
        };

        let mut segments = Vec::new();

        for event in events {
            let start_ms = event.get("tStartMs").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let duration_ms = event.get("dDurationMs").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let start_sec = start_ms / 1000.0;
            let end_sec = (start_ms + duration_ms) / 1000.0;

            let mut seg_text = String::new();
            if let Some(segs) = event.get("segs").and_then(|s| s.as_array()) {
                for s in segs {
                    if let Some(utf8) = s.get("utf8").and_then(|u| u.as_str()) {
                        seg_text.push_str(utf8);
                    }
                }
            }

            let clean = Self::clean_text(&seg_text);
            if !clean.is_empty() {
                segments.push(TranscriptSegment::new(start_sec, end_sec, clean));
            }
        }

        Self::deduplicate_and_merge(segments)
    }

    pub fn format_for_ai(segments: &[TranscriptSegment]) -> String {
        let mut sb = String::new();
        for seg in segments {
            sb.push_str(&seg.formatted_time_range());
            sb.push(' ');
            sb.push_str(&seg.text);
            sb.push('\n');
        }
        sb
    }

    pub fn extract_plain_text(segments: &[TranscriptSegment]) -> String {
        let mut lines = Vec::new();
        for seg in segments {
            let t = seg.text.trim();
            if !t.is_empty() {
                lines.push(t);
            }
        }
        lines.join("\n")
    }

    pub fn extract_plain_text_from_vtt(vtt_content: &str) -> String {
        let segments = Self::parse_vtt(vtt_content);
        Self::extract_plain_text(&segments)
    }

    pub fn clean_text(input: &str) -> String {
        let no_tags = TAG_PATTERN.replace_all(input, "");
        let decoded = no_tags
            .replace("&nbsp;", " ")
            .replace("&amp;", "&")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&#39;", "'")
            .replace("&apos;", "'");

        MULTI_SPACE_PATTERN.replace_all(&decoded, " ").trim().to_string()
    }

    fn parse_timestamp_to_seconds(timestamp: &str) -> f64 {
        let normalized = timestamp.replace(',', ".");
        let parts: Vec<&str> = normalized.split(':').collect();
        if parts.len() == 3 {
            let hours: f64 = parts[0].parse().unwrap_or(0.0);
            let minutes: f64 = parts[1].parse().unwrap_or(0.0);
            let seconds: f64 = parts[2].parse().unwrap_or(0.0);
            (hours * 3600.0) + (minutes * 60.0) + seconds
        } else if parts.len() == 2 {
            let minutes: f64 = parts[0].parse().unwrap_or(0.0);
            let seconds: f64 = parts[1].parse().unwrap_or(0.0);
            (minutes * 60.0) + seconds
        } else {
            0.0
        }
    }

    fn deduplicate_and_merge(mut raw_segments: Vec<TranscriptSegment>) -> Vec<TranscriptSegment> {
        if raw_segments.is_empty() {
            return Vec::new();
        }

        raw_segments.sort_by(|a, b| {
            a.start_seconds
                .partial_cmp(&b.start_seconds)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| {
                    a.end_seconds
                        .partial_cmp(&b.end_seconds)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
        });

        let mut result = Vec::new();
        let mut previous: Option<TranscriptSegment> = None;

        for current in raw_segments {
            match previous {
                None => {
                    previous = Some(current);
                }
                Some(prev) => {
                    let prev_lower = prev.text.to_lowercase();
                    let cur_lower = current.text.to_lowercase();

                    if prev_lower == cur_lower {
                        previous = Some(TranscriptSegment::new(
                            prev.start_seconds,
                            prev.end_seconds.max(current.end_seconds),
                            prev.text,
                        ));
                    } else if cur_lower.starts_with(&prev_lower) && current.text.len() > prev.text.len() {
                        previous = Some(TranscriptSegment::new(
                            prev.start_seconds,
                            prev.end_seconds.max(current.end_seconds),
                            current.text,
                        ));
                    } else {
                        result.push(prev);
                        previous = Some(current);
                    }
                }
            }
        }

        if let Some(prev) = previous {
            result.push(prev);
        }

        result
    }
}
