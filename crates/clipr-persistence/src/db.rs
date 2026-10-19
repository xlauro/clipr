use crate::error::PersistenceError;
use crate::schema::initialize_schema;
use chrono::{DateTime, Utc};
use clipr_domain::prelude::*;
use rusqlite::{params, Connection};
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::{Arc, Mutex};

/// Gerenciador de banco de dados SQLite local de alta performance para o Clippr.
#[derive(Clone, Debug)]
pub struct DatabaseManager {
    conn: Arc<Mutex<Connection>>,
}

impl DatabaseManager {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, PersistenceError> {
        let path_ref = path.as_ref();
        if let Some(parent) = path_ref.parent() {
            fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(path_ref)?;
        // Habilita modo WAL para concorrência de leitura e escrita e alta performance
        conn.execute_batch(
            r#"
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA foreign_keys = ON;
            "#,
        )?;

        initialize_schema(&conn)?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn open_in_memory() -> Result<Self, PersistenceError> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        initialize_schema(&conn)?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn open_default() -> Result<Self, PersistenceError> {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let db_path = PathBuf::from(home).join(".clippr").join("clippr.db");
        Self::open(db_path)
    }

    fn lock_conn(&self) -> Result<std::sync::MutexGuard<'_, Connection>, PersistenceError> {
        self.conn
            .lock()
            .map_err(|e| PersistenceError::LockError(e.to_string()))
    }

    // =========================================================================
    // VideoSource CRUD
    // =========================================================================

    pub fn save_video(&self, source: &VideoSource) -> Result<VideoSource, PersistenceError> {
        let conn = self.lock_conn()?;
        let created_at_str = source.created_at.to_rfc3339();
        let status_str = source.status.to_string();

        if let Some(id) = source.id {
            conn.execute(
                r#"
                UPDATE videos SET
                    url = ?1,
                    video_id = ?2,
                    title = ?3,
                    author = ?4,
                    duration_seconds = ?5,
                    transcript_path = ?6,
                    local_video_path = ?7,
                    status = ?8,
                    error_message = ?9
                WHERE id = ?10
                "#,
                params![
                    source.url,
                    source.video_id,
                    source.title,
                    source.author,
                    source.duration_seconds,
                    source.transcript_path,
                    source.local_video_path,
                    status_str,
                    source.error_message,
                    id
                ],
            )?;
            Ok(source.clone())
        } else {
            conn.execute(
                r#"
                INSERT INTO videos (
                    url, video_id, title, author, duration_seconds,
                    transcript_path, local_video_path, status, error_message, created_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                "#,
                params![
                    source.url,
                    source.video_id,
                    source.title,
                    source.author,
                    source.duration_seconds,
                    source.transcript_path,
                    source.local_video_path,
                    status_str,
                    source.error_message,
                    created_at_str
                ],
            )?;
            let new_id = conn.last_insert_rowid();
            let mut saved = source.clone();
            saved.id = Some(new_id);
            Ok(saved)
        }
    }

    pub fn find_video_by_id(&self, video_id: i64) -> Result<Option<VideoSource>, PersistenceError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT id, url, video_id, title, author, duration_seconds,
                   transcript_path, local_video_path, status, error_message, created_at
            FROM videos WHERE id = ?1
            "#,
        )?;

