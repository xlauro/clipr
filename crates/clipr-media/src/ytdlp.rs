use clipr_domain::{EventBus, TranscriptSegment};
use crate::error::MediaError;
use crate::transcript::TranscriptParser;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::LazyLock;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tracing::{debug, error, info, warn};

static PROGRESS_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\[download\]\s+(\d+(?:\.\d+)?)%").expect("Valid regex for yt-dlp download progress")
});

static SPEED_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"at\s+([~0-9.]+[kKMGT]?i?B/s)").expect("Valid speed pattern")
});

const GITHUB_YTDLP_URL: &str = "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VideoMetadata {
    pub id: String,
    pub title: String,
    pub author: String,
    pub duration_seconds: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TranscriptResult {
    pub metadata: VideoMetadata,
    pub segments: Vec<TranscriptSegment>,
    pub formatted_transcript: String,
    pub vtt_file: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubtitleTrack {
    pub language: String,
    pub url: String,
    pub format: String,
}

pub struct YtDlpManager {
    binary_path: PathBuf,
}

impl Default for YtDlpManager {
    fn default() -> Self {
        Self::new()
    }
}

impl YtDlpManager {
    pub fn new() -> Self {
        Self {
            binary_path: Self::resolve_or_download_binary(),
        }
    }

    pub fn with_custom_binary(custom_binary_path: impl Into<PathBuf>) -> Self {
        Self {
            binary_path: custom_binary_path.into(),
        }
    }

    pub fn get_binary_path(&self) -> &Path {
        &self.binary_path
    }

    pub async fn version(&self) -> Result<String, MediaError> {
        let output = Command::new(&self.binary_path)
            .arg("--version")
            .output()
            .await?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            Err(MediaError::YtDlpExecution {
                exit_code: output.status.code().unwrap_or(-1),
                details: String::from_utf8_lossy(&output.stderr).to_string(),
            })
        }
    }

    pub fn resolve_or_download_binary() -> PathBuf {
        // 1. Verifica no PATH do sistema
        if let Ok(path_env) = std::env::var("PATH") {
            for dir in std::env::split_paths(&path_env) {
                let candidate = dir.join("yt-dlp");
                if candidate.is_file() {
                    info!("[YTDLP] yt-dlp encontrado no PATH: {}", candidate.display());
                    return candidate;
                }
            }
        }

        // 2. Verifica na pasta bin/ local
        let local_bin = PathBuf::from("bin").join("yt-dlp");
        if local_bin.is_file() {
            info!("[YTDLP] yt-dlp encontrado na pasta bin/: {}", local_bin.display());
            return local_bin;
        }

        // 3. Verifica em ~/.clippr/bin/yt-dlp
        let home_dir = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let user_clippr_bin = PathBuf::from(home_dir).join(".clippr").join("bin").join("yt-dlp");
        if user_clippr_bin.is_file() {
            info!("[YTDLP] yt-dlp encontrado em ~/.clippr/bin/: {}", user_clippr_bin.display());
            return user_clippr_bin;
        }

        // 4. Se não existir, faz download automático
        Self::download_standalone_binary(&user_clippr_bin)
    }

    pub fn download_standalone_binary(target_path: &Path) -> PathBuf {
        info!("[YTDLP] Baixando binário oficial do yt-dlp para {}", target_path.display());
        EventBus::global().publish_log("[YTDLP] Baixando ferramenta yt-dlp para processamento de vídeos...", false);

        if let Some(parent) = target_path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        let resp = reqwest::blocking::get(GITHUB_YTDLP_URL);
        match resp {
            Ok(response) if response.status().is_success() => {
                if let Ok(bytes) = response.bytes()
                    && let Ok(()) = fs::write(target_path, bytes) {
                        #[cfg(unix)]
                        {
                            use std::os::unix::fs::PermissionsExt;
                            let _ = fs::set_permissions(target_path, fs::Permissions::from_mode(0o755));
                        }
                        info!("[YTDLP] yt-dlp baixado com sucesso e marcado como executável.");
                        EventBus::global().publish_log("[YTDLP] yt-dlp instalado com sucesso.", false);
                        return target_path.to_path_buf();
                    }
            }
            Ok(response) => {
                error!("[YTDLP] Falha ao baixar yt-dlp: HTTP {}", response.status());
            }
            Err(e) => {
                error!("[YTDLP] Erro ao baixar binário yt-dlp: {}", e);
            }
        }

        target_path.to_path_buf()
    }

    pub fn resolve_js_runtime() -> Option<String> {
        // 1. Verifica no PATH
        if let Ok(path_env) = std::env::var("PATH") {
            for dir in std::env::split_paths(&path_env) {
                let candidate = dir.join("node");
                if candidate.is_file() {
                    return Some(candidate.to_string_lossy().to_string());
                }
            }
        }

        // 2. Procura em diretórios NVM comuns
        if let Some(home) = dirs::home_dir() {
            let nvm_pattern = home.join(".nvm/versions/node");
            if nvm_pattern.is_dir() {
                if let Ok(entries) = std::fs::read_dir(&nvm_pattern) {
                    for entry in entries.flatten() {
                        let node = entry.path().join("bin/node");
                        if node.is_file() {
                            return Some(node.to_string_lossy().to_string());
                        }
                    }
                }
            }
        }

        // 3. Verifica caminhos padrão do sistema
        for p in &["/usr/bin/node", "/usr/local/bin/node", "/opt/homebrew/bin/node"] {
            if Path::new(p).is_file() {
                return Some((*p).to_string());
            }
        }

        None
    }

    pub fn resolve_ffmpeg() -> Option<String> {
        if let Ok(path_env) = std::env::var("PATH") {
            for dir in std::env::split_paths(&path_env) {
                let candidate = dir.join("ffmpeg");
                if candidate.is_file() {
                    return Some(candidate.to_string_lossy().to_string());
                }
            }
        }
        if Path::new("/usr/bin/ffmpeg").is_file() {
            return Some("/usr/bin/ffmpeg".to_string());
        }
        None
    }

    pub async fn fetch_metadata_and_transcript(&self, url: &str) -> Result<TranscriptResult, MediaError> {
        info!("[YTDLP] Extraindo metadados e transcrição do YouTube: {}", url);
        EventBus::global().publish_log(format!("[YTDLP] Extraindo transcrição e timestamps do vídeo: {url}"), false);

        let temp_dir = PathBuf::from("temp").join("transcripts");
        let _ = fs::create_dir_all(&temp_dir);

        let mut cmd = Command::new(&self.binary_path);
        if let Some(ref js_runtime) = Self::resolve_js_runtime() {
            cmd.arg("--js-runtimes").arg(format!("node:{js_runtime}"));
        }
        cmd.arg("--dump-single-json")
            .arg("--no-playlist")
            .arg(url)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let output = cmd.output().await?;

        if !output.status.success() {
            let stderr_str = String::from_utf8_lossy(&output.stderr).to_string();
            error!("[YTDLP] Falha no yt-dlp ao processar URL: {}", stderr_str);
            return Err(MediaError::YtDlpExecution {
                exit_code: output.status.code().unwrap_or(-1),
                details: stderr_str,
            });
        }

        let json_str = String::from_utf8_lossy(&output.stdout);
        let root: serde_json::Value = serde_json::from_str(&json_str)?;

        let id = root
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let title = root
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Vídeo sem título")
            .to_string();

        let author = root
            .get("uploader")
            .or_else(|| root.get("channel"))
            .and_then(|v| v.as_str())
            .unwrap_or("Canal Desconhecido")
            .to_string();

        let duration_seconds = root
            .get("duration")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let metadata = VideoMetadata {
            id: id.clone(),
            title,
            author,
            duration_seconds,
        };

        let best_track = Self::find_best_subtitle_track(&root);
        let track_desc = best_track
            .as_ref()
            .map(|t| format!("{} ({})", t.language, t.format))
            .unwrap_or_else(|| "Nenhuma detectada".to_string());
        let meta_log = format!(
            "[YTDLP] Metadados extraídos: Título: '{}' | Autor: '{}' | Duração: {:.1}s | Faixa de legendas: {}",
            metadata.title, metadata.author, metadata.duration_seconds, track_desc
        );
        EventBus::global().publish_log(&meta_log, false);
        info!("{}", meta_log);

        let mut found_vtt: Option<PathBuf> = None;

        // 1. Download direto da legenda via HTTP
        if let Some(ref track) = best_track
            && !track.url.trim().is_empty() {
                info!("[YTDLP] Baixando legenda '{}' (formato {}) de {}", track.language, track.format, track.url);
                EventBus::global().publish_log(format!("[YTDLP] Baixando legenda [{}] diretamente...", track.language), false);

                let client = reqwest::Client::builder()
                    .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
                    .timeout(std::time::Duration::from_secs(15))
                    .build();

                if let Ok(c) = client
                    && let Ok(res) = c.get(&track.url).send().await
                        && res.status().is_success()
                            && let Ok(body) = res.text().await
                                && !body.trim().is_empty() {
                                    let dest_vtt = temp_dir.join(format!("{}.{}.vtt", id, track.language));
                                    if fs::write(&dest_vtt, body).is_ok() {
                                        found_vtt = Some(dest_vtt);
                                    }
                                }
            }

        // 2. Fallback: verifica se algum arquivo .vtt já existe no diretório temporário
        if found_vtt.is_none()
            && let Ok(entries) = fs::read_dir(&temp_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(file_name) = path.file_name().and_then(|n| n.to_str())
                        && file_name.starts_with(&id) && file_name.ends_with(".vtt") {
                            found_vtt = Some(path);
                            break;
                        }
                }
            }

        // 3. Fallback: tenta yt-dlp com seleção de idiomas
        if found_vtt.is_none() {
            info!("[YTDLP] Tentando baixar legenda via yt-dlp fallback...");
            let output_template = temp_dir.join("%(id)s.%(ext)s").to_string_lossy().to_string();

            let sub_res = tokio::time::timeout(
                std::time::Duration::from_secs(45),
                Command::new(&self.binary_path)
                    .arg("--no-playlist")
                    .arg("--skip-download")
                    .arg("--write-auto-sub")
                    .arg("--write-sub")
                    .arg("--sub-lang")
                    .arg("pt-orig,pt,en")
                    .arg("--sub-format")
                    .arg("vtt")
                    .arg("--output")
                    .arg(&output_template)
                    .arg(url)
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .output(),
            )
            .await;

            if let Ok(Ok(_)) = sub_res
                && let Ok(entries) = fs::read_dir(&temp_dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if let Some(file_name) = path.file_name().and_then(|n| n.to_str())
                            && file_name.starts_with(&id) && file_name.ends_with(".vtt") {
                                found_vtt = Some(path);
                                break;
                            }
                    }
                }
        }

        let segments;
        let formatted_transcript;

        if let Some(ref vtt_path) = found_vtt {
            if let Ok(vtt_content) = fs::read_to_string(vtt_path) {
                segments = TranscriptParser::parse_vtt(&vtt_content);
                formatted_transcript = TranscriptParser::format_for_ai(&segments);
                if !segments.is_empty() {
                    info!("[YTDLP] Transcrição extraída com sucesso: {} segmentos encontrados.", segments.len());
                    EventBus::global().publish_log(
                        format!("[YTDLP] Transcrição obtida com sucesso: {} trechos identificados.", segments.len()),
                        false,
                    );
                } else {
                    warn!("[YTDLP] Arquivo de legenda foi encontrado, mas nenhum segmento válido foi extraído: {}", vtt_path.display());
                    EventBus::global().publish_log("[YTDLP] Aviso: O arquivo de legendas não continha texto reconhecível.", false);
                }
            } else {
                segments = Vec::new();
                formatted_transcript = String::new();
            }
        } else {
            warn!("[YTDLP] Nenhuma legenda encontrada para o vídeo {}.", id);
            segments = Vec::new();
            formatted_transcript = String::new();
            EventBus::global().publish_log("[YTDLP] Aviso: Nenhuma legenda automática ou manual disponível para este vídeo.", false);
        }

        Ok(TranscriptResult {
            metadata,
            segments,
            formatted_transcript,
            vtt_file: found_vtt,
        })
    }

    pub fn find_best_subtitle_track(root: &serde_json::Value) -> Option<SubtitleTrack> {
        let preferred_langs = [
            "pt-orig", "pt-BR", "pt", "pt-PT",
            "en-orig", "en", "en-US", "en-GB",
        ];

        let manual_subs = root.get("subtitles");
        let auto_subs = root.get("automatic_captions");

        // 1. Legendas manuais com idiomas preferenciais
        if let Some(manual) = manual_subs {
            for lang in &preferred_langs {
                if let Some(track) = Self::extract_track(manual, lang) {
                    return Some(track);
                }
            }
        }

        // 2. Legendas automáticas com idiomas preferenciais
        if let Some(auto) = auto_subs {
            for lang in &preferred_langs {
                if let Some(track) = Self::extract_track(auto, lang) {
                    return Some(track);
                }
            }
        }

        // 3. Qualquer legenda manual disponível
        if let Some(manual) = manual_subs
            && let Some(track) = Self::extract_any_track(manual) {
                return Some(track);
            }

        // 4. Qualquer legenda automática disponível
        if let Some(auto) = auto_subs
            && let Some(track) = Self::extract_any_track(auto) {
                return Some(track);
            }

        None
    }

    fn extract_track(subs_node: &serde_json::Value, lang: &str) -> Option<SubtitleTrack> {
        let formats = subs_node.get(lang)?.as_array()?;

        // Prioridade 1: VTT
        for fmt in formats {
            let ext = fmt.get("ext").and_then(|v| v.as_str()).unwrap_or("");
            let url = fmt.get("url").and_then(|v| v.as_str()).unwrap_or("").trim();
            if ext.eq_ignore_ascii_case("vtt") && !url.is_empty() {
                return Some(SubtitleTrack {
                    language: lang.to_string(),
                    url: url.to_string(),
                    format: "vtt".to_string(),
                });
            }
        }

        // Prioridade 2: SRT
        for fmt in formats {
            let ext = fmt.get("ext").and_then(|v| v.as_str()).unwrap_or("");
            let url = fmt.get("url").and_then(|v| v.as_str()).unwrap_or("").trim();
            if ext.eq_ignore_ascii_case("srt") && !url.is_empty() {
                return Some(SubtitleTrack {
                    language: lang.to_string(),
                    url: url.to_string(),
                    format: "srt".to_string(),
                });
            }
        }

        None
    }

    fn extract_any_track(subs_node: &serde_json::Value) -> Option<SubtitleTrack> {
        let obj = subs_node.as_object()?;
        for lang in obj.keys() {
            if let Some(track) = Self::extract_track(subs_node, lang) {
                return Some(track);
            }
        }
        None
    }

    pub async fn download_full_video<F>(
        &self,
        url: &str,
        destination_file: &Path,
        progress_consumer: Option<F>,
    ) -> Result<PathBuf, MediaError>
    where
        F: Fn(f64) + Send + Sync + 'static,
    {
        let start_msg = format!("[YTDLP] Iniciando download do vídeo original: {} -> {}", url, destination_file.display());
        EventBus::global().publish_log(&start_msg, false);
        info!("{}", start_msg);

        if let Some(parent) = destination_file.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut cmd = Command::new(&self.binary_path);
        if let Some(ref js_runtime) = Self::resolve_js_runtime() {
            cmd.arg("--js-runtimes").arg(format!("node:{js_runtime}"));
        }
        if let Some(ref ffmpeg) = Self::resolve_ffmpeg() {
            cmd.arg("--ffmpeg-location").arg(ffmpeg);
        }
        cmd.arg("-S")
            .arg("vcodec:h264,res,acodec:m4a")
            .arg("-f")
            .arg("bestvideo[ext=mp4]+bestaudio[ext=m4a]/best[ext=mp4]/best")
            .arg("--output")
            .arg(destination_file.to_string_lossy().to_string())
            .arg("--newline")
            .arg(url)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = cmd.spawn()?;

        let mut cp25 = false;
        let mut cp50 = false;
        let mut cp75 = false;
        let mut cp100 = false;

        let stderr = child.stderr.take();
        let stderr_task = tokio::spawn(async move {
            let mut err_lines = Vec::new();
            if let Some(err) = stderr {
                let mut reader = BufReader::new(err).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    EventBus::global().publish_log(format!("[YTDLP-ERR] {line}"), true);
                    debug!("[YTDLP-ERR] {line}");
                    err_lines.push(line);
                }
            }
            err_lines.join("\n")
        });

        let stdout = child.stdout.take();
        if let Some(out) = stdout {
            let mut reader = BufReader::new(out).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                EventBus::global().publish_log(format!("[YTDLP] {line}"), true);
                debug!("[YTDLP] {line}");
                if let Some(caps) = PROGRESS_PATTERN.captures(&line)
                    && let Ok(pct) = caps[1].parse::<f64>() {
                        let normalized = pct / 100.0;
                        if let Some(ref cb) = progress_consumer {
                            cb(normalized);
                        }
                        EventBus::global().publish_progress("DOWNLOAD_VIDEO", normalized, &line);

                        let speed = SPEED_PATTERN
                            .captures(&line)
                            .map(|c| c[1].to_string())
                            .unwrap_or_else(|| "N/A".to_string());

                        if pct >= 25.0 && !cp25 {
                            cp25 = true;
                            let cp_msg = format!("[YTDLP] Progresso do download: 25% | Velocidade: {speed}");
                            EventBus::global().publish_log(&cp_msg, false);
                            info!("{}", cp_msg);
                        } else if pct >= 50.0 && !cp50 {
                            cp50 = true;
                            let cp_msg = format!("[YTDLP] Progresso do download: 50% | Velocidade: {speed}");
                            EventBus::global().publish_log(&cp_msg, false);
                            info!("{}", cp_msg);
                        } else if pct >= 75.0 && !cp75 {
                            cp75 = true;
                            let cp_msg = format!("[YTDLP] Progresso do download: 75% | Velocidade: {speed}");
                            EventBus::global().publish_log(&cp_msg, false);
                            info!("{}", cp_msg);
                        } else if pct >= 99.9 && !cp100 {
                            cp100 = true;
                            let cp_msg = format!("[YTDLP] Progresso do download: 100% | Velocidade: {speed}");
                            EventBus::global().publish_log(&cp_msg, false);
                            info!("{}", cp_msg);
                        }
                    }
            }
        }

        let status = child.wait().await?;
        let stderr_output = stderr_task.await.unwrap_or_default();
        if !status.success() {
            let err_msg = format!("[YTDLP] Erro ao baixar vídeo via yt-dlp (exit code: {}): {stderr_output}", status);
            error!("{err_msg}");
            EventBus::global().publish_log(&err_msg, false);
            return Err(MediaError::YtDlpExecution {
                exit_code: status.code().unwrap_or(-1),
                details: err_msg,
            });
        }

        let file_size_mb = fs::metadata(destination_file).map(|m| m.len() as f64 / 1_048_576.0).unwrap_or(0.0);
        let done_msg = format!(
            "[YTDLP] Download do vídeo concluído com sucesso: {} | Tamanho final: {:.2} MB",
            destination_file.file_name().unwrap_or_default().to_string_lossy(),
            file_size_mb
        );
        EventBus::global().publish_log(&done_msg, false);
        info!("{}", done_msg);

        Ok(destination_file.to_path_buf())
    }
}
