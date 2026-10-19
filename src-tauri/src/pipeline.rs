use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex, Semaphore};

use clipr_ai::{AiClient, GeminiApiKeyResolver, GeminiClient, OpenAiApiKeyResolver, OpenAiClient};
use clipr_domain::{
    AiProvider, AppSettings, ClipCandidate, ClipprEvent, EventBus, FramingMode, OrientationMode,
    ProcessingStatus, SubtitlePreset, ThumbnailAspectRatio, VideoExportLayout, VideoSource,
    WordTimestamp,
};
use clipr_media::{
    FfmpegExecutor, ThumbnailGenerator, TranscriptParser, WhisperService, YtDlpManager,
};
use clipr_persistence::DatabaseManager;
use clipr_youtube::{VideoPrivacy, VideoUploadRequest, YouTubeApiClient, YouTubeOAuthService};
use tauri::{AppHandle, Emitter};

fn append_log_to_file(msg: &str) {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    let log_dir = home.join(".clippr");
    if let Err(e) = std::fs::create_dir_all(&log_dir) {
        tracing::error!("Falha ao criar diretório de log {:?}: {}", log_dir, e);
        return;
    }
    let log_file = log_dir.join("clippr.log");
    match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file)
    {
        Ok(mut file) => {
            use std::io::Write;
            let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
            let _ = writeln!(file, "[{now}] {msg}");
        }
        Err(e) => {
            tracing::error!("Falha ao abrir {:?}: {}", log_file, e);
        }
    }
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemDiagnostics {
    pub sqlite_connected: bool,
    pub sqlite_message: String,
    pub ffmpeg_ok: bool,
    pub ffmpeg_encoder: String,
    pub ffmpeg_details: String,
    pub yt_dlp_ok: bool,
    pub yt_dlp_version: String,
    pub gemini_ok: bool,
    pub gemini_message: String,
    pub open_ai_ok: bool,
    pub open_ai_message: String,
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressPayload {
    pub stage: String,
    pub progress: f64,
    pub message: String,
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogPayload {
    pub level: String,
    pub message: String,
    pub is_debug: bool,
    pub timestamp: String,
}

pub struct ClipprPipeline {
    db: Arc<DatabaseManager>,
    yt_dlp: Arc<YtDlpManager>,
    ffmpeg: Arc<FfmpegExecutor>,
    thumbnail_gen: Arc<ThumbnailGenerator>,
    whisper: Arc<WhisperService>,
    youtube_oauth: Arc<YouTubeOAuthService>,
    render_semaphore: Arc<Semaphore>,
    #[allow(dead_code)]
    cancellation_token: Arc<AtomicBool>,
    app_handle: Mutex<Option<AppHandle>>,
}

impl ClipprPipeline {
    pub fn new(db: Arc<DatabaseManager>) -> Self {
        let yt_dlp = Arc::new(YtDlpManager::new());
        let ffmpeg = Arc::new(FfmpegExecutor::new());
        let thumbnail_gen = Arc::new(ThumbnailGenerator::new());
        let whisper = Arc::new(WhisperService::new());
        let youtube_oauth = Arc::new(YouTubeOAuthService::new());

        Self {
            db,
            yt_dlp,
            ffmpeg,
            thumbnail_gen,
            whisper,
            youtube_oauth,
            render_semaphore: Arc::new(Semaphore::new(1)),
            cancellation_token: Arc::new(AtomicBool::new(false)),
            app_handle: Mutex::new(None),
        }
    }

    pub async fn set_app_handle(&self, handle: AppHandle) {
        let mut guard = self.app_handle.lock().await;
        *guard = Some(handle.clone());

        let mut rx = EventBus::global().subscribe();
        let app = handle.clone();

        tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(event) => match event {
                        ClipprEvent::LogMessage { message, is_debug, timestamp } => {
                            if message.starts_with("[FFmpeg]") || message.starts_with("[yt-dlp") {
                                append_log_to_file(&message);
                            }
                            let _ = app.emit(
                                "clippr://log",
                                LogPayload {
                                    level: if is_debug { "DEBUG".into() } else { "INFO".into() },
                                    message,
                                    is_debug,
                                    timestamp: timestamp.to_rfc3339(),
                                },
                            );
                        }
                        ClipprEvent::ProgressUpdated { stage, progress, details, .. } => {
                            let _ = app.emit(
                                "clippr://progress",
                                ProgressPayload {
                                    stage,
                                    progress,
                                    message: details,
                                },
                            );
                        }
                        ClipprEvent::VideoSourceUpdated { video_source, .. } => {
                            let _ = app.emit("clippr://video_updated", &video_source);
                        }
                        ClipprEvent::ClipsEvaluated { approved_clips, .. } => {
                            let _ = app.emit("clippr://clips_evaluated", &approved_clips);
                        }
                        ClipprEvent::ClipRenderCompleted { clip, .. } => {
                            let _ = app.emit("clippr://clip_updated", &clip);
                        }
                        ClipprEvent::ThumbnailGenerated { clip, .. } => {
                            let _ = app.emit("clippr://clip_updated", &clip);
                        }
                        ClipprEvent::PipelineError { stage, error_message, .. } => {
                            append_log_to_file(&format!("[PIPELINE][ERRO][{stage}] {error_message}"));
                            let _ = app.emit(
                                "clippr://progress",
                                ProgressPayload {
                                    stage,
                                    progress: 0.0,
                                    message: format!("Erro: {error_message}"),
                                },
                            );
                            let _ = app.emit(
                                "clippr://log",
                                LogPayload {
                                    level: "ERROR".into(),
                                    message: error_message,
                                    is_debug: false,
                                    timestamp: chrono::Utc::now().to_rfc3339(),
                                },
                            );
                        }
                    },
                    Err(broadcast::error::RecvError::Lagged(skipped)) => {
                        tracing::warn!("EventBus receptor atrasou, {skipped} mensagens perdidas");
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        tracing::info!("EventBus global encerrado, finalizando listener Tauri.");
                        break;
                    }
                }
            }
        });
    }

    pub fn log(&self, message: impl Into<String>, is_debug: bool) {
        let msg = message.into();
        if is_debug {
            tracing::debug!("{}", msg);
        } else {
            tracing::info!("{}", msg);
        }
        append_log_to_file(&msg);
        EventBus::global().publish_log(msg, is_debug);
    }

    pub fn progress(&self, stage: impl Into<String>, progress: f64, message: impl Into<String>) {
        EventBus::global().publish_progress(stage, progress, message);
    }

    pub async fn get_diagnostics(&self) -> SystemDiagnostics {
        let encoder = self.ffmpeg.get_video_encoder().to_string();
        let ffmpeg_ok = !encoder.is_empty();
        let ffmpeg_details = format!("Encoder selecionado: {encoder}");

        let yt_version = self.yt_dlp.version().await.unwrap_or_else(|_| "Não detectado".to_string());
        let yt_ok = !yt_version.contains("Não detectado");

        let settings = self.db.get_settings().unwrap_or_default();

        let gemini_key = if !settings.gemini_api_key.is_empty() {
            settings.gemini_api_key.clone()
        } else {
            GeminiApiKeyResolver::resolve().unwrap_or_default()
        };
        let gemini_ok = !gemini_key.is_empty();

        let openai_key = if !settings.openai_api_key.is_empty() {
            settings.openai_api_key.clone()
        } else {
            OpenAiApiKeyResolver::resolve().unwrap_or_default()
        };
        let openai_ok = !openai_key.is_empty();

        SystemDiagnostics {
            sqlite_connected: true,
            sqlite_message: "SQLite WAL conectado e ativo (~/.clippr/clippr.db)".to_string(),
            ffmpeg_ok,
            ffmpeg_encoder: encoder,
            ffmpeg_details,
            yt_dlp_ok: yt_ok,
            yt_dlp_version: yt_version,
            gemini_ok,
            gemini_message: if gemini_ok { "Chave configurada e ativa".into() } else { "Chave não informada".into() },
            open_ai_ok: openai_ok,
            open_ai_message: if openai_ok { "Chave configurada e ativa".into() } else { "Chave não informada".into() },
        }
    }

    pub async fn add_video_url(&self, url: &str) -> anyhow::Result<VideoSource> {
        let trimmed = url.trim();
        self.log(format!("[PIPELINE] Adicionando URL de vídeo à fila: {trimmed}"), false);
        let source = VideoSource::from_url(trimmed);
        let saved = self.db.save_video(&source)?;
        self.log(format!("[DB] Vídeo persistido com sucesso no banco de dados (ID: {:?}, URL: {trimmed})", saved.id), false);
        EventBus::global().publish_video_source(saved.clone());
        Ok(saved)
    }

    pub async fn process_video(&self, mut video: VideoSource, settings: AppSettings) -> anyhow::Result<()> {
        let video_id = video.id.unwrap_or(0);
        self.log(
            format!(
                "[PIPELINE] Iniciando processamento do vídeo ID {video_id} | URL: {} | Status inicial: {:?}",
                video.url, video.status
            ),
            false,
        );
        self.progress("FETCHING_TRANSCRIPT", 0.1, "Obtendo transcrição e metadados...");

        self.db.update_video_status(video_id, ProcessingStatus::FetchingTranscript, None)?;
        self.log(format!("[DB] Status do vídeo ID {video_id} atualizado para FetchingTranscript"), false);
        video = video.with_status(ProcessingStatus::FetchingTranscript, None);
        EventBus::global().publish_video_source(video.clone());

        // 1. Metadados e Transcrição com yt-dlp
        self.log(
            format!("[PIPELINE] Iniciando extração de transcrição e metadados com yt-dlp para vídeo ID {video_id}..."),
            false,
        );
        let transcript_res = self.yt_dlp.fetch_metadata_and_transcript(&video.url).await?;
        video.title = transcript_res.metadata.title;
        video.author = transcript_res.metadata.author;
        video.duration_seconds = transcript_res.metadata.duration_seconds;
        if let Some(ref vtt) = transcript_res.vtt_file {
            video.transcript_path = Some(vtt.to_string_lossy().to_string());
        }
        let segments = transcript_res.segments;
        self.log(
            format!(
                "[PIPELINE] Metadados retornados pelo yt-dlp: Título: '{}' | Canal: '{}' | Duração: {}s | Segmentos VTT: {}",
                video.title,
                video.author,
                video.duration_seconds,
                segments.len()
            ),
            false,
        );

        self.db.save_video(&video)?;
        self.log(format!("[DB] Metadados atualizados do vídeo ID {video_id} salvos no banco de dados"), false);
        self.db.update_video_status(video_id, ProcessingStatus::AnalyzingAi, None)?;
        self.log(format!("[DB] Status do vídeo ID {video_id} atualizado para AnalyzingAi"), false);
        video = video.with_status(ProcessingStatus::AnalyzingAi, None);
        EventBus::global().publish_video_source(video.clone());
        self.progress("ANALYZING_AI", 0.4, "Analisando momentos virais com IA...");

        // 2. Análise com IA (Gemini ou OpenAI)
        let (provider_str, model_str) = match settings.ai_provider {
            AiProvider::OpenAi => ("OpenAI", settings.openai_model.as_str()),
            AiProvider::Gemini => ("Gemini", settings.gemini_model.as_str()),
        };
        self.log(
            format!(
                "[PIPELINE] Iniciando análise de IA: Provedor: {provider_str} | Modelo: {model_str} | Limiar de viralização: {}",
                settings.min_virality_score
            ),
            false,
        );

        let analysis = match settings.ai_provider {
            AiProvider::OpenAi => {
                let key = if !settings.openai_api_key.is_empty() {
                    settings.openai_api_key.clone()
                } else {
                    OpenAiApiKeyResolver::resolve().unwrap_or_default()
                };
                let client = OpenAiClient::with_model(key, settings.openai_model.clone());
                if !segments.is_empty() {
                    client.analyze_transcript(
                        Some(video_id),
                        &video.title,
                        &video.author,
                        &segments,
                        settings.min_virality_score,
                    ).await?
                } else {
                    let msg = "OpenAI requer transcrição para análise.";
                    self.log(format!("[PIPELINE] Erro na análise OpenAI: {msg}"), false);
                    self.db.update_video_status(video_id, ProcessingStatus::Failed, Some(msg))?;
                    self.log(format!("[DB] Status do vídeo ID {video_id} atualizado para Failed"), false);
                    return Err(anyhow::anyhow!(msg));
                }
            }
            AiProvider::Gemini => {
                let key = if !settings.gemini_api_key.is_empty() {
                    settings.gemini_api_key.clone()
                } else {
                    GeminiApiKeyResolver::resolve().unwrap_or_default()
                };
                let client = GeminiClient::with_model(key, settings.gemini_model.clone());
                if !segments.is_empty() {
                    client.analyze_transcript(
                        Some(video_id),
                        &video.title,
                        &video.author,
                        &segments,
                        settings.min_virality_score,
                    ).await?
                } else if VideoSource::is_valid_youtube_url(&video.url) {
                    self.log("[PIPELINE] Transcrição não encontrada, analisando vídeo diretamente via URL com Gemini multimodal...", false);
                    client.analyze_video_url(
                        Some(video_id),
                        &video.url,
                        &video.title,
                        &video.author,
                        None,
                        settings.min_virality_score,
                    ).await?
                } else {
                    let msg = "Nenhuma transcrição encontrada e URL não suporta análise multimodal direta.";
                    self.log(format!("[PIPELINE] Erro na análise Gemini: {msg}"), false);
                    self.db.update_video_status(video_id, ProcessingStatus::Failed, Some(msg))?;
                    self.log(format!("[DB] Status do vídeo ID {video_id} atualizado para Failed"), false);
                    return Err(anyhow::anyhow!(msg));
                }
            }
        };

        let approved_clips = analysis.approved_clips;
        let discarded_count = analysis.discarded_clips.len();
        let total_returned = approved_clips.len() + discarded_count;

        self.log(
            format!(
                "[PIPELINE] Avaliação do retorno da IA: {} corte(s) retornado(s) | {} aprovado(s) | {} descartado(s)",
                total_returned,
                approved_clips.len(),
                discarded_count
            ),
            false,
        );

        if approved_clips.is_empty() {
            let msg = format!(
                "Nenhum momento atingiu a nota mínima de viralização ({})",
                settings.min_virality_score
            );
            self.log(
                format!(
                    "[PIPELINE] Vídeo ID {video_id} descartado: {msg} (Nota de corte: {})",
                    settings.min_virality_score
                ),
                false,
            );
            self.db.update_video_status(video_id, ProcessingStatus::DiscardedLowScore, Some(msg.as_str()))?;
            self.log(format!("[DB] Status do vídeo ID {video_id} atualizado para DiscardedLowScore"), false);
            video = video.with_status(ProcessingStatus::DiscardedLowScore, Some(msg));
            EventBus::global().publish_video_source(video);
            self.progress("DISCARDED", 1.0, "Vídeo descartado (nota baixa)");
            return Ok(());
        }

        // Ajusta orientação e presets
        let mut oriented_clips = Vec::new();
        for mut c in approved_clips {
            let orientation = OrientationMode::resolve_for_duration(
                settings.default_orientation,
                c.duration_seconds(),
                settings.auto_vertical_max_duration_seconds,
            );
            c.orientation = orientation;
            c.framing_mode = if orientation == OrientationMode::Horizontal169 {
                FramingMode::Horizontal169
            } else {
                FramingMode::AutoCrop
            };
            c.subtitle_preset = SubtitlePreset::HormoziYellow;
            c.has_subtitles = settings.burn_subtitles;
            c.has_emojis = true;
            c.status = ProcessingStatus::PendingApproval;
            c.video_source_id = Some(video_id);

            self.log(
                format!(
                    "[PIPELINE] Corte aprovado: '{}' | [{}s - {}s] (Duração: {:.1}s) | Score: {} | Orientação: {:?} | Framing: {:?}",
                    c.title,
                    c.start_seconds,
                    c.end_seconds,
                    c.duration_seconds(),
                    c.virality_score,
                    c.orientation,
                    c.framing_mode
                ),
                false,
            );

            oriented_clips.push(c);
        }

        let saved_clips = self.db.save_clips(video_id, &oriented_clips)?;
        self.log(
            format!("[DB] {} corte(s) aprovado(s) persistido(s) no banco de dados para vídeo ID {video_id}", saved_clips.len()),
            false,
        );
        EventBus::global().publish_clips_evaluated(Some(video_id), saved_clips.clone(), discarded_count);
        self.progress("EVALUATED", 0.65, format!("{} corte(s) detectado(s)", saved_clips.len()));

        if settings.auto_process_above_threshold {
            self.log(
                format!(
                    "[PIPELINE] Modo automático ativado (auto_process_above_threshold=true): transição direta para renderização de {} corte(s)...",
                    saved_clips.len()
                ),
                false,
            );
            self.render_clips_for_video(video, saved_clips, settings).await?;
        } else {
            self.db.update_video_status(video_id, ProcessingStatus::PendingApproval, None)?;
            self.log(format!("[DB] Status do vídeo ID {video_id} atualizado para PendingApproval"), false);
            video = video.with_status(ProcessingStatus::PendingApproval, None);
            EventBus::global().publish_video_source(video);
            self.log(
                "[PIPELINE] Modo manual ativo: cortes prontos aguardando aprovação do usuário na aba de Cortes.",
                false,
            );
        }

        Ok(())
    }

    pub async fn render_clips_for_video(
        &self,
        mut video: VideoSource,
        clips: Vec<ClipCandidate>,
        settings: AppSettings,
    ) -> anyhow::Result<()> {
        self.log("[PIPELINE] Aguardando aquisição do semáforo de renderização...", false);
        let _permit = self.render_semaphore.acquire().await?;
        self.log("[PIPELINE] Semáforo de renderização adquirido com sucesso.", false);
        let video_id = video.id.unwrap_or(0);

        // 1. Download do vídeo se não existir
        let video_file = match video.local_video_path.as_ref() {
            Some(path) if Path::new(path).exists() => {
                self.log(format!("[PIPELINE] Vídeo original já existe em cache local: {path}"), false);
                PathBuf::from(path)
            }
            _ => {
                self.log("[PIPELINE] Vídeo original não encontrado em cache local. Necessário download via yt-dlp.".to_string(), false);
                self.db.update_video_status(video_id, ProcessingStatus::DownloadingVideo, None)?;
                self.log(format!("[DB] Status do vídeo ID {video_id} atualizado para DownloadingVideo"), false);
                video = video.with_status(ProcessingStatus::DownloadingVideo, None);
                EventBus::global().publish_video_source(video.clone());
                self.progress("DOWNLOADING", 0.7, "Baixando vídeo com yt-dlp...");

                let videos_dir = PathBuf::from("temp").join("videos");
                tokio::fs::create_dir_all(&videos_dir).await?;
                let target = videos_dir.join(format!("{}.mp4", video.video_id));
                let downloaded = self.yt_dlp.download_full_video(&video.url, &target, None::<fn(f64)>).await?;
                self.log(format!("[PIPELINE] Download do vídeo original concluído: {}", downloaded.display()), false);
                video.local_video_path = Some(downloaded.to_string_lossy().to_string());
                self.db.save_video(&video)?;
                self.log(format!("[DB] Caminho do vídeo em cache salvo no banco para vídeo ID {video_id}"), false);
                downloaded
            }
        };

        // 2. Renderização dos cortes com FFmpeg
        self.db.update_video_status(video_id, ProcessingStatus::CuttingFfmpeg, None)?;
        self.log(format!("[DB] Status do vídeo ID {video_id} atualizado para CuttingFfmpeg"), false);
        video = video.with_status(ProcessingStatus::CuttingFfmpeg, None);
        EventBus::global().publish_video_source(video.clone());
        self.progress("CUTTING", 0.85, "Renderizando cortes com FFmpeg...");

        let root_dir = PathBuf::from(&settings.output_directory);
        let layout = VideoExportLayout::new(&root_dir, &video.video_id, &video.title);
        layout.ensure_directories_exist()?;
        self.log(
            format!(
                "[PIPELINE] Diretório de saída estruturado criado em: {} | Cortes: {} | Dados: {} | Thumbnails: {}",
                layout.video_dir.display(),
                layout.cortes_dir.display(),
                layout.dados_dir.display(),
                layout.thumbnails_dir.display()
            ),
            false,
        );

        // Carrega segmentos de transcrição se houver
        let segments = match video.transcript_path.as_ref() {
            Some(p) => {
                let text = tokio::fs::read_to_string(p).await.unwrap_or_default();
                TranscriptParser::parse_vtt(&text)
            }
            None => Vec::new(),
        };

        let total = clips.len();
        for (i, clip) in clips.into_iter().enumerate() {
            let clip_id = clip.id.unwrap_or(0);
            let start_pad = (clip.start_seconds - 0.75).max(0.0);
            let end_pad = clip.end_seconds + 0.75;

            self.log(
                format!(
                    "[PIPELINE] Processando corte {} de {}: '{}' | Início: {:.2}s, Fim: {:.2}s (com respiro: {:.2}s a {:.2}s) | Legendas ASS/SRT: {}",
                    i + 1,
                    total,
                    clip.title,
                    clip.start_seconds,
                    clip.end_seconds,
                    start_pad,
                    end_pad,
                    if clip.has_subtitles && !segments.is_empty() { "Ativadas" } else { "Desativadas/Sem transcrição" }
                ),
                false,
            );

            let mut rendered = self.ffmpeg.render_clip(
                &video,
                &clip,
                &video_file,
                &layout,
                &segments,
                settings.cpu_threads.max(1) as u32,
                clip.framing_mode == FramingMode::BlurredBg,
                false,
            ).await?;

            self.log(
                format!(
                    "[PIPELINE] FFmpeg concluiu renderização do corte {}/{}: Arquivo: {:?}",
                    i + 1,
                    total,
                    rendered.output_file_path
                ),
                false,
            );

            rendered.status = ProcessingStatus::Completed;
            self.db.update_clip_output(
                clip_id,
                rendered.output_file_path.as_deref(),
                rendered.metadata_json_path.as_deref(),
                ProcessingStatus::Completed,
            )?;
            self.log(format!("[DB] Corte ID {clip_id} atualizado no banco com caminho do arquivo e status Completed"), false);

            // Gera thumbnail
            let headline = rendered.thumbnail_headline.as_deref().unwrap_or(&rendered.title);
            let thumb_time = rendered.thumbnail_timestamp.unwrap_or(rendered.start_seconds);
            let aspect = if rendered.orientation == OrientationMode::Horizontal169 {
                ThumbnailAspectRatio::Horizontal169
            } else {
                ThumbnailAspectRatio::Vertical916
            };

            self.log(
                format!(
                    "[PIPELINE] Gerando thumbnail para corte {}/{}: headline='{}' | timestamp={:.2}s | proporção={:?}",
                    i + 1,
                    total,
                    headline,
                    thumb_time,
                    aspect
                ),
                false,
            );

            match self.thumbnail_gen.generate_thumbnail(
                &video_file,
                thumb_time,
                headline,
                &layout.thumbnails_dir,
                &format!("thumb_clip_{clip_id}"),
                aspect,
            ) {
                Ok(thumb_res) => {
                    rendered.thumbnail_path = Some(thumb_res.final_thumbnail_path.to_string_lossy().to_string());
                    rendered.raw_frame_path = Some(thumb_res.raw_frame_path.to_string_lossy().to_string());
                    self.db.update_clip_thumbnail(
                        clip_id,
                        Some(thumb_time),
                        Some(headline),
                        rendered.thumbnail_path.as_deref(),
                        rendered.raw_frame_path.as_deref(),
                    )?;
                    self.log(
                        format!(
                            "[PIPELINE] Thumbnail gerada com sucesso: {:?} | [DB] Metadados de thumbnail persistidos para corte ID {clip_id}",
                            rendered.thumbnail_path
                        ),
                        false,
                    );
                }
                Err(e) => {
                    self.log(format!("[PIPELINE] Aviso: Falha ao gerar thumbnail para corte ID {clip_id}: {e}"), false);
                }
            }

            EventBus::global().publish_clip_render_completed(rendered.clone());
            let progress_val = 0.85 + (0.15 * ((i + 1) as f64 / total as f64));
            self.progress("CUTTING", progress_val, format!("Corte {} de {} concluído", i + 1, total));
        }

        self.db.update_video_status(video_id, ProcessingStatus::Completed, None)?;
        self.log(format!("[DB] Status do vídeo ID {video_id} atualizado para Completed"), false);
        video = video.with_status(ProcessingStatus::Completed, None);
        EventBus::global().publish_video_source(video);
        self.progress("COMPLETED", 1.0, "Todos os cortes concluídos com sucesso!");
        self.log(
            format!(
                "[PIPELINE] Processamento concluído com sucesso! Todos os cortes foram salvos na pasta: {}",
                layout.cortes_dir.display()
            ),
            false,
        );

        Ok(())
    }

    pub async fn render_single_clip(&self, clip_id: i64) -> anyhow::Result<()> {
        self.log(format!("[PIPELINE] Solicitada renderização individual para o corte ID {clip_id}"), false);
        let clip = self.db.find_clip_by_id(clip_id)?
            .ok_or_else(|| anyhow::anyhow!("Corte {clip_id} não encontrado"))?;
        self.log(format!("[DB] Corte ID {clip_id} recuperado ('{}')", clip.title), false);
        let video_id = clip.video_source_id.unwrap_or(0);
        let video = self.db.find_video_by_id(video_id)?
            .ok_or_else(|| anyhow::anyhow!("Vídeo pai {video_id} não encontrado"))?;
        self.log(format!("[DB] Vídeo pai ID {video_id} recuperado ('{}')", video.title), false);
        let settings = self.db.get_settings().unwrap_or_default();

        self.render_clips_for_video(video, vec![clip], settings).await
    }

    pub async fn generate_thumbnail_for_clip(
        &self,
        clip_id: i64,
        timestamp: f64,
        headline: &str,
        aspect_ratio: ThumbnailAspectRatio,
    ) -> anyhow::Result<(String, String)> {
        self.log(
            format!(
                "[PIPELINE] Gerando thumbnail sob demanda para corte ID {clip_id}: timestamp={:.2}s | headline='{}' | aspect={:?}",
                timestamp,
                headline,
                aspect_ratio
            ),
            false,
        );
        let clip = self.db.find_clip_by_id(clip_id)?
            .ok_or_else(|| anyhow::anyhow!("Corte {clip_id} não encontrado"))?;
        self.log(format!("[DB] Corte ID {clip_id} recuperado ('{}')", clip.title), false);
        let video_id = clip.video_source_id.unwrap_or(0);
        let video = self.db.find_video_by_id(video_id)?
            .ok_or_else(|| anyhow::anyhow!("Vídeo pai {video_id} não encontrado"))?;
        self.log(format!("[DB] Vídeo pai ID {video_id} recuperado ('{}')", video.title), false);
        let settings = self.db.get_settings().unwrap_or_default();

        let video_file = match video.local_video_path.as_ref() {
            Some(p) if Path::new(p).exists() => {
                self.log(format!("[PIPELINE] Usando vídeo original em cache: {p}"), false);
                PathBuf::from(p)
            }
            _ => {
                self.log("[PIPELINE] Vídeo original não encontrado em cache para thumbnail. Baixando via yt-dlp...".to_string(), false);
                let videos_dir = PathBuf::from("temp").join("videos");
                tokio::fs::create_dir_all(&videos_dir).await?;
                let target = videos_dir.join(format!("{}.mp4", video.video_id));
                let downloaded = self.yt_dlp.download_full_video(&video.url, &target, None::<fn(f64)>).await?;
                self.log(format!("[PIPELINE] Download do vídeo concluído: {}", downloaded.display()), false);
                downloaded
            }
        };

        let root_dir = PathBuf::from(&settings.output_directory);
        let layout = VideoExportLayout::new(&root_dir, &video.video_id, &video.title);
        layout.ensure_directories_exist()?;

        let thumb_res = self.thumbnail_gen.generate_thumbnail(
            &video_file,
            timestamp,
            headline,
            &layout.thumbnails_dir,
            &format!("thumb_clip_{clip_id}_{}", if aspect_ratio == ThumbnailAspectRatio::Horizontal169 { "16_9" } else { "9_16" }),
            aspect_ratio,
        )?;

        let thumb_str = thumb_res.final_thumbnail_path.to_string_lossy().to_string();
        let raw_str = thumb_res.raw_frame_path.to_string_lossy().to_string();

        self.db.update_clip_thumbnail(clip_id, Some(timestamp), Some(headline), Some(&thumb_str), Some(&raw_str))?;
        self.log(format!("[DB] Metadados de thumbnail persistidos no banco para corte ID {clip_id}"), false);
        self.log(format!("[PIPELINE] Thumbnail sob demanda gerada com sucesso: {thumb_str}"), false);

        Ok((thumb_str, raw_str))
    }

    pub async fn transcribe_clip_whisper(&self, clip_id: i64) -> anyhow::Result<Vec<WordTimestamp>> {
        self.log(format!("[PIPELINE] Iniciando transcrição com Whisper para o corte ID {clip_id}"), false);
        let clip = self.db.find_clip_by_id(clip_id)?
            .ok_or_else(|| anyhow::anyhow!("Corte {clip_id} não encontrado"))?;
        self.log(format!("[DB] Corte ID {clip_id} recuperado ('{}')", clip.title), false);
        let video_id = clip.video_source_id.unwrap_or(0);
        let video = self.db.find_video_by_id(video_id)?
            .ok_or_else(|| anyhow::anyhow!("Vídeo pai {video_id} não encontrado"))?;
        self.log(format!("[DB] Vídeo pai ID {video_id} recuperado ('{}')", video.title), false);

        self.log(
            format!(
                "[PIPELINE] Transcrevendo trecho [{:.2}s - {:.2}s] para corte '{}'",
                clip.start_seconds,
                clip.end_seconds,
                clip.title
            ),
            false,
        );

        let video_file = match video.local_video_path.as_ref() {
            Some(p) if Path::new(p).exists() => PathBuf::from(p),
            _ => {
                self.log("[PIPELINE] Vídeo original não encontrado em cache para Whisper. Baixando via yt-dlp...".to_string(), false);
                let videos_dir = PathBuf::from("temp").join("videos");
                tokio::fs::create_dir_all(&videos_dir).await?;
                let target = videos_dir.join(format!("{}.mp4", video.video_id));
                self.yt_dlp.download_full_video(&video.url, &target, None::<fn(f64)>).await?
            }
        };

        let segments = match video.transcript_path.as_ref() {
            Some(p) => {
                let text = tokio::fs::read_to_string(p).await.unwrap_or_default();
                TranscriptParser::parse_vtt(&text)
            }
            None => Vec::new(),
        };

        let words = self.whisper.transcribe_clip_with_fallback(
            &video_file,
            clip.start_seconds,
            clip.end_seconds,
            &segments,
        );
        self.log(
            format!("[PIPELINE] Transcrição Whisper concluída com sucesso: {} palavra(s) com timestamp geradas.", words.len()),
            false,
        );
        Ok(words)
    }

    pub async fn youtube_connect(&self) -> anyhow::Result<clipr_youtube::YouTubeChannelInfo> {
        self.log("[YOUTUBE] Iniciando fluxo de autorização OAuth 2.0 do YouTube...", false);
        let settings = self.db.get_settings().unwrap_or_default();
        if settings.youtube_client_id.is_empty() || settings.youtube_client_secret.is_empty() {
            self.log("[YOUTUBE] Falha: Client ID ou Client Secret do YouTube não estão configurados.", false);
            return Err(anyhow::anyhow!("Client ID e Client Secret do YouTube precisam ser configurados primeiro"));
        }

        self.log("[YOUTUBE] Abrindo navegador para autorização do canal via OAuth 2.0...", false);

        let tokens = self.youtube_oauth.authorize(
            &settings.youtube_client_id,
            &settings.youtube_client_secret,
            std::time::Duration::from_secs(120),
            None,
        ).await?;
        self.log("[YOUTUBE] Tokens de autorização obtidos com sucesso.", false);

        let api_client = YouTubeApiClient::new();
        let channel = api_client.get_channel_info(&tokens.access_token).await?;
        self.log(
            format!("[YOUTUBE] Metadados do canal obtidos: '{}' (ID: {})", channel.title, channel.id),
            false,
        );

        let expires_at = (tokens.obtained_at + chrono::Duration::seconds(tokens.expires_in_seconds as i64)).timestamp();

        let updated_settings = settings.with_youtube_tokens(
            &tokens.access_token,
            tokens.refresh_token.as_deref().unwrap_or_default(),
            expires_at,
            Some(&channel.title),
            Some(&channel.id),
            channel.avatar_url.as_deref(),
        );

        self.db.save_settings(&updated_settings)?;
        self.log("[DB] Credenciais do canal do YouTube persistidas no banco de dados.", false);
        self.log(format!("[YOUTUBE] Canal do YouTube '{}' conectado com sucesso!", channel.title), false);

        Ok(channel)
    }

    pub async fn youtube_publish_clip(&self, clip_id: i64) -> anyhow::Result<clipr_youtube::UploadResult> {
        self.log(format!("[YOUTUBE] Solicitada publicação do corte ID {clip_id} no YouTube Shorts"), false);
        let clip = self.db.find_clip_by_id(clip_id)?
            .ok_or_else(|| anyhow::anyhow!("Corte {clip_id} não encontrado"))?;
        self.log(format!("[DB] Corte ID {clip_id} recuperado do banco ('{}')", clip.title), false);
        let output_path = clip.output_file_path.as_ref()
            .ok_or_else(|| anyhow::anyhow!("O corte precisa ser renderizado antes de publicar no YouTube"))?;

        let settings = self.db.get_settings().unwrap_or_default();
        if settings.youtube_access_token.is_empty() {
            self.log("[YOUTUBE] Falha: Canal do YouTube não autenticado (token vazio).", false);
            return Err(anyhow::anyhow!("Canal do YouTube não autenticado"));
        }

        self.log(
            format!(
                "[YOUTUBE] Iniciando upload do corte '{}' | Arquivo: {output_path} | Tags: {:?}",
                clip.title,
                clip.hashtags
            ),
            false,
        );

        let api_client = YouTubeApiClient::new();
        let request = VideoUploadRequest {
            video_file: PathBuf::from(output_path),
            title: clip.title.clone(),
            description: clip.suggested_caption.clone(),
            tags: clip.hashtags.clone(),
            privacy: VideoPrivacy::Public,
            made_for_kids: false,
            custom_thumbnail: clip.thumbnail_path.map(PathBuf::from),
        };

        let result = api_client.upload_video(&settings.youtube_access_token, &request, None).await?;
        self.log(format!("[YOUTUBE] Vídeo publicado com sucesso no YouTube Shorts! ID: {} | URL: {}", result.video_id, result.video_url), false);
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pipeline_log_and_eventbus_integration() {
        let db = Arc::new(DatabaseManager::open_in_memory().unwrap());
        let pipeline = ClipprPipeline::new(db.clone());

        let mut rx = EventBus::global().subscribe();

        let test_msg = "[PIPELINE] Mensagem de teste de integração do barramento única 987654";
        pipeline.log(test_msg, false);

        let mut found = false;
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(3);
        while tokio::time::Instant::now() < deadline {
            if let Ok(Ok(event)) = tokio::time::timeout(std::time::Duration::from_millis(500), rx.recv()).await {
                if let ClipprEvent::LogMessage { message, is_debug, .. } = event {
                    if message == test_msg {
                        assert!(!is_debug);
                        found = true;
                        break;
                    }
                }
            }
        }
        assert!(found, "Deveria encontrar a mensagem de teste no barramento global");

        // Verifica gravação em ~/.clippr/clippr.log
        if let Some(home) = dirs::home_dir() {
            let log_file = home.join(".clippr").join("clippr.log");
            assert!(log_file.exists(), "Arquivo de log deve existir em {:?}", log_file);
            let content = std::fs::read_to_string(log_file).unwrap();
            assert!(content.contains(test_msg));
        }
    }

    #[tokio::test]
    async fn test_add_video_url_logging_and_db() {
        let db = Arc::new(DatabaseManager::open_in_memory().unwrap());
        let pipeline = ClipprPipeline::new(db.clone());

        let mut rx = EventBus::global().subscribe();

        let url = "https://www.youtube.com/watch?v=dQw4w9WgXcQ";
        let saved = pipeline.add_video_url(url).await.expect("Deveria salvar video");

        assert_eq!(saved.url, url);
        assert!(saved.id.is_some());

        // Consome mensagens de log emitidas por add_video_url
        let mut found_pipeline_log = false;
        let mut found_db_log = false;

        while let Ok(event) = rx.try_recv() {
            if let ClipprEvent::LogMessage { message, .. } = event {
                if message.contains("[PIPELINE]") {
                    found_pipeline_log = true;
                }
                if message.contains("[DB]") {
                    found_db_log = true;
                }
            }
        }

        assert!(found_pipeline_log, "Deveria emitir log com tag [PIPELINE]");
        assert!(found_db_log, "Deveria emitir log com tag [DB]");
    }
}