        let mut rows = stmt.query(params![video_id])?;
        if let Some(row) = rows.next()? {
            let status_str: String = row.get(8)?;
            let created_at_str: String = row.get(10)?;
            let status = ProcessingStatus::from_str(&status_str).unwrap_or(ProcessingStatus::Pending);
            let created_at = DateTime::parse_from_rfc3339(&created_at_str)
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            Ok(Some(VideoSource {
                id: Some(row.get(0)?),
                url: row.get(1)?,
                video_id: row.get(2)?,
                title: row.get(3)?,
                author: row.get(4)?,
                duration_seconds: row.get(5)?,
                transcript_path: row.get(6)?,
                local_video_path: row.get(7)?,
                status,
                error_message: row.get(9)?,
                created_at,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn find_all_videos(&self) -> Result<Vec<VideoSource>, PersistenceError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT id, url, video_id, title, author, duration_seconds,
                   transcript_path, local_video_path, status, error_message, created_at
            FROM videos ORDER BY created_at DESC
            "#,
        )?;

        let rows = stmt.query_map([], |row| {
            let status_str: String = row.get(8)?;
            let created_at_str: String = row.get(10)?;
            let status = ProcessingStatus::from_str(&status_str).unwrap_or(ProcessingStatus::Pending);
            let created_at = DateTime::parse_from_rfc3339(&created_at_str)
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            Ok(VideoSource {
                id: Some(row.get(0)?),
                url: row.get(1)?,
                video_id: row.get(2)?,
                title: row.get(3)?,
                author: row.get(4)?,
                duration_seconds: row.get(5)?,
                transcript_path: row.get(6)?,
                local_video_path: row.get(7)?,
                status,
                error_message: row.get(9)?,
                created_at,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn update_video_status(
        &self,
        video_id: i64,
        status: ProcessingStatus,
        error_message: Option<&str>,
    ) -> Result<(), PersistenceError> {
        let conn = self.lock_conn()?;
        let status_str = status.to_string();
        conn.execute(
            "UPDATE videos SET status = ?1, error_message = ?2 WHERE id = ?3",
            params![status_str, error_message, video_id],
        )?;
        Ok(())
    }

    pub fn delete_video(&self, video_id: i64) -> Result<(), PersistenceError> {
        let conn = self.lock_conn()?;
        // Deleta em cascata na tabela de clipes e em seguida remove o vídeo
        conn.execute("DELETE FROM clips WHERE video_source_id = ?1", params![video_id])?;
        conn.execute("DELETE FROM videos WHERE id = ?1", params![video_id])?;
        Ok(())
    }

    // =========================================================================
    // ClipCandidate CRUD
    // =========================================================================

    pub fn save_clip(&self, candidate: &ClipCandidate) -> Result<ClipCandidate, PersistenceError> {
        let video_source_id = candidate.video_source_id.unwrap_or(0);
        let list = self.save_clips(video_source_id, std::slice::from_ref(candidate))?;
        list.into_iter()
            .next()
            .ok_or_else(|| PersistenceError::NotFound("Falha ao salvar clipe".to_string()))
    }

    pub fn save_clips(
        &self,
        video_source_id: i64,
        candidates: &[ClipCandidate],
    ) -> Result<Vec<ClipCandidate>, PersistenceError> {
        let mut conn = self.lock_conn()?;
        let tx = conn.transaction()?;

        let mut saved_list = Vec::with_capacity(candidates.len());
        let now_str = Utc::now().to_rfc3339();

        for candidate in candidates {
            let hashtags_json = serde_json::to_string(&candidate.hashtags)?;
            let scores_json = serde_json::to_string(&candidate.scores)?;
            let orientation_str = serde_json::to_string(&candidate.orientation)?.replace('"', "");
            let framing_str = serde_json::to_string(&candidate.framing_mode)?.replace('"', "");
            let preset_str = serde_json::to_string(&candidate.subtitle_preset)?.replace('"', "");
            let emotion_str = serde_json::to_string(&candidate.emotion_tag)?.replace('"', "");
            let status_str = serde_json::to_string(&candidate.status)?.replace('"', "");

            if let Some(id) = candidate.id {
                tx.execute(
                    r#"
                    UPDATE clips SET
                        video_source_id = ?1,
                        title = ?2,
                        hook = ?3,
                        start_seconds = ?4,
                        end_seconds = ?5,
                        virality_score = ?6,
                        reasoning = ?7,
                        suggested_caption = ?8,
                        hashtags_json = ?9,
                        orientation = ?10,
                        has_subtitles = ?11,
                        has_emojis = ?12,
                        output_file_path = ?13,
                        metadata_json_path = ?14,
                        status = ?15,
                        thumbnail_timestamp = ?16,
                        thumbnail_headline = ?17,
                        thumbnail_path = ?18,
                        raw_frame_path = ?19,
                        framing_mode = ?20,
                        subtitle_preset = ?21,
                        emotion_tag = ?22,
                        scores_json = ?23
                    WHERE id = ?24
                    "#,
                    params![
                        video_source_id,
                        candidate.title,
                        candidate.hook,
                        candidate.start_seconds,
                        candidate.end_seconds,
                        candidate.virality_score,
                        candidate.reasoning,
                        candidate.suggested_caption,
                        hashtags_json,
                        orientation_str,
                        if candidate.has_subtitles { 1 } else { 0 },
                        if candidate.has_emojis { 1 } else { 0 },
                        candidate.output_file_path,
                        candidate.metadata_json_path,
                        status_str,
                        candidate.thumbnail_timestamp,
                        candidate.thumbnail_headline,
                        candidate.thumbnail_path,
                        candidate.raw_frame_path,
                        framing_str,
                        preset_str,
                        emotion_str,
                        scores_json,
                        id
                    ],
                )?;
                let mut saved = candidate.clone();
                saved.video_source_id = Some(video_source_id);
                saved_list.push(saved);
            } else {
                tx.execute(
                    r#"
                    INSERT INTO clips (
                        video_source_id, title, hook, start_seconds, end_seconds,
                        virality_score, reasoning, suggested_caption, hashtags_json,
                        orientation, has_subtitles, has_emojis, output_file_path,
                        metadata_json_path, status, thumbnail_timestamp, thumbnail_headline,
                        thumbnail_path, raw_frame_path, framing_mode, subtitle_preset,
                        emotion_tag, scores_json, created_at
                    ) VALUES (
                        ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13,
                        ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24
                    )
                    "#,
                    params![
                        video_source_id,
                        candidate.title,
                        candidate.hook,
                        candidate.start_seconds,
                        candidate.end_seconds,
                        candidate.virality_score,
                        candidate.reasoning,
                        candidate.suggested_caption,
                        hashtags_json,
                        orientation_str,
                        if candidate.has_subtitles { 1 } else { 0 },
                        if candidate.has_emojis { 1 } else { 0 },
                        candidate.output_file_path,
                        candidate.metadata_json_path,
                        status_str,
                        candidate.thumbnail_timestamp,
                        candidate.thumbnail_headline,
                        candidate.thumbnail_path,
                        candidate.raw_frame_path,
                        framing_str,
                        preset_str,
                        emotion_str,
                        scores_json,
                        now_str
                    ],
                )?;
                let new_id = tx.last_insert_rowid();
                let mut saved = candidate.clone();
                saved.id = Some(new_id);
                saved.video_source_id = Some(video_source_id);
                saved_list.push(saved);
            }
        }

        tx.commit()?;
        Ok(saved_list)
    }

    pub fn find_clip_by_id(&self, clip_id: i64) -> Result<Option<ClipCandidate>, PersistenceError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT id, video_source_id, title, hook, start_seconds, end_seconds,
                   virality_score, reasoning, suggested_caption, hashtags_json,
                   orientation, has_subtitles, has_emojis, output_file_path,
                   metadata_json_path, status, thumbnail_timestamp, thumbnail_headline,
                   thumbnail_path, raw_frame_path, framing_mode, subtitle_preset,
                   emotion_tag, scores_json
            FROM clips WHERE id = ?1
            "#,
        )?;

        let mut rows = stmt.query(params![clip_id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(map_clip_row(row)?))
        } else {
            Ok(None)
        }
    }

    pub fn find_clips_by_video_id(
        &self,
        video_source_id: i64,
    ) -> Result<Vec<ClipCandidate>, PersistenceError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT id, video_source_id, title, hook, start_seconds, end_seconds,
                   virality_score, reasoning, suggested_caption, hashtags_json,
                   orientation, has_subtitles, has_emojis, output_file_path,
                   metadata_json_path, status, thumbnail_timestamp, thumbnail_headline,
                   thumbnail_path, raw_frame_path, framing_mode, subtitle_preset,
                   emotion_tag, scores_json
            FROM clips WHERE video_source_id = ?1 ORDER BY id ASC
            "#,
        )?;

        let rows = stmt.query_map(params![video_source_id], map_clip_row)?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn update_clip_time_bounds(
        &self,
        clip_id: i64,
        start_seconds: f64,
        end_seconds: f64,
    ) -> Result<(), PersistenceError> {
        let conn = self.lock_conn()?;
        conn.execute(
            "UPDATE clips SET start_seconds = ?1, end_seconds = ?2 WHERE id = ?3",
            params![start_seconds, end_seconds, clip_id],
        )?;
        Ok(())
    }

    pub fn update_clip_output(
        &self,
        clip_id: i64,
        output_path: Option<&str>,
        metadata_path: Option<&str>,
        status: ProcessingStatus,
    ) -> Result<(), PersistenceError> {
        let conn = self.lock_conn()?;
        let status_str = serde_json::to_string(&status)?.replace('"', "");
        conn.execute(
            "UPDATE clips SET output_file_path = ?1, metadata_json_path = ?2, status = ?3 WHERE id = ?4",
            params![output_path, metadata_path, status_str, clip_id],
        )?;
        Ok(())
    }

    pub fn update_clip_thumbnail(
        &self,
        clip_id: i64,
        timestamp: Option<f64>,
        headline: Option<&str>,
        thumbnail_path: Option<&str>,
        raw_frame_path: Option<&str>,
    ) -> Result<(), PersistenceError> {
        let conn = self.lock_conn()?;
        conn.execute(
            r#"
            UPDATE clips SET
                thumbnail_timestamp = COALESCE(?1, thumbnail_timestamp),
                thumbnail_headline = COALESCE(?2, thumbnail_headline),
                thumbnail_path = COALESCE(?3, thumbnail_path),
                raw_frame_path = COALESCE(?4, raw_frame_path)
            WHERE id = ?5
            "#,
            params![timestamp, headline, thumbnail_path, raw_frame_path, clip_id],
        )?;
        Ok(())
    }

    pub fn update_clip_framing_and_subtitle(
        &self,
        clip_id: i64,
        framing_mode: FramingMode,
        subtitle_preset: SubtitlePreset,
    ) -> Result<(), PersistenceError> {
        let conn = self.lock_conn()?;
        let orientation = framing_mode.to_orientation_mode();
        let orientation_str = serde_json::to_string(&orientation)?.replace('"', "");
        let framing_str = serde_json::to_string(&framing_mode)?.replace('"', "");
        let preset_str = serde_json::to_string(&subtitle_preset)?.replace('"', "");

        conn.execute(
            r#"
            UPDATE clips SET
                framing_mode = ?1,
                orientation = ?2,
                subtitle_preset = ?3
            WHERE id = ?4
            "#,
            params![framing_str, orientation_str, preset_str, clip_id],
        )?;
        Ok(())
    }

    pub fn update_clip_subtitle_and_emojis(
        &self,
        clip_id: i64,
        subtitle_preset: Option<SubtitlePreset>,
        has_emojis: Option<bool>,
    ) -> Result<(), PersistenceError> {
        let conn = self.lock_conn()?;
        if let Some(preset) = subtitle_preset {
            let preset_str = serde_json::to_string(&preset)?.replace('"', "");
            conn.execute(
                "UPDATE clips SET subtitle_preset = ?1 WHERE id = ?2",
                params![preset_str, clip_id],
            )?;
        }
        if let Some(emojis) = has_emojis {
            conn.execute(
                "UPDATE clips SET has_emojis = ?1 WHERE id = ?2",
                params![if emojis { 1 } else { 0 }, clip_id],
            )?;
        }
        Ok(())
    }

    pub fn update_clip_emotion_and_scores(
        &self,
        clip_id: i64,
        emotion_tag: Option<EmotionTag>,
        scores: Option<ScoreBreakdown>,
    ) -> Result<(), PersistenceError> {
        let conn = self.lock_conn()?;
        if let Some(tag) = emotion_tag {
            let tag_str = serde_json::to_string(&tag)?.replace('"', "");
            conn.execute(
                "UPDATE clips SET emotion_tag = ?1 WHERE id = ?2",
                params![tag_str, clip_id],
            )?;
        }
        if let Some(sc) = scores {
            let sc_json = serde_json::to_string(&sc)?;
            let virality = sc.calculate_weighted_virality_score();
            conn.execute(
                "UPDATE clips SET scores_json = ?1, virality_score = ?2 WHERE id = ?3",
                params![sc_json, virality, clip_id],
            )?;
        }
        Ok(())
    }

    pub fn delete_clip(&self, clip_id: i64) -> Result<(), PersistenceError> {
        let conn = self.lock_conn()?;
        conn.execute("DELETE FROM clips WHERE id = ?1", params![clip_id])?;
        Ok(())
    }

    // =========================================================================
    // AppSettings CRUD
    // =========================================================================

    pub fn get_settings(&self) -> Result<AppSettings, PersistenceError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT output_directory, min_virality_score, auto_process, default_orientation,
                   auto_vertical_max_duration, burn_subtitles, cpu_threads, gemini_api_key,
                   gemini_model, ai_provider, openai_api_key, openai_model, youtube_client_id,
                   youtube_client_secret, youtube_access_token, youtube_refresh_token,
                   youtube_token_expires_at, youtube_channel_title, youtube_channel_id,
                   youtube_channel_avatar_url
            FROM settings WHERE id = 1
            "#,
        )?;

        let mut rows = stmt.query([])?;
        if let Some(row) = rows.next()? {
            let default_orient_str: String = row.get(3)?;
            let ai_prov_str: String = row.get(9)?;

            let orientation = OrientationMode::from_str(&default_orient_str).unwrap_or(OrientationMode::Auto);
            let provider = AiProvider::from_str(&ai_prov_str).unwrap_or(AiProvider::Gemini);

            let mut settings = AppSettings {
                output_directory: row.get(0)?,
                min_virality_score: row.get(1)?,
                auto_process_above_threshold: row.get::<_, i32>(2)? != 0,
                default_orientation: orientation,
                auto_vertical_max_duration_seconds: row.get(4)?,
                burn_subtitles: row.get::<_, i32>(5)? != 0,
                cpu_threads: row.get(6)?,
                gemini_api_key: row.get(7)?,
                gemini_model: row.get(8)?,
                ai_provider: provider,
                openai_api_key: row.get(10)?,
                openai_model: row.get(11)?,
                youtube_client_id: row.get(12)?,
                youtube_client_secret: row.get(13)?,
                youtube_access_token: row.get(14)?,
                youtube_refresh_token: row.get(15)?,
                youtube_token_expires_at: row.get(16)?,
                youtube_channel_title: row.get(17)?,
                youtube_channel_id: row.get(18)?,
                youtube_channel_avatar_url: row.get(19)?,
            };

            // Auto-resolução como na paridade original com Java
            let mut needs_update = false;
            if settings.gemini_api_key.trim().is_empty() {
                let resolved = AppSettings::resolve_default_api_key();
                if !resolved.is_empty() {
                    settings.gemini_api_key = resolved;
                    needs_update = true;
                }
            }
            if settings.openai_api_key.trim().is_empty() {
                let resolved = AppSettings::resolve_default_openai_api_key();
                if !resolved.is_empty() {
                    settings.openai_api_key = resolved;
                    needs_update = true;
                }
            }
            if settings.youtube_client_id.trim().is_empty() {
                let resolved = AppSettings::resolve_default_youtube_client_id();
                if !resolved.is_empty() {
                    settings.youtube_client_id = resolved;
                    needs_update = true;
                }
            }
            if settings.youtube_client_secret.trim().is_empty() {
                let resolved = AppSettings::resolve_default_youtube_client_secret();
                if !resolved.is_empty() {
                    settings.youtube_client_secret = resolved;
                    needs_update = true;
                }
            }

            if needs_update {
                drop(rows);
                drop(stmt);
                drop(conn);
                self.save_settings(&settings)?;
            }

            Ok(settings)
        } else {
            drop(rows);
            drop(stmt);
            drop(conn);
            let defaults = AppSettings::default_settings();
            self.save_settings(&defaults)?;
            Ok(defaults)
        }
    }

    pub fn save_settings(&self, settings: &AppSettings) -> Result<(), PersistenceError> {
        let conn = self.lock_conn()?;
        let orientation_str = serde_json::to_string(&settings.default_orientation)?.replace('"', "");
        let provider_str = serde_json::to_string(&settings.ai_provider)?.replace('"', "");

        conn.execute(
            r#"
            INSERT INTO settings (
                id, output_directory, min_virality_score, auto_process, default_orientation,
                auto_vertical_max_duration, burn_subtitles, cpu_threads, gemini_api_key,
                gemini_model, ai_provider, openai_api_key, openai_model, youtube_client_id,
                youtube_client_secret, youtube_access_token, youtube_refresh_token,
                youtube_token_expires_at, youtube_channel_title, youtube_channel_id,
                youtube_channel_avatar_url
            ) VALUES (
                1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15,
                ?16, ?17, ?18, ?19, ?20
            )
            ON CONFLICT(id) DO UPDATE SET
                output_directory = excluded.output_directory,
                min_virality_score = excluded.min_virality_score,
                auto_process = excluded.auto_process,
                default_orientation = excluded.default_orientation,
                auto_vertical_max_duration = excluded.auto_vertical_max_duration,
                burn_subtitles = excluded.burn_subtitles,
                cpu_threads = excluded.cpu_threads,
                gemini_api_key = excluded.gemini_api_key,
                gemini_model = excluded.gemini_model,
                ai_provider = excluded.ai_provider,
                openai_api_key = excluded.openai_api_key,
                openai_model = excluded.openai_model,
                youtube_client_id = excluded.youtube_client_id,
                youtube_client_secret = excluded.youtube_client_secret,
                youtube_access_token = excluded.youtube_access_token,
                youtube_refresh_token = excluded.youtube_refresh_token,
                youtube_token_expires_at = excluded.youtube_token_expires_at,
                youtube_channel_title = excluded.youtube_channel_title,
                youtube_channel_id = excluded.youtube_channel_id,
                youtube_channel_avatar_url = excluded.youtube_channel_avatar_url
            "#,
            params![
                settings.output_directory,
                settings.min_virality_score,
                if settings.auto_process_above_threshold { 1 } else { 0 },
                orientation_str,
                settings.auto_vertical_max_duration_seconds,
                if settings.burn_subtitles { 1 } else { 0 },
                settings.cpu_threads,
                settings.gemini_api_key,
                settings.gemini_model,
                provider_str,
                settings.openai_api_key,
                settings.openai_model,
                settings.youtube_client_id,
                settings.youtube_client_secret,
                settings.youtube_access_token,
                settings.youtube_refresh_token,
                settings.youtube_token_expires_at,
                settings.youtube_channel_title,
                settings.youtube_channel_id,
                settings.youtube_channel_avatar_url,
            ],
        )?;
        Ok(())
    }
}

fn map_clip_row(row: &rusqlite::Row<'_>) -> Result<ClipCandidate, rusqlite::Error> {
    let hashtags_str: String = row.get(9)?;
    let orientation_str: String = row.get(10)?;
    let status_str: String = row.get(15)?;
    let framing_str: String = row.get(20)?;
    let preset_str: String = row.get(21)?;
    let emotion_str: String = row.get(22)?;
    let scores_str: String = row.get(23)?;

    let virality_score: i32 = row.get(6)?;
    let hashtags: Vec<String> = serde_json::from_str(&hashtags_str).unwrap_or_default();
    let orientation = OrientationMode::from_str(&orientation_str).unwrap_or(OrientationMode::Auto);
    let status = ProcessingStatus::from_str(&status_str).unwrap_or(ProcessingStatus::Pending);
    let framing_mode = FramingMode::from_str_loose(&framing_str, FramingMode::AutoCrop);
    let subtitle_preset = SubtitlePreset::from_str_loose(&preset_str, SubtitlePreset::HormoziYellow);
    let emotion_tag = EmotionTag::from_str_loose(&emotion_str, EmotionTag::AltaEnergia);
    let scores = serde_json::from_str::<ScoreBreakdown>(&scores_str)
        .unwrap_or_else(|_| ScoreBreakdown::from_uniform(virality_score));

    Ok(ClipCandidate {
        id: Some(row.get(0)?),
        video_source_id: Some(row.get(1)?),
        title: row.get(2)?,
        hook: row.get(3)?,
        start_seconds: row.get(4)?,
        end_seconds: row.get(5)?,
        virality_score,
        reasoning: row.get(7)?,
        suggested_caption: row.get(8)?,
        hashtags,
        orientation,
        has_subtitles: row.get::<_, i32>(11)? != 0,
        has_emojis: row.get::<_, i32>(12)? != 0,
        output_file_path: row.get(13)?,
        metadata_json_path: row.get(14)?,
        status,
        thumbnail_timestamp: row.get(16)?,
        thumbnail_headline: row.get(17)?,
        thumbnail_path: row.get(18)?,
        raw_frame_path: row.get(19)?,
        framing_mode,
        subtitle_preset,
        emotion_tag,
        scores,
    })
}
