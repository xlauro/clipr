use clipr_domain::prelude::*;
use clipr_persistence::prelude::*;

#[test]
fn should_delete_video_with_cascade_deletion_of_child_clips() {
    let db = DatabaseManager::open_in_memory().expect("Falha ao abrir banco em memória");

    // 1. Cria e persiste vídeo pai
    let parent_video = db
        .save_video(&VideoSource::from_url("https://youtube.com/watch?v=orphan_test_video"))
        .expect("Deveria salvar vídeo");
    let video_id = parent_video.id.expect("Vídeo deve ter ID gerado");
    assert!(video_id > 0);

    // 2. Cria múltiplos cortes filhos
    let c1 = create_test_clip(video_id, "Corte Filho 1", 10.0, 35.0, 94);
    let c2 = create_test_clip(video_id, "Corte Filho 2", 40.0, 75.0, 89);

    let saved_clips = db.save_clips(video_id, &[c1, c2]).expect("Deveria salvar clipes");
    assert_eq!(saved_clips.len(), 2);

    let clip_id1 = saved_clips[0].id.expect("Clipe 1 deve ter ID");
    let clip_id2 = saved_clips[1].id.expect("Clipe 2 deve ter ID");

    // Comprova que vídeo e clipes existem antes da remoção
    assert!(db.find_video_by_id(video_id).unwrap().is_some());
    assert!(db.find_clip_by_id(clip_id1).unwrap().is_some());
    assert!(db.find_clip_by_id(clip_id2).unwrap().is_some());
    assert_eq!(db.find_clips_by_video_id(video_id).unwrap().len(), 2);

    // 3. Executa a deleção em cascata do vídeo
    db.delete_video(video_id).expect("Deveria deletar vídeo pai");

    // 4. Comprova remoção do vídeo pai e clipes filhos
    assert!(db.find_video_by_id(video_id).unwrap().is_none());
    assert!(db.find_clip_by_id(clip_id1).unwrap().is_none());
    assert!(db.find_clip_by_id(clip_id2).unwrap().is_none());
    assert!(db.find_clips_by_video_id(video_id).unwrap().is_empty());
}

#[test]
fn should_delete_individual_clip_without_affecting_parent_or_sibling() {
    let db = DatabaseManager::open_in_memory().unwrap();
    let video = db
        .save_video(&VideoSource::from_url("https://youtube.com/watch?v=single_delete_test"))
        .unwrap();
    let video_id = video.id.unwrap();

    let clip_a = create_test_clip(video_id, "Clipe A - Para Deletar", 0.0, 20.0, 91);
    let clip_b = create_test_clip(video_id, "Clipe B - Para Permanecer", 25.0, 55.0, 96);

    let saved = db.save_clips(video_id, &[clip_a, clip_b]).unwrap();
    let id_a = saved[0].id.unwrap();
    let id_b = saved[1].id.unwrap();

    db.delete_clip(id_a).expect("Deveria deletar clipe A");

    assert!(db.find_clip_by_id(id_a).unwrap().is_none());

    let remaining_clip = db.find_clip_by_id(id_b).unwrap();
    assert!(remaining_clip.is_some());
    assert_eq!(remaining_clip.unwrap().title, "Clipe B - Para Permanecer");

    assert!(db.find_video_by_id(video_id).unwrap().is_some());
    let clips_left = db.find_clips_by_video_id(video_id).unwrap();
    assert_eq!(clips_left.len(), 1);
    assert_eq!(clips_left[0].id, Some(id_b));
}

#[test]
fn should_update_clip_framing_and_subtitle_configuration() {
    let db = DatabaseManager::open_in_memory().unwrap();
    let video = db
        .save_video(&VideoSource::from_url("https://youtube.com/watch?v=framing_test"))
        .unwrap();
    let video_id = video.id.unwrap();

    let initial = create_test_clip(video_id, "Clipe Enquadramento", 15.0, 45.0, 92);
    let saved = db.save_clip(&initial).unwrap();
    let clip_id = saved.id.unwrap();

    // 1. Atualiza para Split-Screen e Cyber Green
    db.update_clip_framing_and_subtitle(clip_id, FramingMode::SplitScreen, SubtitlePreset::CyberGreen)
        .unwrap();

    let after_first = db.find_clip_by_id(clip_id).unwrap().unwrap();
    assert_eq!(after_first.framing_mode, FramingMode::SplitScreen);
    assert_eq!(after_first.orientation, OrientationMode::Vertical916);
    assert_eq!(after_first.subtitle_preset, SubtitlePreset::CyberGreen);

    // 2. Atualiza para Horizontal 16:9 e Clean White
    db.update_clip_framing_and_subtitle(clip_id, FramingMode::Horizontal169, SubtitlePreset::CleanWhite)
        .unwrap();

    let after_second = db.find_clip_by_id(clip_id).unwrap().unwrap();
    assert_eq!(after_second.framing_mode, FramingMode::Horizontal169);
    assert_eq!(after_second.orientation, OrientationMode::Horizontal169);
    assert_eq!(after_second.subtitle_preset, SubtitlePreset::CleanWhite);

    // 3. Atualiza status de saída e bounds de tempo
    db.update_clip_output(
        clip_id,
        Some("/exports/rendered_clip.mp4"),
        Some("/exports/rendered_clip.json"),
        ProcessingStatus::Completed,
    )
    .unwrap();
    db.update_clip_time_bounds(clip_id, 14.5, 46.0).unwrap();

    let fully_updated = db.find_clip_by_id(clip_id).unwrap().unwrap();
    assert_eq!(fully_updated.output_file_path.as_deref(), Some("/exports/rendered_clip.mp4"));
    assert_eq!(fully_updated.metadata_json_path.as_deref(), Some("/exports/rendered_clip.json"));
    assert_eq!(fully_updated.status, ProcessingStatus::Completed);
    assert_eq!(fully_updated.start_seconds, 14.5);
    assert_eq!(fully_updated.end_seconds, 46.0);
}

