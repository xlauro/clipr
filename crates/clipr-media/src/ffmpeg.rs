use clipr_domain::{
    ClipCandidate, EventBus, FramingMode, ProcessingStatus,
    TranscriptSegment, VideoExportLayout, VideoSource,
};
use crate::error::MediaError;
use crate::subtitles::SubtitleGenerator;
use chrono::Utc;
use regex::Regex;
use serde_json::json;
use std::collections::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command as SyncCommand, Stdio};
use std::sync::LazyLock;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tracing::{debug, error, info};

pub const TEMPORAL_PADDING_SECONDS: f64 = 0.2;
pub const AUDIO_FADE_DURATION_SECONDS: f64 = 0.05;

static TIME_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"time=(\d+):(\d{1,2}):(\d{1,2}(?:\.\d+)?)").expect("Valid regex for ffmpeg time progress")
});

static FFMPEG_SPEED_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"speed=\s*(\S+)").expect("Valid regex for ffmpeg speed")
});

pub struct FfmpegExecutor {
    ffmpeg_binary: String,
    video_encoder: String,
}

impl Default for FfmpegExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl FfmpegExecutor {
    pub fn new() -> Self {
        let binary = Self::resolve_ffmpeg_binary();
        let encoder = Self::resolve_video_encoder(&binary);
        Self {
            ffmpeg_binary: binary,
            video_encoder: encoder,
        }
    }

    pub fn with_binary(custom_binary: impl Into<String>) -> Self {
        let binary = custom_binary.into();
        let encoder = Self::resolve_video_encoder(&binary);
        Self {
            ffmpeg_binary: binary,
            video_encoder: encoder,
        }
    }

    pub fn with_binary_and_encoder(custom_binary: impl Into<String>, custom_encoder: impl Into<String>) -> Self {
        let binary = custom_binary.into();
        let enc_str = custom_encoder.into();
        let encoder = if enc_str.trim().is_empty() {
            Self::resolve_video_encoder(&binary)
        } else {
            enc_str
        };
        Self {
            ffmpeg_binary: binary,
            video_encoder: encoder,
        }
    }

    pub fn get_ffmpeg_binary(&self) -> &str {
        &self.ffmpeg_binary
    }

    pub fn get_video_encoder(&self) -> &str {
        &self.video_encoder
    }

    pub fn resolve_ffmpeg_binary() -> String {
        if let Ok(path_env) = std::env::var("PATH") {
            for dir in std::env::split_paths(&path_env) {
                let candidate = dir.join("ffmpeg");
                if candidate.is_file() {
                    return candidate.to_string_lossy().to_string();
                }
            }
        }
        if Path::new("/usr/bin/ffmpeg").is_file() {
            return "/usr/bin/ffmpeg".to_string();
        }
        "ffmpeg".to_string()
    }

    pub fn resolve_video_encoder(binary: &str) -> String {
        let output = SyncCommand::new(binary)
            .args(["-hide_banner", "-encoders"])
            .output();

        if let Ok(out) = output {
            let encoders_str = String::from_utf8_lossy(&out.stdout).to_string()
                + &String::from_utf8_lossy(&out.stderr);

            // Hardware Encoders
            if encoders_str.contains("h264_nvenc") {
                info!("[FFMPEG] Encoder de vídeo selecionado: h264_nvenc (NVIDIA)");
                return "h264_nvenc".to_string();
            }
            if encoders_str.contains("h264_vaapi") {
                info!("[FFMPEG] Encoder de vídeo selecionado: h264_vaapi (Linux Intel/AMD)");
                return "h264_vaapi".to_string();
            }
            if encoders_str.contains("h264_qsv") {
                info!("[FFMPEG] Encoder de vídeo selecionado: h264_qsv (Intel QuickSync)");
                return "h264_qsv".to_string();
            }
            if encoders_str.contains("h264_videotoolbox") {
                info!("[FFMPEG] Encoder de vídeo selecionado: h264_videotoolbox (Apple Silicon/macOS)");
                return "h264_videotoolbox".to_string();
            }

            // Software / Multithreaded CPU
            if encoders_str.contains("libx264") {
                info!("[FFMPEG] Encoder de vídeo selecionado: libx264 (CPU Multithreaded)");
                return "libx264".to_string();
            }
            if encoders_str.contains("libopenh264") {
                info!("[FFMPEG] Encoder de vídeo selecionado: libopenh264");
                return "libopenh264".to_string();
            }
        }

        info!("[FFMPEG] Encoder de vídeo selecionado (fallback padrão): h264");
        "h264".to_string()
    }

