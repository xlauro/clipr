use crate::error::MediaError;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::info;

pub struct AudioExtractor;

impl AudioExtractor {
    pub fn extract_wav_16k_mono(
        video_path: &Path,
        start_seconds: f64,
        end_seconds: f64,
        output_wav_path: &Path,
    ) -> Result<PathBuf, MediaError> {
        let binary = Self::resolve_ffmpeg_binary();
        Self::extract_wav_16k_mono_with_binary(&binary, video_path, start_seconds, end_seconds, output_wav_path)
    }

    pub fn extract_wav_16k_mono_full(video_path: &Path, output_wav_path: &Path) -> Result<PathBuf, MediaError> {
        Self::extract_wav_16k_mono(video_path, -1.0, -1.0, output_wav_path)
    }

    pub fn extract_wav_16k_mono_with_binary(
        ffmpeg_binary: &str,
        video_path: &Path,
        start_seconds: f64,
        end_seconds: f64,
        output_wav_path: &Path,
    ) -> Result<PathBuf, MediaError> {
        if !video_path.exists() {
            return Err(MediaError::InvalidArgument(format!(
                "Arquivo de vídeo não encontrado: {}",
                video_path.display()
            )));
        }

        if let Some(parent) = output_wav_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut cmd = Command::new(ffmpeg_binary);
        cmd.arg("-y");

        let has_interval = start_seconds >= 0.0 && end_seconds > start_seconds;
        if has_interval {
            cmd.arg("-ss");
            cmd.arg(format!("{:.3}", start_seconds.max(0.0)));
            cmd.arg("-to");
            cmd.arg(format!("{:.3}", end_seconds));
        }

        cmd.arg("-i");
        cmd.arg(video_path);
        cmd.arg("-vn");
        cmd.arg("-ar");
        cmd.arg("16000");
        cmd.arg("-ac");
        cmd.arg("1");
        cmd.arg("-c:a");
        cmd.arg("pcm_s16le");
        cmd.arg(output_wav_path);

        info!("Executando extração de áudio FFmpeg para Whisper: {:?}", cmd);

        let output = cmd.output()?;
        if !output.status.success() {
            let stderr_str = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(MediaError::FfmpegExecution {
                exit_code: output.status.code().unwrap_or(-1),
                details: stderr_str,
            });
        }

        if !output_wav_path.exists() || fs::metadata(output_wav_path)?.len() == 0 {
            return Err(MediaError::General(format!(
                "O arquivo de áudio WAV gerado está vazio ou ausente: {}",
                output_wav_path.display()
            )));
        }

        info!(
            "Áudio extraído com sucesso para Whisper: {} (tamanho: {} bytes)",
            output_wav_path.display(),
            fs::metadata(output_wav_path)?.len()
        );

        Ok(output_wav_path.to_path_buf())
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
}