#[test]
fn should_persist_and_retrieve_clip_with_emotion_tag_and_score_breakdown() {
    let db = DatabaseManager::open_in_memory().unwrap();
    let video = db
        .save_video(&VideoSource::from_url("https://youtube.com/watch?v=emotion_test"))
        .unwrap();
    let video_id = video.id.unwrap();

    let initial_scores = ScoreBreakdown::new(90, 85, 80, 75, 70, 65);
    let mut clip = create_test_clip(video_id, "Clipe Emocional", 12.0, 42.0, initial_scores.calculate_weighted_virality_score());
    clip.emotion_tag = EmotionTag::TensaoClimax;
    clip.scores = initial_scores;

    let saved = db.save_clip(&clip).unwrap();
    let clip_id = saved.id.unwrap();
    assert_eq!(saved.emotion_tag, EmotionTag::TensaoClimax);
    assert_eq!(saved.scores, initial_scores);

    // Recupera do SQLite
    let loaded = db.find_clip_by_id(clip_id).unwrap().unwrap();
    assert_eq!(loaded.emotion_tag, EmotionTag::TensaoClimax);
    assert_eq!(loaded.scores, initial_scores);
    assert_eq!(loaded.virality_score, initial_scores.calculate_weighted_virality_score());

    // Atualiza emotionTag e scores
    let new_scores = ScoreBreakdown::new(95, 95, 90, 85, 80, 90);
    db.update_clip_emotion_and_scores(clip_id, Some(EmotionTag::InsightChocante), Some(new_scores))
        .unwrap();

    let updated = db.find_clip_by_id(clip_id).unwrap().unwrap();
    assert_eq!(updated.emotion_tag, EmotionTag::InsightChocante);
    assert_eq!(updated.scores, new_scores);
    assert_eq!(updated.virality_score, new_scores.calculate_weighted_virality_score());
}

#[test]
fn should_update_and_retrieve_app_settings() {
    let db = DatabaseManager::open_in_memory().unwrap();

    let custom_settings = AppSettings {
        output_directory: "/custom/sqlite/exports".to_string(),
        min_virality_score: 91,
        auto_process_above_threshold: true,
        default_orientation: OrientationMode::Vertical916,
        auto_vertical_max_duration_seconds: 60.0,
        burn_subtitles: true,
        cpu_threads: 8,
        gemini_api_key: "AIzaSyTestOperations-ApiKey-Initial-12345".to_string(),
        gemini_model: "gemini-3.8-flash".to_string(),
        ai_provider: AiProvider::OpenAi,
        openai_api_key: "sk-openai-custom-key-1".to_string(),
        openai_model: "gpt-4o-mini".to_string(),
        youtube_client_id: "yt-client-id".to_string(),
        youtube_client_secret: "yt-secret".to_string(),
        youtube_access_token: "access".to_string(),
        youtube_refresh_token: "refresh".to_string(),
        youtube_token_expires_at: 1800000000,
        youtube_channel_title: "Canal Clippr".to_string(),
        youtube_channel_id: "UC999".to_string(),
        youtube_channel_avatar_url: "https://avatar.png".to_string(),
    };

    db.save_settings(&custom_settings).unwrap();

    let retrieved = db.get_settings().unwrap();
    assert_eq!(retrieved.output_directory, "/custom/sqlite/exports");
    assert_eq!(retrieved.min_virality_score, 91);
    assert!(retrieved.auto_process_above_threshold);
    assert_eq!(retrieved.default_orientation, OrientationMode::Vertical916);
    assert_eq!(retrieved.ai_provider, AiProvider::OpenAi);
    assert_eq!(retrieved.gemini_api_key, "AIzaSyTestOperations-ApiKey-Initial-12345");
    assert_eq!(retrieved.openai_api_key, "sk-openai-custom-key-1");
    assert_eq!(retrieved.youtube_channel_title, "Canal Clippr");
    assert!(retrieved.is_youtube_connected());
}

#[test]
fn should_support_concurrent_access_safely() {
    let db = DatabaseManager::open_in_memory().unwrap();
    let video = db
        .save_video(&VideoSource::from_url("https://youtube.com/watch?v=concurrent_test"))
        .unwrap();
    let video_id = video.id.unwrap();

    let mut handles = vec![];
    for i in 0..10 {
        let db_clone = db.clone();
        handles.push(std::thread::spawn(move || {
            let clip = create_test_clip(video_id, &format!("Clip Concorrente {}", i), i as f64 * 5.0, (i + 1) as f64 * 5.0, 90 + i);
            db_clone.save_clip(&clip).unwrap();
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    let clips = db.find_clips_by_video_id(video_id).unwrap();
    assert_eq!(clips.len(), 10);
}

fn create_test_clip(video_source_id: i64, title: &str, start: f64, end: f64, score: i32) -> ClipCandidate {
    ClipCandidate::new(
        None,
        Some(video_source_id),
        title,
        "Hook de teste",
        start,
        end,
        score,
        "Motivo do clipe",
        "Legenda do clipe",
        vec!["#teste".to_string(), "#clippr".to_string()],
        OrientationMode::Vertical916,
        true,
        true,
        None,
        None,
        ProcessingStatus::PendingApproval,
        None,
        None,
        None,
        None,
        Some(FramingMode::AutoCrop),
        Some(SubtitlePreset::HormoziYellow),
        Some(EmotionTag::AltaEnergia),
        None,
    )
}