    pub fn parse_progress_seconds(line: &str) -> Option<f64> {
        let caps = TIME_PATTERN.captures(line)?;
        let hours: f64 = caps[1].parse().ok()?;
        let mins: f64 = caps[2].parse().ok()?;
        let secs: f64 = caps[3].parse().ok()?;
        Some((hours * 3600.0) + (mins * 60.0) + secs)
    }

    pub fn build_video_filter_graph(&self, framing_mode: FramingMode, ass_subtitles: Option<&Path>) -> String {
        let mut filter_graph = match framing_mode {
            FramingMode::AutoCrop => "crop=ih*9/16:ih:(iw-ow)/2:0,scale=1080:1920".to_string(),
            FramingMode::SplitScreen => {
                "split=2[left][right];[left]crop=iw/2:ih:0:0,scale=1080:960[top];[right]crop=iw/2:ih:iw/2:0,scale=1080:960[bottom];[top][bottom]vstack=inputs=2".to_string()
            }
            FramingMode::BlurredBg => {
                "split[v1][v2];[v1]scale=1080:1920:force_original_aspect_ratio=increase,crop=1080:1920,boxblur=20:5[bg];[v2]scale=1080:1920:force_original_aspect_ratio=decrease[fg];[bg][fg]overlay=(W-w)/2:(H-h)/2".to_string()
            }
            FramingMode::Horizontal169 => {
                "scale=1920:1080:force_original_aspect_ratio=decrease,pad=1920:1080:(ow-iw)/2:(oh-ih)/2".to_string()
            }
        };

        if let Some(ass_file) = ass_subtitles
            && ass_file.exists() {
                let escaped_ass = ass_file
                    .to_string_lossy()
                    .replace('\\', "/")
                    .replace(':', "\\:");

                let fonts_dir = SubtitleGenerator::get_fonts_directory();
                if let Some(fonts) = fonts_dir {
                    if fonts.exists() {
                        let escaped_fonts = fonts
                            .to_string_lossy()
                            .replace('\\', "/")
                            .replace(':', "\\:");
                        filter_graph.push_str(&format!(",ass='{escaped_ass}':fontsdir='{escaped_fonts}'"));
                    } else {
                        filter_graph.push_str(&format!(",ass='{escaped_ass}'"));
                    }
                } else {
                    filter_graph.push_str(&format!(",ass='{escaped_ass}'"));
                }
            }

        filter_graph
    }

    pub fn build_audio_filter_graph(&self, duration: f64, optimize_pacing: bool) -> String {
        let mut af = String::new();
        if optimize_pacing {
            af.push_str("silenceremove=stop_periods=-1:stop_duration=0.5:stop_threshold=-40dB,");
        }
        let fade_out_start = (duration - AUDIO_FADE_DURATION_SECONDS).max(0.0);
        af.push_str(&format!(
            "afade=t=in:ss=0:d={:.2},afade=t=out:st={:.3}:d={:.2}",
            AUDIO_FADE_DURATION_SECONDS, fade_out_start, AUDIO_FADE_DURATION_SECONDS
        ));
        af
    }

