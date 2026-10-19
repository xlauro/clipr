use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;

use clipr_ai::{GeminiApiKeyResolver, OpenAiApiKeyResolver};
use clipr_domain::{
    AppSettings, ClipCandidate, FramingMode, SubtitlePreset, ThumbnailAspectRatio, VideoSource,
    WordTimestamp,
};
use clipr_persistence::DatabaseManager;

use crate::pipeline::{ClipprPipeline, SystemDiagnostics};

#[tauri::command]
pub async fn get_diagnostics(pipeline: State<'_, Arc<ClipprPipeline>>) -> Result<SystemDiagnostics, String> {
    Ok(pipeline.get_diagnostics().await)
}

#[tauri::command]
pub async fn get_settings(db: State<'_, Arc<DatabaseManager>>) -> Result<AppSettings, String> {
    db.get_settings().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_settings(
    settings: AppSettings,
    db: State<'_, Arc<DatabaseManager>>,
) -> Result<bool, String> {
    db.save_settings(&settings).map_err(|e| e.to_string())?;
    Ok(true)
}

#[tauri::command]
pub async fn detect_gemini_key() -> Result<String, String> {
    Ok(GeminiApiKeyResolver::resolve().unwrap_or_default())
}

#[tauri::command]
pub async fn detect_openai_key() -> Result<String, String> {
    Ok(OpenAiApiKeyResolver::resolve().unwrap_or_default())
}

#[tauri::command]
pub async fn get_videos(db: State<'_, Arc<DatabaseManager>>) -> Result<Vec<VideoSource>, String> {
    db.find_all_videos().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_video_url(
    url: String,
    pipeline: State<'_, Arc<ClipprPipeline>>,
) -> Result<VideoSource, String> {
    pipeline.add_video_url(&url).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn clear_queue(db: State<'_, Arc<DatabaseManager>>) -> Result<bool, String> {
    let videos = db.find_all_videos().map_err(|e| e.to_string())?;
    for v in videos {
        if let Some(id) = v.id {
            let _ = db.delete_video(id);
        }
    }
    Ok(true)
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineStartOptions {
    pub burn_subtitles: bool,
    pub auto_process_above_threshold: bool,
    pub orientation: String,
}

#[tauri::command]
pub async fn start_pipeline(
    options: PipelineStartOptions,
    pipeline: State<'_, Arc<ClipprPipeline>>,
    db: State<'_, Arc<DatabaseManager>>,
) -> Result<bool, String> {
    let mut settings = db.get_settings().map_err(|e| e.to_string())?;
    settings.burn_subtitles = options.burn_subtitles;
    settings.auto_process_above_threshold = options.auto_process_above_threshold;
    if options.orientation == "VERTICAL_9_16" {
        settings.default_orientation = clipr_domain::OrientationMode::Vertical916;
    } else if options.orientation == "HORIZONTAL_16_9" {
        settings.default_orientation = clipr_domain::OrientationMode::Horizontal169;
    } else {
        settings.default_orientation = clipr_domain::OrientationMode::Auto;
    }

    let videos = db.find_all_videos().map_err(|e| e.to_string())?;
    let pipe = pipeline.inner().clone();

    tokio::spawn(async move {
        if videos.is_empty() {
            pipe.log("[PIPELINE] A fila está vazia. Adicione ao menos um link do YouTube antes de iniciar.", false);
            pipe.progress("IDLE", 1.0, "Fila vazia.");
            return;
        }

        let pending_videos: Vec<_> = videos
            .into_iter()
            .filter(|v| v.status == clipr_domain::ProcessingStatus::Pending || v.status == clipr_domain::ProcessingStatus::Failed)
            .collect();

        if pending_videos.is_empty() {
            pipe.log("[PIPELINE] Nenhum vídeo pendente encontrado na fila para processar.", false);
            pipe.progress("IDLE", 1.0, "Todos os vídeos da fila já foram processados.");
            return;
        }

        let total = pending_videos.len();
        pipe.log(format!("[PIPELINE] Iniciando processamento de {total} vídeo(s) na fila..."), false);

        for (idx, video) in pending_videos.into_iter().enumerate() {
            let vid_url = video.url.clone();
            pipe.log(format!("[PIPELINE] Processando vídeo {} de {total}: {vid_url}", idx + 1), false);
            if let Err(e) = pipe.process_video(video, settings.clone()).await {
                pipe.log(format!("[PIPELINE][ERRO] Falha ao processar {vid_url}: {e}"), false);
                tracing::error!("Erro ao processar vídeo: {e}");
            }
        }

        pipe.log("[PIPELINE] Processamento da fila concluído.", false);
        pipe.progress("COMPLETED", 1.0, "Processamento da fila finalizado.");
    });

    Ok(true)
}

#[tauri::command]
pub async fn get_clips(db: State<'_, Arc<DatabaseManager>>) -> Result<Vec<ClipCandidate>, String> {
    let videos = db.find_all_videos().map_err(|e| e.to_string())?;
    let mut all_clips = Vec::new();
    for v in videos {
        if let Some(id) = v.id {
            if let Ok(clips) = db.find_clips_by_video_id(id) {
                all_clips.extend(clips);
            }
        }
    }
    Ok(all_clips)
}

#[tauri::command]
pub async fn update_clip_bounds(
    clip_id: i64,
    start_seconds: f64,
    end_seconds: f64,
    db: State<'_, Arc<DatabaseManager>>,
) -> Result<bool, String> {
    db.update_clip_time_bounds(clip_id, start_seconds, end_seconds)
        .map_err(|e| e.to_string())?;
    Ok(true)
}

#[tauri::command]
pub async fn update_clip_framing_subtitle(
    clip_id: i64,
    framing_mode: String,
    subtitle_preset: String,
    db: State<'_, Arc<DatabaseManager>>,
) -> Result<bool, String> {
    let framing = FramingMode::from_str_loose(&framing_mode, FramingMode::AutoCrop);
    let preset = match subtitle_preset.as_str() {
        "HORMOZI_YELLOW" => SubtitlePreset::HormoziYellow,
        "CYBER_GREEN" => SubtitlePreset::CyberGreen,
        "CLEAN_WHITE" => SubtitlePreset::CleanWhite,
        "MRBEAST_BLUE_YELLOW" => SubtitlePreset::MrbeastBlueYellow,
        "NEON_MAGENTA" => SubtitlePreset::NeonMagenta,
        _ => SubtitlePreset::MinimalistElegant,
    };

    db.update_clip_framing_and_subtitle(clip_id, framing, preset)
        .map_err(|e| e.to_string())?;
    Ok(true)
}

#[tauri::command]
pub async fn get_clip_words(
    clip_id: i64,
    pipeline: State<'_, Arc<ClipprPipeline>>,
) -> Result<Vec<WordTimestamp>, String> {
    pipeline.transcribe_clip_whisper(clip_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_clip_words(
    _clip_id: i64,
    _words: Vec<WordTimestamp>,
) -> Result<bool, String> {
    Ok(true)
}

#[tauri::command]
pub async fn render_clip(
    clip_id: i64,
    pipeline: State<'_, Arc<ClipprPipeline>>,
) -> Result<bool, String> {
    let pipe = pipeline.inner().clone();
    tokio::spawn(async move {
        let _ = pipe.render_single_clip(clip_id).await;
    });
    Ok(true)
}

#[tauri::command]
pub async fn render_all_clips(
    pipeline: State<'_, Arc<ClipprPipeline>>,
    db: State<'_, Arc<DatabaseManager>>,
) -> Result<bool, String> {
    let videos = db.find_all_videos().map_err(|e| e.to_string())?;
    let settings = db.get_settings().map_err(|e| e.to_string())?;
    let database = db.inner().clone();
    let pipe = pipeline.inner().clone();

    tokio::spawn(async move {
        for video in videos {
            if let Some(id) = video.id {
                if let Ok(clips) = database.find_clips_by_video_id(id) {
                    let _ = pipe.render_clips_for_video(video, clips, settings.clone()).await;
                }
            }
        }
    });

    Ok(true)
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThumbnailResultDto {
    pub thumbnail_path: String,
    pub raw_frame_path: String,
}

#[tauri::command]
pub async fn generate_thumbnail(
    clip_id: i64,
    timestamp: f64,
    headline: String,
    aspect_ratio: String,
    pipeline: State<'_, Arc<ClipprPipeline>>,
) -> Result<ThumbnailResultDto, String> {
    let aspect = if aspect_ratio == "16:9" {
        ThumbnailAspectRatio::Horizontal169
    } else {
        ThumbnailAspectRatio::Vertical916
    };

    let (thumb, raw) = pipeline
        .generate_thumbnail_for_clip(clip_id, timestamp, &headline, aspect)
        .await
        .map_err(|e| e.to_string())?;

    Ok(ThumbnailResultDto {
        thumbnail_path: thumb,
        raw_frame_path: raw,
    })
}

#[tauri::command]
pub async fn open_path(path: String) -> Result<bool, String> {
    let p = PathBuf::from(path);
    let target = if p.is_file() {
        p.parent().unwrap_or(&p).to_path_buf()
    } else {
        p
    };
    open::that(target).map_err(|e| e.to_string())?;
    Ok(true)
}

#[tauri::command]
pub async fn transcribe_clip_whisper(
    clip_id: i64,
    pipeline: State<'_, Arc<ClipprPipeline>>,
) -> Result<Vec<WordTimestamp>, String> {
    pipeline.transcribe_clip_whisper(clip_id).await.map_err(|e| e.to_string())
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct YouTubeChannelDto {
    pub channel_title: String,
    pub avatar_url: String,
}

#[tauri::command]
pub async fn youtube_connect(
    pipeline: State<'_, Arc<ClipprPipeline>>,
) -> Result<YouTubeChannelDto, String> {
    let channel = pipeline.youtube_connect().await.map_err(|e| e.to_string())?;
    Ok(YouTubeChannelDto {
        channel_title: channel.title,
        avatar_url: channel.avatar_url.unwrap_or_default(),
    })
}

#[tauri::command]
pub async fn youtube_disconnect(db: State<'_, Arc<DatabaseManager>>) -> Result<bool, String> {
    let settings = db.get_settings().map_err(|e| e.to_string())?;
    let updated = settings.without_youtube_auth();
    db.save_settings(&updated).map_err(|e| e.to_string())?;
    Ok(true)
}

#[tauri::command]
pub async fn youtube_save_credentials(
    client_id: String,
    client_secret: String,
    db: State<'_, Arc<DatabaseManager>>,
) -> Result<bool, String> {
    let settings = db.get_settings().map_err(|e| e.to_string())?;
    let updated = settings.with_youtube_credentials(&client_id, &client_secret);
    db.save_settings(&updated).map_err(|e| e.to_string())?;
    Ok(true)
}

#[tauri::command]
pub async fn youtube_publish_clip(
    clip_id: i64,
    pipeline: State<'_, Arc<ClipprPipeline>>,
) -> Result<bool, String> {
    pipeline.youtube_publish_clip(clip_id).await.map_err(|e| e.to_string())?;
    Ok(true)
}

#[tauri::command]
pub async fn select_directory() -> Result<Option<String>, String> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    Ok(Some(format!("{home}/ClipprExports")))
}

#[tauri::command]
pub async fn prepare_video_source(
    video_id: i64,
    pipeline: State<'_, Arc<ClipprPipeline>>,
) -> Result<String, String> {
    pipeline.prepare_video_for_preview(video_id).await.map_err(|e| e.to_string())
}
