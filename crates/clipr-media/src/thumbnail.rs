use clipr_domain::{EventBus, ThumbnailAspectRatio, VideoExportLayout};
use crate::error::MediaError;
use crate::subtitles::SubtitleGenerator;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::{info, warn};

pub const TARGET_WIDTH: u32 = 1280;
pub const TARGET_HEIGHT: u32 = 720;
pub const TARGET_VERTICAL_WIDTH: u32 = 1080;
pub const TARGET_VERTICAL_HEIGHT: u32 = 1920;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThumbnailResult {
    pub final_thumbnail_path: PathBuf,
    pub raw_frame_path: PathBuf,
}

pub struct ThumbnailGenerator {
    ffmpeg_binary: String,
}

impl Default for ThumbnailGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl ThumbnailGenerator {
    pub fn new() -> Self {
        Self {
            ffmpeg_binary: Self::resolve_ffmpeg_binary(),
        }
    }

    pub fn with_binary(ffmpeg_binary: impl Into<String>) -> Self {
        let b = ffmpeg_binary.into();
        Self {
            ffmpeg_binary: if b.trim().is_empty() {
                Self::resolve_ffmpeg_binary()
            } else {
                b
            },
        }
    }

    pub fn get_ffmpeg_binary(&self) -> &str {
        &self.ffmpeg_binary
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

    pub fn build_extract_frame_command(
        &self,
        video_file: &Path,
        timestamp_seconds: f64,
        raw_frame_output_path: &Path,
        aspect_ratio: ThumbnailAspectRatio,
    ) -> Vec<String> {
        let safe_timestamp = timestamp_seconds.max(0.0);

        if aspect_ratio == ThumbnailAspectRatio::Vertical916 {
            let vf = format!(
                "scale={}:{}:force_original_aspect_ratio=increase,crop={}:{}",
                aspect_ratio.width(),
                aspect_ratio.height(),
                aspect_ratio.width(),
                aspect_ratio.height()
            );
            vec![
                self.ffmpeg_binary.clone(),
                "-y".to_string(),
                "-ss".to_string(),
                format!("{:.3}", safe_timestamp),
                "-i".to_string(),
                video_file.to_string_lossy().to_string(),
                "-vf".to_string(),
                vf,
                "-vframes".to_string(),
                "1".to_string(),
                "-q:v".to_string(),
                "2".to_string(),
                raw_frame_output_path.to_string_lossy().to_string(),
            ]
        } else {
            vec![
                self.ffmpeg_binary.clone(),
                "-y".to_string(),
                "-ss".to_string(),
                format!("{:.3}", safe_timestamp),
                "-i".to_string(),
                video_file.to_string_lossy().to_string(),
                "-vframes".to_string(),
                "1".to_string(),
                "-q:v".to_string(),
                "2".to_string(),
                raw_frame_output_path.to_string_lossy().to_string(),
            ]
        }
    }

    pub fn extract_clean_frame(
        &self,
        video_file: &Path,
        timestamp_seconds: f64,
        raw_frame_output_path: &Path,
        aspect_ratio: ThumbnailAspectRatio,
    ) -> Result<(), MediaError> {
        if let Some(parent) = raw_frame_output_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let cmd_args = self.build_extract_frame_command(video_file, timestamp_seconds, raw_frame_output_path, aspect_ratio);
        info!("[THUMBNAIL] Executando extração de frame FFmpeg: {}", cmd_args.join(" "));

        let child = Command::new(&cmd_args[0])
            .args(&cmd_args[1..])
            .output()?;

        if !child.status.success() || !raw_frame_output_path.exists() {
            let stderr_str = String::from_utf8_lossy(&child.stderr).to_string();
            let stdout_str = String::from_utf8_lossy(&child.stdout).to_string();
            let details = if !stderr_str.is_empty() { stderr_str } else { stdout_str };
            return Err(MediaError::FfmpegExecution {
                exit_code: child.status.code().unwrap_or(-1),
                details,
            });
        }

        Ok(())
    }

    pub fn wrap_headline(headline: &str, max_width_chars: usize, max_words_per_line: usize) -> Vec<String> {
        let trimmed = headline.trim();
        if trimmed.is_empty() {
            return Vec::new();
        }

        let words: Vec<&str> = trimmed.split_whitespace().collect();
        if words.is_empty() {
            return Vec::new();
        }

        if words.len() <= max_words_per_line {
            let single_line = words.join(" ");
            if single_line.len() <= max_width_chars {
                return vec![single_line];
            }
        }

        let mut lines = Vec::new();
        let mut current_line = String::new();
        let mut words_in_line = 0;

        for word in words {
            let candidate = if current_line.is_empty() {
                word.to_string()
            } else {
                format!("{current_line} {word}")
            };

            if !current_line.is_empty() && (candidate.len() > max_width_chars || words_in_line >= max_words_per_line) {
                lines.push(current_line);
                current_line = word.to_string();
                words_in_line = 1;
            } else {
                current_line = candidate;
                words_in_line += 1;
            }
        }

        if !current_line.is_empty() {
            lines.push(current_line);
        }

        if lines.len() > 4 {
            let mut capped = lines[..3].to_vec();
            capped.push(format!("{}...", lines[3]));
            return capped;
        }

        lines
    }

    pub fn compose_thumbnail_command(
        &self,
        raw_frame_path: &Path,
        final_thumbnail_path: &Path,
        headline: &str,
        aspect_ratio: ThumbnailAspectRatio,
    ) -> Vec<String> {
        let width = aspect_ratio.width();
        let height = aspect_ratio.height();
        let is_vertical = aspect_ratio == ThumbnailAspectRatio::Vertical916;

        let max_width_chars = if is_vertical { 18 } else { 22 };
        let lines = Self::wrap_headline(headline, max_width_chars, 3);

        let mut vf = format!(
            "scale={width}:{height}:force_original_aspect_ratio=increase,crop={width}:{height}"
        );

        if !lines.is_empty() {
            let fonts_dir = SubtitleGenerator::get_fonts_directory();
            let font_file = fonts_dir
                .as_ref()
                .map(|d| d.join("Montserrat-ExtraBold.otf"))
                .filter(|p| p.is_file());

            let font_clause = if let Some(ref f) = font_file {
                format!(":fontfile='{}'", f.to_string_lossy().replace('\\', "/").replace(':', "\\:"))
            } else {
                String::new()
            };

            let font_size = if is_vertical { 72 } else { 58 };
            let line_height = (font_size as f64 * 1.25) as u32;
            let start_y = if is_vertical {
                (height as f64 * 0.18) as u32
            } else {
                120
            };

            for (i, line) in lines.iter().enumerate() {
                let y = start_y + (i as u32 * line_height);
                let color = if lines.len() == 1 || i % 2 == 1 {
                    "#FFE600" // Alto CTR amarelo vibrante
                } else {
                    "white"
                };

                let safe_text = line
                    .replace('\\', "\\\\")
                    .replace('\'', "\\'")
                    .replace(':', "\\:");

                let x_clause = if is_vertical {
                    "(w-text_w)/2".to_string()
                } else {
                    "70".to_string()
                };

                vf.push_str(&format!(
                    ",drawtext=text='{}'{}:fontsize={}:fontcolor={}:borderw=6:bordercolor=black:shadowx=4:shadowy=4:shadowcolor=black@0.8:x={}:y={}",
                    safe_text, font_clause, font_size, color, x_clause, y
                ));
            }
        }

        vec![
            self.ffmpeg_binary.clone(),
            "-y".to_string(),
            "-i".to_string(),
            raw_frame_path.to_string_lossy().to_string(),
            "-vf".to_string(),
            vf,
            "-vframes".to_string(),
            "1".to_string(),
            "-q:v".to_string(),
            "2".to_string(),
            final_thumbnail_path.to_string_lossy().to_string(),
        ]
    }

    pub fn compose_thumbnail(
        &self,
        raw_frame_path: &Path,
        final_thumbnail_path: &Path,
        headline: &str,
        aspect_ratio: ThumbnailAspectRatio,
    ) -> Result<(), MediaError> {
        let cmd_args = self.compose_thumbnail_command(raw_frame_path, final_thumbnail_path, headline, aspect_ratio);
        info!("[THUMBNAIL] Compondo thumbnail de alto impacto: {}", cmd_args.join(" "));

        let child = Command::new(&cmd_args[0])
            .args(&cmd_args[1..])
            .output()?;

        if !child.status.success() || !final_thumbnail_path.exists() {
            let stderr_str = String::from_utf8_lossy(&child.stderr).to_string();
            let stdout_str = String::from_utf8_lossy(&child.stdout).to_string();
            let details = if !stderr_str.is_empty() { stderr_str } else { stdout_str };
            return Err(MediaError::FfmpegExecution {
                exit_code: child.status.code().unwrap_or(-1),
                details,
            });
        }

        Ok(())
    }

    pub fn generate_thumbnail(
        &self,
        video_file: &Path,
        timestamp_seconds: f64,
        headline: &str,
        output_directory: &Path,
        base_file_name: &str,
        aspect_ratio: ThumbnailAspectRatio,
    ) -> Result<ThumbnailResult, MediaError> {
        if !video_file.exists() {
            return Err(MediaError::InvalidArgument(format!(
                "Arquivo de vídeo não encontrado: {}",
                video_file.display()
            )));
        }

        let mut target_dir = output_directory.to_path_buf();
        if let Some(name) = target_dir.file_name()
            && name != "thumbnails" {
                let candidate = target_dir.join("thumbnails");
                if candidate.is_dir() || target_dir.join("cortes").is_dir() || target_dir.join("dados").is_dir() {
                    target_dir = candidate;
                }
            }
        fs::create_dir_all(&target_dir)?;

        let start_msg = format!(
            "[THUMBNAIL] Iniciando geração da thumbnail | Timestamp: {:.2}s | Proporção: {:?} | Headline: '{}'",
            timestamp_seconds,
            aspect_ratio,
            headline
        );
        EventBus::global().publish_log(&start_msg, false);
        info!("{}", start_msg);

        let suffix = if aspect_ratio == ThumbnailAspectRatio::Vertical916 { "_v916" } else { "" };
        let raw_frame_path = target_dir.join(format!("{base_file_name}{suffix}_raw.jpg"));
        let final_thumbnail_path = target_dir.join(format!("{base_file_name}{suffix}_thumb.jpg"));

        // 1. Extração do frame limpo via FFmpeg
        self.extract_clean_frame(video_file, timestamp_seconds, &raw_frame_path, aspect_ratio)?;

        // 2. Composição com texto magnético de alto CTR
        if let Err(e) = self.compose_thumbnail(&raw_frame_path, &final_thumbnail_path, headline, aspect_ratio) {
            let warn_msg = format!("[THUMBNAIL] Falha ao compor thumbnail com texto ({e}), utilizando raw frame como fallback.");
            EventBus::global().publish_log(&warn_msg, false);
            warn!("{}", warn_msg);
            let _ = fs::copy(&raw_frame_path, &final_thumbnail_path);
        }

        let done_msg = format!(
            "[THUMBNAIL] Thumbnail gerada com sucesso | Frame bruto: {} | Thumbnail final: {}",
            raw_frame_path.display(),
            final_thumbnail_path.display()
        );
        EventBus::global().publish_log(&done_msg, false);
        info!("{}", done_msg);

        Ok(ThumbnailResult {
            final_thumbnail_path,
            raw_frame_path,
        })
    }

    pub fn generate_thumbnail_layout(
        &self,
        video_file: &Path,
        timestamp_seconds: f64,
        headline: &str,
        layout: &VideoExportLayout,
        base_file_name: &str,
        aspect_ratio: ThumbnailAspectRatio,
    ) -> Result<ThumbnailResult, MediaError> {
        layout.ensure_directories_exist()?;
        self.generate_thumbnail(
            video_file,
            timestamp_seconds,
            headline,
            &layout.thumbnails_dir,
            base_file_name,
            aspect_ratio,
        )
    }
}

pub struct ThumbnailService {
    generator: ThumbnailGenerator,
}

impl Default for ThumbnailService {
    fn default() -> Self {
        Self::new()
    }
}

impl ThumbnailService {
    pub fn new() -> Self {
        Self {
            generator: ThumbnailGenerator::new(),
        }
    }

    pub fn with_binary(ffmpeg_binary: impl Into<String>) -> Self {
        Self {
            generator: ThumbnailGenerator::with_binary(ffmpeg_binary),
        }
    }

    pub fn generator(&self) -> &ThumbnailGenerator {
        &self.generator
    }

    pub fn generate_thumbnail(
        &self,
        video_file: &Path,
        timestamp_seconds: f64,
        headline: &str,
        output_directory: &Path,
        base_file_name: &str,
        aspect_ratio: ThumbnailAspectRatio,
    ) -> Result<ThumbnailResult, MediaError> {
        self.generator.generate_thumbnail(
            video_file,
            timestamp_seconds,
            headline,
            output_directory,
            base_file_name,
            aspect_ratio,
        )
    }
}