    #[allow(clippy::too_many_arguments)]
    pub fn build_ffmpeg_command(
        &self,
        source_video: &Path,
        output_file: &Path,
        start_seconds: f64,
        duration: f64,
        framing_mode: FramingMode,
        ass_subtitles: Option<&Path>,
        cpu_threads: u32,
        optimize_pacing: bool,
    ) -> Vec<String> {
        let mut cmd = vec![
            self.ffmpeg_binary.clone(),
            "-y".to_string(),
            "-ss".to_string(),
            format!("{:.3}", start_seconds),
            "-i".to_string(),
            source_video.to_string_lossy().to_string(),
            "-t".to_string(),
            format!("{:.3}", duration),
            "-vf".to_string(),
            self.build_video_filter_graph(framing_mode, ass_subtitles),
            "-af".to_string(),
            self.build_audio_filter_graph(duration, optimize_pacing),
            "-c:v".to_string(),
            self.video_encoder.clone(),
        ];

        match self.video_encoder.as_str() {
            "libx264" => {
                cmd.extend([
                    "-preset".to_string(),
                    "fast".to_string(),
                    "-crf".to_string(),
                    "20".to_string(),
                ]);
            }
            "h264_nvenc" => {
                cmd.extend([
                    "-preset".to_string(),
                    "fast".to_string(),
                    "-b:v".to_string(),
                    "4M".to_string(),
                ]);
            }
            _ => {
                cmd.extend(["-b:v".to_string(), "4M".to_string()]);
            }
        }

        cmd.extend([
            "-pix_fmt".to_string(),
            "yuv420p".to_string(),
            "-c:a".to_string(),
            "aac".to_string(),
            "-b:a".to_string(),
            "192k".to_string(),
            "-threads".to_string(),
            cpu_threads.to_string(),
            output_file.to_string_lossy().to_string(),
        ]);

        cmd
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn render_clip(
        &self,
        source: &VideoSource,
        clip: &ClipCandidate,
        source_video_file: &Path,
        layout: &VideoExportLayout,
        segments: &[TranscriptSegment],
        cpu_threads: u32,
        use_blurred_background: bool,
        optimize_pacing: bool,
    ) -> Result<ClipCandidate, MediaError> {
        layout.ensure_directories_exist()?;

        let start_pad = (clip.start_seconds - TEMPORAL_PADDING_SECONDS).max(0.0);
        let end_pad = clip.end_seconds + TEMPORAL_PADDING_SECONDS;
        let duration_pad = (end_pad - start_pad).max(0.1);

        let safe_title: String = clip
            .title
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
            .collect();
        let short_title = if safe_title.len() > 50 {
            &safe_title[..50]
        } else {
            &safe_title
        };

        let output_file_name = format!("clip_{}_{:03}_{}.mp4", layout.video_id, clip.virality_score, short_title);
        let output_file = layout.resolve_clip(&output_file_name);
        let json_metadata_file = layout.resolve_data(&output_file_name.replace(".mp4", ".json"));

        let framing = if clip.framing_mode == FramingMode::AutoCrop && use_blurred_background {
            FramingMode::BlurredBg
        } else {
            clip.framing_mode
        };
        let orientation = framing.to_orientation_mode();

        let start_msg = format!(
            "[FFMPEG] Iniciando render do corte: '{}' | Encoder: {} | Enquadramento: {:?} | Threads: {} | Respiro temporal: {:.2}s ({:.2}s -> {:.2}s)",
            clip.title,
            self.video_encoder,
            framing,
            cpu_threads,
            TEMPORAL_PADDING_SECONDS,
            start_pad,
            end_pad
        );
        EventBus::global().publish_log(&start_msg, false);
        info!("{}", start_msg);

        let mut ass_file: Option<PathBuf> = None;
        let subtitle_preset = clip.subtitle_preset;

        if clip.has_subtitles && !segments.is_empty() {
            let final_ass_file = output_file
                .parent()
                .unwrap_or(Path::new("."))
                .join(output_file_name.replace(".mp4", ".ass"));
            let final_srt_file = output_file
                .parent()
                .unwrap_or(Path::new("."))
                .join(output_file_name.replace(".mp4", ".srt"));

            let _ = SubtitleGenerator::generate_ass_subtitles(
                segments,
                start_pad,
                end_pad,
                orientation,
                subtitle_preset,
                &final_ass_file,
                clip.has_emojis,
                true,
            );
            let _ = SubtitleGenerator::generate_srt_subtitles(
                segments,
                start_pad,
                end_pad,
                &final_srt_file,
                clip.has_emojis,
            );

            let sub_msg = format!(
                "[FFMPEG] Legendas geradas | ASS: {} | SRT: {} | Emojis: {}",
                final_ass_file.display(),
                final_srt_file.display(),
                clip.has_emojis
            );
            EventBus::global().publish_log(&sub_msg, false);
            info!("{}", sub_msg);

            ass_file = Some(final_ass_file);
        }

        let command_args = self.build_ffmpeg_command(
            source_video_file,
            &output_file,
            start_pad,
            duration_pad,
            framing,
            ass_file.as_deref(),
            cpu_threads,
            optimize_pacing,
        );

        info!("[FFMPEG] Executando comando FFmpeg: {}", command_args.join(" "));

        let mut child = Command::new(&command_args[0])
            .args(&command_args[1..])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let mut recent_lines: VecDeque<String> = VecDeque::with_capacity(50);
        let mut cp25 = false;
        let mut cp50 = false;
        let mut cp75 = false;
        let mut cp100 = false;

        let stderr = child.stderr.take();

        if let Some(err) = stderr {
            let mut reader = BufReader::new(err).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                if recent_lines.len() >= 50 {
                    recent_lines.pop_front();
                }
                recent_lines.push_back(line.clone());

                EventBus::global().publish_log(format!("[FFMPEG] {line}"), true);
                debug!("[FFMPEG] {line}");
                if let Some(curr_sec) = Self::parse_progress_seconds(&line) {
                    let progress = (curr_sec / duration_pad.max(1.0)).min(1.0);
                    EventBus::global().publish_progress("CUTTING_FFMPEG", progress, &line);
                    let pct = progress * 100.0;
                    let speed = FFMPEG_SPEED_PATTERN
                        .captures(&line)
                        .map(|c| c[1].to_string())
                        .unwrap_or_else(|| "1.0x".to_string());

                    if pct >= 25.0 && !cp25 {
                        cp25 = true;
                        let m = format!("[FFMPEG] Progresso do render: 25% | Tempo processado: {:.1}s / {:.1}s | Velocidade: {}", curr_sec, duration_pad, speed);
                        EventBus::global().publish_log(&m, false);
                        info!("{}", m);
                    } else if pct >= 50.0 && !cp50 {
                        cp50 = true;
                        let m = format!("[FFMPEG] Progresso do render: 50% | Tempo processado: {:.1}s / {:.1}s | Velocidade: {}", curr_sec, duration_pad, speed);
                        EventBus::global().publish_log(&m, false);
                        info!("{}", m);
                    } else if pct >= 75.0 && !cp75 {
                        cp75 = true;
                        let m = format!("[FFMPEG] Progresso do render: 75% | Tempo processado: {:.1}s / {:.1}s | Velocidade: {}", curr_sec, duration_pad, speed);
                        EventBus::global().publish_log(&m, false);
                        info!("{}", m);
                    } else if pct >= 99.9 && !cp100 {
                        cp100 = true;
                        let m = format!("[FFMPEG] Progresso do render: 100% | Tempo processado: {:.1}s / {:.1}s | Velocidade: {}", curr_sec, duration_pad, speed);
                        EventBus::global().publish_log(&m, false);
                        info!("{}", m);
                    }
                }
            }
        }

        let status = child.wait().await?;
        if !status.success() {
            let exit_code = status.code().unwrap_or(-1);
            let error_details = recent_lines.into_iter().collect::<Vec<_>>().join("\n");
            let err_msg = format!("[FFMPEG][ERRO] Falha na execução do FFmpeg para o corte '{}' (exit code: {}):\n{}", clip.title, exit_code, error_details);
            error!("{err_msg}");
            EventBus::global().publish_log(&err_msg, false);
            return Err(MediaError::FfmpegExecution {
                exit_code,
                details: error_details,
            });
        }

        // Write companion JSON metadata
        self.write_metadata_json(source, clip, &output_file, &json_metadata_file);

        let mut completed_clip = clip.clone();
        completed_clip.output_file_path = Some(output_file.to_string_lossy().to_string());
        completed_clip.metadata_json_path = Some(json_metadata_file.to_string_lossy().to_string());
        completed_clip.status = ProcessingStatus::Completed;
        completed_clip.orientation = orientation;
        completed_clip.framing_mode = framing;

        let file_size_mb = fs::metadata(&output_file).map(|m| m.len() as f64 / 1_048_576.0).unwrap_or(0.0);
        let done_msg = format!(
            "[FFMPEG] Corte finalizado com sucesso: {} | Tamanho: {:.2} MB",
            output_file.file_name().unwrap_or_default().to_string_lossy(),
            file_size_mb
        );
        EventBus::global().publish_log(&done_msg, false);
        info!("{}", done_msg);
        EventBus::global().publish_clip_render_completed(completed_clip.clone());

        Ok(completed_clip)
    }

