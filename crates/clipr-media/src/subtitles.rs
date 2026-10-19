use clipr_domain::{EmojiDictionary, OrientationMode, SubtitlePreset, TranscriptSegment, WordTimestamp};
use crate::error::MediaError;
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};

pub const SPEECH_PAUSE_THRESHOLD_SECONDS: f64 = 0.6;

static PUNCTUATION_END_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r".*[.!?,;:]$").expect("Valid regex for punctuation end")
});

static STRONG_BREAK_PUNCTUATION: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r".*[.!?]$").expect("Valid regex for strong break punctuation")
});

static ALPHANUMERIC_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"[^\p{L}\p{N}]").expect("Valid regex for non-alphanumeric")
});

#[derive(Debug, Clone, PartialEq)]
pub struct InterpolatedWord {
    pub word: String,
    pub start_seconds: f64,
    pub end_seconds: f64,
}

impl InterpolatedWord {
    pub fn new(word: impl Into<String>, start_seconds: f64, end_seconds: f64) -> Self {
        Self {
            word: word.into(),
            start_seconds,
            end_seconds,
        }
    }

    pub fn duration(&self) -> f64 {
        (self.end_seconds - self.start_seconds).max(0.0)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WordChunk {
    pub words: Vec<InterpolatedWord>,
    pub start_seconds: f64,
    pub end_seconds: f64,
}

impl WordChunk {
    pub fn new(words: Vec<InterpolatedWord>, start_seconds: f64, end_seconds: f64) -> Self {
        Self {
            words,
            start_seconds,
            end_seconds,
        }
    }

    pub fn duration(&self) -> f64 {
        (self.end_seconds - self.start_seconds).max(0.0)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SubtitleDialogueEvent {
    pub start_seconds: f64,
    pub end_seconds: f64,
    pub formatted_ass_text: String,
}

impl SubtitleDialogueEvent {
    pub fn new(start_seconds: f64, end_seconds: f64, formatted_ass_text: impl Into<String>) -> Self {
        Self {
            start_seconds,
            end_seconds,
            formatted_ass_text: formatted_ass_text.into(),
        }
    }
}

pub struct WordTimestampInterpolator;

impl WordTimestampInterpolator {
    pub fn interpolate_words(segment: &TranscriptSegment) -> Vec<InterpolatedWord> {
        let trimmed = segment.text.trim();
        if trimmed.is_empty() {
            return Vec::new();
        }

        let tokens: Vec<String> = trimmed
            .split_whitespace()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        if tokens.is_empty() {
            return Vec::new();
        }

        let seg_start = segment.start_seconds;
        let seg_end = segment.end_seconds;
        let total_duration = (0.05f64 * tokens.len() as f64).max(seg_end - seg_start);

        let mut weights = Vec::with_capacity(tokens.len());
        let mut total_weight = 0.0;

        for token in &tokens {
            let clean = ALPHANUMERIC_PATTERN.replace_all(token, "");
            let base_len = if clean.is_empty() { token.len() } else { clean.len() };
            let mut base_weight = (base_len as f64).max(1.0);

            if PUNCTUATION_END_PATTERN.is_match(token) {
                base_weight += if STRONG_BREAK_PUNCTUATION.is_match(token) {
                    2.5
                } else {
                    1.5
                };
            }

            weights.push(base_weight);
            total_weight += base_weight;
        }

        let mut result = Vec::with_capacity(tokens.len());
        let mut current_start = seg_start;

        for (i, token) in tokens.into_iter().enumerate() {
            let word_duration = (weights[i] / total_weight) * total_duration;
            let current_end = if i == weights.len() - 1 {
                seg_end
            } else {
                current_start + word_duration
            };

            result.push(InterpolatedWord::new(token, current_start, current_end));
            current_start = current_end;
        }

        result
    }

    pub fn interpolate_words_all(segments: &[TranscriptSegment]) -> Vec<InterpolatedWord> {
        let mut all = Vec::new();
        for seg in segments {
            all.extend(Self::interpolate_words(seg));
        }
        all
    }

    pub fn group_into_chunks(words: &[InterpolatedWord], min_words: usize, max_words: usize) -> Vec<WordChunk> {
        if words.is_empty() {
            return Vec::new();
        }

        let min = min_words.max(1);
        let max = max_words.max(min);

        // 1. Divide em frases com base em pausas naturais de fala
        let mut phrases: Vec<Vec<InterpolatedWord>> = Vec::new();
        let mut current_phrase: Vec<InterpolatedWord> = Vec::new();

        for word in words {
            if let Some(prev) = current_phrase.last() {
                let gap = word.start_seconds - prev.end_seconds;
                if gap > SPEECH_PAUSE_THRESHOLD_SECONDS {
                    phrases.push(std::mem::take(&mut current_phrase));
                }
            }
            current_phrase.push(word.clone());
        }
        if !current_phrase.is_empty() {
            phrases.push(current_phrase);
        }

        // 2. Para cada frase contínua, particiona dinamicamente em blocos de [min, max] palavras
        let mut result = Vec::new();

        for phrase in phrases {
            let n = phrase.len();
            let mut index = 0;

            while index < n {
                let rem = n - index;

                if rem <= max {
                    let sub = &phrase[index..n];
                    result.push(WordChunk::new(
                        sub.to_vec(),
                        sub.first().unwrap().start_seconds,
                        sub.last().unwrap().end_seconds,
                    ));
                    break;
                }

                let mut best_candidate: Option<usize> = None;

                for candidate in (min..=max).rev() {
                    let remaining_after = rem - candidate;
                    if remaining_after >= min {
                        let last_word_of_chunk = &phrase[index + candidate - 1];
                        if PUNCTUATION_END_PATTERN.is_match(&last_word_of_chunk.word) {
                            best_candidate = Some(candidate);
                            break;
                        }
                    }
                }

                let chosen_k = if let Some(best) = best_candidate {
                    best
                } else {
                    let target = if max >= 3 && min <= 3 { 3 } else { max };
                    if rem.saturating_sub(target) >= min {
                        target
                    } else if rem.saturating_sub(min) >= min {
                        min
                    } else {
                        min.max(rem / 2)
                    }
                };

                let sub = &phrase[index..index + chosen_k];
                result.push(WordChunk::new(
                    sub.to_vec(),
                    sub.first().unwrap().start_seconds,
                    sub.last().unwrap().end_seconds,
                ));
                index += chosen_k;
            }
        }

        result
    }

    pub fn group_timestamps_into_chunks(words: &[WordTimestamp], min_words: usize, max_words: usize) -> Vec<WordChunk> {
        let converted: Vec<InterpolatedWord> = words
            .iter()
            .map(|w| InterpolatedWord::new(&w.word, w.start_seconds, w.end_seconds))
            .collect();
        Self::group_into_chunks(&converted, min_words, max_words)
    }

    pub fn generate_karaoke_events(
        chunks: &[WordChunk],
        preset: SubtitlePreset,
        has_emojis: bool,
        animated_pop: bool,
    ) -> Vec<SubtitleDialogueEvent> {
        if chunks.is_empty() {
            return Vec::new();
        }

        let highlight_color = preset.highlight_color();
        let primary_color = preset.primary_color();
        let primary_tag = format!(r"{{\c{primary_color}&}}");
        let highlight_tag = format!(r"{{\c{highlight_color}&}}");

        let mut events = Vec::new();

        for chunk in chunks {
            let chunk_words = &chunk.words;
            if chunk_words.is_empty() {
                continue;
            }

            let count = chunk_words.len();

            for i in 0..count {
                let current_word = &chunk_words[i];
                let event_start = current_word.start_seconds;
                let mut event_end = if i == count - 1 {
                    current_word.end_seconds
                } else {
                    chunk_words[i + 1].start_seconds
                };

                if event_end <= event_start {
                    event_end = event_start + 0.05;
                }

                let mut text_builder = String::new();

                for (j, item) in chunk_words.iter().enumerate() {
                    if j > 0 {
                        text_builder.push(' ');
                    }
                    let raw_word = &item.word;
                    let word_text = raw_word.to_uppercase();

                    let mut emoji_suffix = String::new();
                    if has_emojis
                        && let Some(emoji) = EmojiDictionary::find_emoji(raw_word) {
                            emoji_suffix = format!(" {emoji}");
                        }

                    if j == i {
                        if animated_pop {
                            text_builder.push_str(&format!(
                                r"{{\t(0,80,\fscx115\fscy115)\t(80,160,\fscx100\fscy100)\c{highlight_color}&}}{word_text}{emoji_suffix}{primary_tag}"
                            ));
                        } else {
                            text_builder.push_str(&format!(
                                "{highlight_tag}{word_text}{emoji_suffix}{primary_tag}"
                            ));
                        }
                    } else {
                        text_builder.push_str(&format!("{word_text}{emoji_suffix}"));
                    }
                }

                events.push(SubtitleDialogueEvent::new(event_start, event_end, text_builder));
            }
        }

        events
    }

    pub fn generate_events_from_timestamps(
        words: &[WordTimestamp],
        preset: SubtitlePreset,
        has_emojis: bool,
        animated_pop: bool,
    ) -> Vec<SubtitleDialogueEvent> {
        let chunks = Self::group_timestamps_into_chunks(words, 2, 3);
        Self::generate_karaoke_events(&chunks, preset, has_emojis, animated_pop)
    }

    pub fn interpolate_and_generate_events(
        all_segments: &[TranscriptSegment],
        clip_start_seconds: f64,
        clip_end_seconds: f64,
        preset: SubtitlePreset,
        has_emojis: bool,
        animated_pop: bool,
    ) -> Vec<SubtitleDialogueEvent> {
        if all_segments.is_empty() {
            return Vec::new();
        }

        let mut relative_words = Vec::new();

        for seg in all_segments {
            if seg.end_seconds < clip_start_seconds || seg.start_seconds > clip_end_seconds {
                continue;
            }

            let words = Self::interpolate_words(seg);
            for w in words {
                if w.end_seconds <= clip_start_seconds || w.start_seconds >= clip_end_seconds {
                    continue;
                }
                let rel_start = (w.start_seconds - clip_start_seconds).max(0.0);
                let rel_end = (w.end_seconds - clip_start_seconds).min(clip_end_seconds - clip_start_seconds);
                if rel_end > rel_start {
                    relative_words.push(InterpolatedWord::new(w.word, rel_start, rel_end));
                }
            }
        }

        if relative_words.is_empty() {
            return Vec::new();
        }

        let chunks = Self::group_into_chunks(&relative_words, 2, 3);
        Self::generate_karaoke_events(&chunks, preset, has_emojis, animated_pop)
    }

    pub fn interpolate_words_for_clip(
        all_segments: &[TranscriptSegment],
        clip_start_seconds: f64,
        clip_end_seconds: f64,
    ) -> Vec<WordTimestamp> {
        if all_segments.is_empty() {
            return Vec::new();
        }

        let mut relative_words = Vec::new();

        for seg in all_segments {
            if seg.end_seconds < clip_start_seconds || seg.start_seconds > clip_end_seconds {
                continue;
            }

            let words = Self::interpolate_words(seg);
            for w in words {
                if w.end_seconds <= clip_start_seconds || w.start_seconds >= clip_end_seconds {
                    continue;
                }
                let rel_start = (w.start_seconds - clip_start_seconds).max(0.0);
                let rel_end = (w.end_seconds - clip_start_seconds).min(clip_end_seconds - clip_start_seconds);
                if rel_end > rel_start {
                    relative_words.push(WordTimestamp::new(w.word, rel_start, rel_end));
                }
            }
        }

        relative_words
    }
}

static CACHED_FONTS_DIR: LazyLock<Mutex<Option<PathBuf>>> = LazyLock::new(|| Mutex::new(None));

pub struct SubtitleGenerator;

impl SubtitleGenerator {
    #[allow(clippy::too_many_arguments)]
    pub fn generate_ass_subtitles(
        all_segments: &[TranscriptSegment],
        clip_start_seconds: f64,
        clip_end_seconds: f64,
        orientation: OrientationMode,
        preset: SubtitlePreset,
        target_ass_file: &Path,
        has_emojis: bool,
        animated_pop: bool,
    ) -> Result<PathBuf, MediaError> {
        let header = Self::build_ass_header(orientation, preset);
        let mut ass = header;

        let events = WordTimestampInterpolator::interpolate_and_generate_events(
            all_segments,
            clip_start_seconds,
            clip_end_seconds,
            preset,
            has_emojis,
            animated_pop,
        );

        for event in events {
            let start_ass = Self::format_ass_time(event.start_seconds);
            let end_ass = Self::format_ass_time(event.end_seconds);
            ass.push_str(&format!(
                "Dialogue: 0,{},{},Viral,,0,0,0,,{}\n",
                start_ass, end_ass, event.formatted_ass_text
            ));
        }

        if let Some(parent) = target_ass_file.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(target_ass_file, ass)?;
        Ok(target_ass_file.to_path_buf())
    }

    pub fn generate_ass_subtitles_from_words(
        words: &[WordTimestamp],
        orientation: OrientationMode,
        preset: SubtitlePreset,
        target_ass_file: &Path,
        has_emojis: bool,
        animated_pop: bool,
    ) -> Result<PathBuf, MediaError> {
        let header = Self::build_ass_header(orientation, preset);
        let mut ass = header;

        let events = WordTimestampInterpolator::generate_events_from_timestamps(
            words,
            preset,
            has_emojis,
            animated_pop,
        );

        for event in events {
            let start_ass = Self::format_ass_time(event.start_seconds);
            let end_ass = Self::format_ass_time(event.end_seconds);
            ass.push_str(&format!(
                "Dialogue: 0,{},{},Viral,,0,0,0,,{}\n",
                start_ass, end_ass, event.formatted_ass_text
            ));
        }

        if let Some(parent) = target_ass_file.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(target_ass_file, ass)?;
        Ok(target_ass_file.to_path_buf())
    }

    pub fn generate_srt_subtitles(
        all_segments: &[TranscriptSegment],
        clip_start_seconds: f64,
        clip_end_seconds: f64,
        target_srt_file: &Path,
        has_emojis: bool,
    ) -> Result<PathBuf, MediaError> {
        let mut relative_words = Vec::new();
        for seg in all_segments {
            if seg.end_seconds < clip_start_seconds || seg.start_seconds > clip_end_seconds {
                continue;
            }
            let words = WordTimestampInterpolator::interpolate_words(seg);
            for w in words {
                if w.end_seconds <= clip_start_seconds || w.start_seconds >= clip_end_seconds {
                    continue;
                }
                let rel_start = (w.start_seconds - clip_start_seconds).max(0.0);
                let rel_end = (w.end_seconds - clip_start_seconds).min(clip_end_seconds - clip_start_seconds);
                if rel_end > rel_start {
                    relative_words.push(InterpolatedWord::new(w.word, rel_start, rel_end));
                }
            }
        }

        let chunks = WordTimestampInterpolator::group_into_chunks(&relative_words, 2, 3);
        Self::write_srt_from_chunks(&chunks, target_srt_file, has_emojis)
    }

    pub fn generate_srt_subtitles_from_words(
        words: &[WordTimestamp],
        target_srt_file: &Path,
        has_emojis: bool,
    ) -> Result<PathBuf, MediaError> {
        let chunks = WordTimestampInterpolator::group_timestamps_into_chunks(words, 2, 3);
        Self::write_srt_from_chunks(&chunks, target_srt_file, has_emojis)
    }

    fn write_srt_from_chunks(
        chunks: &[WordChunk],
        target_srt_file: &Path,
        has_emojis: bool,
    ) -> Result<PathBuf, MediaError> {
        let mut srt = String::new();
        let mut index = 1;

        for chunk in chunks {
            if chunk.words.is_empty() {
                continue;
            }

            let start_sec = chunk.start_seconds;
            let mut end_sec = chunk.end_seconds;
            if end_sec <= start_sec {
                end_sec = start_sec + 0.5;
            }

            let mut text_parts = Vec::new();
            for w in &chunk.words {
                let mut word_str = w.word.to_uppercase();
                if has_emojis
                    && let Some(emoji) = EmojiDictionary::find_emoji(&w.word) {
                        word_str = format!("{word_str} {emoji}");
                    }
                text_parts.push(word_str);
            }

            let text = text_parts.join(" ");

            srt.push_str(&format!("{index}\n"));
            srt.push_str(&format!(
                "{} --> {}\n",
                Self::format_srt_time(start_sec),
                Self::format_srt_time(end_sec)
            ));
            srt.push_str(&format!("{text}\n\n"));
            index += 1;
        }

        if let Some(parent) = target_srt_file.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(target_srt_file, srt)?;
        Ok(target_srt_file.to_path_buf())
    }

    pub fn build_ass_header(mode: OrientationMode, preset: SubtitlePreset) -> String {
        let is_horiz = mode == OrientationMode::Horizontal169;
        let play_res_x = if is_horiz { 1920 } else { 1080 };
        let play_res_y = if is_horiz { 1080 } else { 1920 };
        let font_size = preset.get_font_size(mode);
        let margin_v = preset.get_margin_v(mode);

        format!(
            "[Script Info]\nScriptType: v4.00+\nPlayResX: {}\nPlayResY: {}\nWrapStyle: 0\nScaledBorderAndShadow: yes\n\n[V4+ Styles]\nFormat: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding\nStyle: Viral,{},{},{},&H000000FF,{},{},-1,0,0,0,100,100,0,0,1,{},{},{},40,40,{},1\n\n[Events]\nFormat: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\n",
            play_res_x,
            play_res_y,
            preset.font_name(),
            font_size,
            preset.primary_color(),
            preset.outline_color(),
            preset.shadow_color(),
            preset.outline_width(),
            preset.shadow_depth(),
            preset.alignment(),
            margin_v
        )
    }

    pub fn format_ass_time(seconds: f64) -> String {
        let clamped = seconds.max(0.0);
        let mut total_sec = clamped.floor() as u64;
        let mut centis = ((clamped - total_sec as f64) * 100.0).round() as u64;

        if centis >= 100 {
            total_sec += 1;
            centis -= 100;
        }

        let mut hours = total_sec / 3600;
        let mut minutes = (total_sec % 3600) / 60;
        let mut secs = total_sec % 60;

        if secs >= 60 {
            minutes += 1;
            secs -= 60;
        }
        if minutes >= 60 {
            hours += 1;
            minutes -= 60;
        }

        format!("{}:{:02}:{:02}.{:02}", hours, minutes, secs, centis)
    }

    pub fn format_srt_time(seconds: f64) -> String {
        let clamped = seconds.max(0.0);
        let mut total_sec = clamped.floor() as u64;
        let mut millis = ((clamped - total_sec as f64) * 1000.0).round() as u64;

        if millis >= 1000 {
            total_sec += 1;
            millis -= 1000;
        }

        let mut hours = total_sec / 3600;
        let mut minutes = (total_sec % 3600) / 60;
        let mut secs = total_sec % 60;

        if secs >= 60 {
            minutes += 1;
            secs -= 60;
        }
        if minutes >= 60 {
            hours += 1;
            minutes -= 60;
        }

        format!("{:02}:{:02}:{:02},{:03}", hours, minutes, secs, millis)
    }

    pub fn get_fonts_directory() -> Option<PathBuf> {
        let mut cache = CACHED_FONTS_DIR.lock().unwrap();
        if let Some(ref dir) = *cache
            && dir.is_dir() {
                return Some(dir.clone());
            }

        // Try extracting embedded font
        let bundled_font_bytes = include_bytes!("../../../resources/fonts/Montserrat-ExtraBold.otf");
        let temp_fonts_dir = std::env::temp_dir().join("clippr_fonts");
        if fs::create_dir_all(&temp_fonts_dir).is_ok() {
            let target_font = temp_fonts_dir.join("Montserrat-ExtraBold.otf");
            if !target_font.exists() || fs::metadata(&target_font).map(|m| m.len() == 0).unwrap_or(true) {
                let _ = fs::write(&target_font, bundled_font_bytes);
            }
            if target_font.is_file() {
                *cache = Some(temp_fonts_dir.clone());
                return Some(temp_fonts_dir);
            }
        }

        // Fallback to resources/fonts
        let dev_fonts = Path::new("resources/fonts");
        if dev_fonts.is_dir() {
            let abs = dev_fonts.canonicalize().unwrap_or_else(|_| dev_fonts.to_path_buf());
            *cache = Some(abs.clone());
            return Some(abs);
        }

        None
    }
}
