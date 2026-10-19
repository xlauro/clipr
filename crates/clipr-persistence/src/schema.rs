use crate::error::PersistenceError;
use rusqlite::Connection;

pub fn initialize_schema(conn: &Connection) -> Result<(), PersistenceError> {
    conn.execute_batch(
        r#"
        PRAGMA foreign_keys = ON;

        CREATE TABLE IF NOT EXISTS videos (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            url TEXT NOT NULL,
            video_id TEXT NOT NULL,
            title TEXT NOT NULL,
            author TEXT NOT NULL DEFAULT '',
            duration_seconds REAL NOT NULL DEFAULT 0.0,
            transcript_path TEXT,
            local_video_path TEXT,
            status TEXT NOT NULL,
            error_message TEXT,
            created_at TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_videos_created_at ON videos(created_at);

        CREATE TABLE IF NOT EXISTS clips (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            video_source_id INTEGER NOT NULL REFERENCES videos(id) ON DELETE CASCADE,
            title TEXT NOT NULL,
            hook TEXT NOT NULL DEFAULT '',
            start_seconds REAL NOT NULL,
            end_seconds REAL NOT NULL,
            virality_score INTEGER NOT NULL,
            reasoning TEXT NOT NULL DEFAULT '',
            suggested_caption TEXT NOT NULL DEFAULT '',
            hashtags_json TEXT NOT NULL DEFAULT '[]',
            orientation TEXT NOT NULL,
            has_subtitles INTEGER NOT NULL DEFAULT 1,
            has_emojis INTEGER NOT NULL DEFAULT 1,
            output_file_path TEXT,
            metadata_json_path TEXT,
            status TEXT NOT NULL,
            thumbnail_timestamp REAL,
            thumbnail_headline TEXT,
            thumbnail_path TEXT,
            raw_frame_path TEXT,
            framing_mode TEXT NOT NULL DEFAULT 'AUTO_CROP',
            subtitle_preset TEXT NOT NULL DEFAULT 'HORMOZI_YELLOW',
            emotion_tag TEXT NOT NULL DEFAULT 'ALTA_ENERGIA',
            scores_json TEXT NOT NULL DEFAULT '{}',
            created_at TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_clips_video_source_id ON clips(video_source_id);

        CREATE TABLE IF NOT EXISTS settings (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            output_directory TEXT NOT NULL,
            min_virality_score INTEGER NOT NULL DEFAULT 90,
            auto_process INTEGER NOT NULL DEFAULT 0,
            default_orientation TEXT NOT NULL DEFAULT 'AUTO',
            auto_vertical_max_duration REAL NOT NULL DEFAULT 60.0,
            burn_subtitles INTEGER NOT NULL DEFAULT 1,
            cpu_threads INTEGER NOT NULL DEFAULT 0,
            gemini_api_key TEXT NOT NULL DEFAULT '',
            gemini_model TEXT NOT NULL DEFAULT 'gemini-2.5-flash',
            ai_provider TEXT NOT NULL DEFAULT 'GEMINI',
            openai_api_key TEXT NOT NULL DEFAULT '',
            openai_model TEXT NOT NULL DEFAULT 'gpt-4o-mini',
            youtube_client_id TEXT NOT NULL DEFAULT '',
            youtube_client_secret TEXT NOT NULL DEFAULT '',
            youtube_access_token TEXT NOT NULL DEFAULT '',
            youtube_refresh_token TEXT NOT NULL DEFAULT '',
            youtube_token_expires_at INTEGER NOT NULL DEFAULT 0,
            youtube_channel_title TEXT NOT NULL DEFAULT '',
            youtube_channel_id TEXT NOT NULL DEFAULT '',
            youtube_channel_avatar_url TEXT NOT NULL DEFAULT ''
        );

        CREATE VIEW IF NOT EXISTS video_sources AS SELECT * FROM videos;
        CREATE VIEW IF NOT EXISTS app_settings AS SELECT * FROM settings;
        "#,
    )?;

    Ok(())
}