    fn write_metadata_json(
        &self,
        source: &VideoSource,
        clip: &ClipCandidate,
        video_file: &Path,
        target_json: &Path,
    ) {
        let meta = json!({
            "title": clip.title,
            "hook": clip.hook,
            "viralityScore": clip.virality_score,
            "startSeconds": clip.start_seconds,
            "endSeconds": clip.end_seconds,
            "durationSeconds": clip.duration_seconds(),
            "suggestedCaption": clip.suggested_caption,
            "reasoning": clip.reasoning,
            "orientation": format!("{:?}", clip.orientation),
            "framingMode": format!("{:?}", clip.framing_mode),
            "subtitlePreset": format!("{:?}", clip.subtitle_preset),
            "sourceUrl": source.url,
            "sourceVideoId": source.video_id,
            "author": source.author,
            "exportedAt": Utc::now().to_rfc3339(),
            "videoFilePath": video_file.to_string_lossy().to_string(),
            "thumbnailTimestamp": clip.thumbnail_timestamp,
            "thumbnailHeadline": clip.thumbnail_headline,
            "thumbnailPath": clip.thumbnail_path,
            "rawFramePath": clip.raw_frame_path,
            "hashtags": clip.hashtags,
        });

        if let Ok(formatted) = serde_json::to_string_pretty(&meta) {
            let _ = fs::write(target_json, formatted);
        }
    }
}
