pub mod commands;
pub mod pipeline;

use std::sync::Arc;
use tauri::Manager;

use clipr_persistence::DatabaseManager;
use pipeline::ClipprPipeline;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .try_init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let db = Arc::new(DatabaseManager::open_default().expect("Falha ao abrir SQLite local"));
            let pipeline = Arc::new(ClipprPipeline::new(db.clone()));

            pipeline.log("[PIPELINE] Backend Tauri inicializado com sucesso. SQLite conectado em modo WAL.", false);

            let handle = app.handle().clone();
            let pipe_clone = pipeline.clone();

            tauri::async_runtime::spawn(async move {
                pipe_clone.set_app_handle(handle).await;
            });

            app.manage(db);
            app.manage(pipeline);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_diagnostics,
            commands::get_settings,
            commands::save_settings,
            commands::detect_gemini_key,
            commands::detect_openai_key,
            commands::get_videos,
            commands::add_video_url,
            commands::clear_queue,
            commands::start_pipeline,
            commands::get_clips,
            commands::update_clip_bounds,
            commands::update_clip_framing_subtitle,
            commands::get_clip_words,
            commands::save_clip_words,
            commands::render_clip,
            commands::render_all_clips,
            commands::generate_thumbnail,
            commands::open_path,
            commands::transcribe_clip_whisper,
            commands::youtube_connect,
            commands::youtube_disconnect,
            commands::youtube_save_credentials,
            commands::youtube_publish_clip,
            commands::select_directory,
        ])
        .run(tauri::generate_context!())
        .expect("Erro ao executar aplicativo Tauri");
}
