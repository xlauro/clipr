use clipr_domain::prelude::*;

#[test]
fn should_validate_clip_candidate_duration_invariants() {
    let valid = create_sample_clip(0.0, 30.0);
    assert!(valid.has_valid_bounds());
    assert!((valid.duration_seconds() - 30.0).abs() < 1e-4);

    let valid_frac = create_sample_clip(10.25, 45.75);
    assert!(valid_frac.has_valid_bounds());
    assert!((valid_frac.duration_seconds() - 35.5).abs() < 1e-4);

    let negative_start = create_sample_clip(-5.0, 25.0);
    assert!(!negative_start.has_valid_bounds());
    assert!((negative_start.duration_seconds() - 30.0).abs() < 1e-4);

    let equal_bounds = create_sample_clip(15.0, 15.0);
    assert!(!equal_bounds.has_valid_bounds());
    assert_eq!(equal_bounds.duration_seconds(), 0.0);

    let inverted = create_sample_clip(40.0, 10.0);
    assert!(!inverted.has_valid_bounds());
    assert_eq!(inverted.duration_seconds(), 0.0);
}

#[test]
fn should_format_duration_string_correctly() {
    let test_cases = [
        (0.0, 45.0, "45s"),
        (0.0, 59.4, "59s"),
        (0.0, 60.0, "01m 00s"),
        (10.0, 145.0, "02m 15s"),
        (0.0, 3599.0, "59m 59s"),
        (0.0, 3600.0, "01h 00m 00s"),
        (100.0, 4012.0, "01h 05m 12s"),
    ];

    for (start, end, expected) in test_cases {
        let clip = create_sample_clip(start, end);
        assert_eq!(clip.formatted_duration(), expected);
    }
}

#[test]
fn should_format_time_ranges_correctly() {
    let short_range = create_sample_clip(65.0, 185.0);
    assert_eq!(short_range.formatted_time_range(), "01:05 - 03:05");
    assert_eq!(
        short_range.formatted_display_interval(),
        "01:05 - 03:05 (02m 00s)"
    );

    let hour_range = create_sample_clip(3665.0, 7325.0);
    assert_eq!(hour_range.formatted_time_range(), "01:01:05 - 02:02:05");
    assert_eq!(
        hour_range.formatted_display_interval(),
        "01:01:05 - 02:02:05 (01h 01m 00s)"
    );
}

#[test]
fn should_classify_shorts_vs_long_clips() {
    let viral_short = create_sample_clip(0.0, 15.0);
    assert!(!viral_short.is_long_clip());
    assert_eq!(viral_short.clip_type_badge(), "⚡ Curto");

    let boundary_clip = create_sample_clip(0.0, 90.0);
    assert!(!boundary_clip.is_long_clip());
    assert_eq!(boundary_clip.clip_type_badge(), "⚡ Curto");

    let long_clip = create_sample_clip(0.0, 90.1);
    assert!(long_clip.is_long_clip());
    assert_eq!(long_clip.clip_type_badge(), "🎬 Longo");

    let podcast_excerpt = create_sample_clip(100.0, 350.0);
    assert!(podcast_excerpt.is_long_clip());
    assert_eq!(podcast_excerpt.clip_type_badge(), "🎬 Longo");
}

#[test]
fn should_preserve_attributes_in_with_bounds() {
    let base = ClipCandidate::new(
        Some(42),
        Some(7),
        "Título Incrível",
        "Gancho Forte",
        10.0,
        40.0,
        96,
        "Excelente raciocínio",
        "Legenda sugerida",
        vec!["#viral".to_string(), "#ai".to_string()],
        OrientationMode::Vertical916,
        true,
        true,
        Some("/out/clip.mp4".to_string()),
        Some("/out/meta.json".to_string()),
        ProcessingStatus::PendingApproval,
        Some(25.0),
        Some("HEADLINE".to_string()),
        Some("/out/thumb.jpg".to_string()),
        Some("/out/raw.jpg".to_string()),
        Some(FramingMode::SplitScreen),
        Some(SubtitlePreset::CyberGreen),
        Some(EmotionTag::AltaEnergia),
        None,
    );

    let modified = base.with_bounds(12.5, 45.0);

    assert_eq!(modified.start_seconds, 12.5);
    assert_eq!(modified.end_seconds, 45.0);
    assert_eq!(modified.duration_seconds(), 32.5);

    assert_eq!(base.id, modified.id);
    assert_eq!(base.video_source_id, modified.video_source_id);
    assert_eq!(base.title, modified.title);
    assert_eq!(base.hook, modified.hook);
    assert_eq!(base.virality_score, modified.virality_score);
    assert_eq!(base.reasoning, modified.reasoning);
    assert_eq!(base.suggested_caption, modified.suggested_caption);
    assert_eq!(base.hashtags, modified.hashtags);
    assert_eq!(base.orientation, modified.orientation);
    assert_eq!(base.has_subtitles, modified.has_subtitles);
    assert_eq!(base.output_file_path, modified.output_file_path);
    assert_eq!(base.metadata_json_path, modified.metadata_json_path);
    assert_eq!(base.status, modified.status);
    assert_eq!(base.thumbnail_timestamp, modified.thumbnail_timestamp);
    assert_eq!(base.thumbnail_headline, modified.thumbnail_headline);
    assert_eq!(base.thumbnail_path, modified.thumbnail_path);
    assert_eq!(base.raw_frame_path, modified.raw_frame_path);
    assert_eq!(base.framing_mode, modified.framing_mode);
    assert_eq!(base.subtitle_preset, modified.subtitle_preset);
    assert_eq!(base.emotion_tag, modified.emotion_tag);
    assert_eq!(base.scores, modified.scores);
}

#[test]
fn should_update_thumbnail_immutably() {
    let original = create_sample_clip(5.0, 35.0);
    assert!(!original.has_thumbnail());

    let with_thumb = original.with_thumbnail(
        Some(12.0),
        Some("ATENÇÃO".to_string()),
        Some("/thumbs/t.jpg".to_string()),
        Some("/thumbs/raw.jpg".to_string()),
    );
    assert!(with_thumb.has_thumbnail());
    assert_eq!(with_thumb.thumbnail_timestamp, Some(12.0));
    assert_eq!(with_thumb.thumbnail_headline.as_deref(), Some("ATENÇÃO"));
    assert_eq!(with_thumb.thumbnail_path.as_deref(), Some("/thumbs/t.jpg"));
    assert_eq!(with_thumb.raw_frame_path.as_deref(), Some("/thumbs/raw.jpg"));

    let thumb_overload = original.with_thumbnail_paths(
        Some("/thumbs/novo.jpg".to_string()),
        Some("/thumbs/novo_raw.jpg".to_string()),
    );
    assert!(thumb_overload.has_thumbnail());
    assert_eq!(
        thumb_overload.thumbnail_path.as_deref(),
        Some("/thumbs/novo.jpg")
    );
}

#[test]
fn should_update_framing_mode_and_orientation() {
    let vertical = create_sample_clip(0.0, 30.0);
    assert_eq!(vertical.framing_mode, FramingMode::AutoCrop);
    assert_eq!(vertical.orientation, OrientationMode::Vertical916);

    let horizontal = vertical.with_framing_mode(Some(FramingMode::Horizontal169));
    assert_eq!(horizontal.framing_mode, FramingMode::Horizontal169);
    assert_eq!(horizontal.orientation, OrientationMode::Horizontal169);

    let split = horizontal.with_framing_mode(Some(FramingMode::SplitScreen));
    assert_eq!(split.framing_mode, FramingMode::SplitScreen);
    assert_eq!(split.orientation, OrientationMode::Vertical916);

    let fallback = split.with_framing_mode(None);
    assert_eq!(fallback.framing_mode, FramingMode::AutoCrop);
    assert_eq!(fallback.orientation, OrientationMode::Vertical916);
}

#[test]
fn should_update_subtitle_preset_safe_fallback() {
    let base = create_sample_clip(0.0, 30.0);
    assert_eq!(base.subtitle_preset, SubtitlePreset::HormoziYellow);

    let green = base.with_subtitle_preset(Some(SubtitlePreset::CyberGreen));
    assert_eq!(green.subtitle_preset, SubtitlePreset::CyberGreen);

    let white = green.with_subtitle_preset(Some(SubtitlePreset::CleanWhite));
    assert_eq!(white.subtitle_preset, SubtitlePreset::CleanWhite);

    let safe_null = white.with_subtitle_preset(None);
    assert_eq!(safe_null.subtitle_preset, SubtitlePreset::HormoziYellow);
}

#[test]
fn should_validate_virality_score() {
    let clip90 = ClipCandidate::new(
        Some(1),
        Some(1),
        "T",
        "H",
        0.0,
        30.0,
        90,
        "R",
        "C",
        vec![],
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
        None,
        None,
        None,
        None,
    );
    assert!(clip90.is_viral_worthy(85));
    assert!(clip90.is_viral_worthy(90));
    assert!(!clip90.is_viral_worthy(91));
}

#[test]
fn should_extract_video_id_from_various_youtube_url_formats() {
    let cases = [
        ("https://youtu.be/dQw4w9WgXcQ", "dQw4w9WgXcQ"),
        ("https://youtu.be/dQw4w9WgXcQ?si=abcdef12345", "dQw4w9WgXcQ"),
        ("https://youtu.be/dQw4w9WgXcQ&feature=share", "dQw4w9WgXcQ"),
        ("https://youtu.be/dQw4w9WgXcQ/", "dQw4w9WgXcQ"),
        ("http://youtu.be/dQw4w9WgXcQ#t=10s", "dQw4w9WgXcQ"),
        ("https://www.youtube.com/shorts/3xyz987Abc", "3xyz987Abc"),
        ("https://www.youtube.com/shorts/3xyz987Abc?feature=share", "3xyz987Abc"),
        ("https://www.youtube.com/shorts/3xyz987Abc/", "3xyz987Abc"),
        ("https://youtube.com/shorts/3xyz987Abc&extra=true", "3xyz987Abc"),
        ("https://www.youtube.com/live/LiveStream123", "LiveStream123"),
        ("https://www.youtube.com/live/LiveStream123?feature=share", "LiveStream123"),
        ("https://www.youtube.com/watch?v=dQw4w9WgXcQ", "dQw4w9WgXcQ"),
        ("https://www.youtube.com/watch?v=dQw4w9WgXcQ&t=30s&ab_channel=RickAstley", "dQw4w9WgXcQ"),
        ("https://www.youtube.com/watch?v=dQw4w9WgXcQ#comments", "dQw4w9WgXcQ"),
        ("   https://youtu.be/dQw4w9WgXcQ   ", "dQw4w9WgXcQ"),
        ("\t https://www.youtube.com/shorts/3xyz987Abc \n", "3xyz987Abc"),
        ("", ""),
        ("   ", ""),
        ("not-a-youtube-url", "not-a-youtube-url"),
        ("not a custom:uri!", "not_a_custom_uri_"),
        ("https://vimeo.com/123456", "https___vimeo_com_123456"),
    ];

    for (url, expected) in cases {
        assert_eq!(VideoSource::extract_video_id(url), expected);
    }
}

#[test]
fn should_validate_youtube_urls() {
    assert!(VideoSource::is_valid_youtube_url("https://www.youtube.com/watch?v=123"));
    assert!(VideoSource::is_valid_youtube_url("https://youtu.be/123"));
    assert!(VideoSource::is_valid_youtube_url("https://youtube.com/shorts/123"));
    assert!(VideoSource::is_valid_youtube_url("https://youtube.com/live/123"));
    assert!(VideoSource::is_valid_youtube_url("http://youtube.com/watch?v=123"));

    assert!(!VideoSource::is_valid_youtube_url("https://vimeo.com/123456"));
    assert!(!VideoSource::is_valid_youtube_url("https://dailymotion.com/video/x123"));
    assert!(!VideoSource::is_valid_youtube_url("not-a-url"));
    assert!(!VideoSource::is_valid_youtube_url(""));
    assert!(!VideoSource::is_valid_youtube_url("   "));
}

#[test]
fn should_create_initial_video_source_properly() {
    let test_url = "https://www.youtube.com/watch?v=abc999xyz";
    let source = VideoSource::from_url(test_url);

    assert_eq!(source.id, None);
    assert_eq!(source.url, test_url);
    assert_eq!(source.video_id, "abc999xyz");
    assert_eq!(source.title, "Aguardando informações...");
    assert_eq!(source.author, "");
    assert_eq!(source.duration_seconds, 0.0);
    assert_eq!(source.transcript_path, None);
    assert_eq!(source.local_video_path, None);
    assert_eq!(source.status, ProcessingStatus::Pending);
    assert_eq!(source.error_message, None);
}

#[test]
fn should_update_video_source_status_immutably() {
    let original = VideoSource::from_url("https://youtu.be/sample");
    let completed = original.with_status(ProcessingStatus::Completed, None);

    assert_eq!(completed.status, ProcessingStatus::Completed);
    assert_eq!(completed.error_message, None);
    assert_eq!(original.status, ProcessingStatus::Pending);

    let failed = original.with_status(ProcessingStatus::Failed, Some("Erro de rede".to_string()));
    assert_eq!(failed.status, ProcessingStatus::Failed);
    assert_eq!(failed.error_message.as_deref(), Some("Erro de rede"));
}

#[test]
fn should_calculate_score_breakdown_weighted_virality() {
    let breakdown = ScoreBreakdown::new(90, 85, 80, 75, 70, 65);
    // 0.25*90 + 0.20*85 + 0.15*80 + 0.15*75 + 0.15*70 + 0.10*65
    // 22.5 + 17.0 + 12.0 + 11.25 + 10.5 + 6.5 = 79.75 -> round = 80
    assert_eq!(breakdown.calculate_weighted_virality_score(), 80);

    let clamped = ScoreBreakdown::new(150, -20, 100, 0, 50, 50);
    assert_eq!(clamped.hook, 100);
    assert_eq!(clamped.retention, 0);

    let uniform = ScoreBreakdown::from_uniform(85);
    assert_eq!(uniform.calculate_weighted_virality_score(), 85);

    let def = ScoreBreakdown::default_breakdown();
    assert_eq!(def.calculate_weighted_virality_score(), 70);
}

#[test]
fn should_test_word_timestamp() {
    let wt = WordTimestamp::new("incrível", 1.25, 2.75);
    assert_eq!(wt.word, "incrível");
    assert_eq!(wt.duration(), 1.50);

    let inverted = WordTimestamp::new("teste", 5.0, 2.0);
    assert_eq!(inverted.duration(), 0.0);

    assert_eq!(WordTimestamp::new("dinheiro!", 0.0, 1.0).clean_word(), "dinheiro");
    assert_eq!(WordTimestamp::new("\"sucesso?\"", 0.0, 1.0).clean_word(), "sucesso");
    assert_eq!(WordTimestamp::new("atenção,", 0.0, 1.0).clean_word(), "atenção");
    assert_eq!(WordTimestamp::new("(vídeo)", 0.0, 1.0).clean_word(), "vídeo");
    assert_eq!(WordTimestamp::new("100%", 0.0, 1.0).clean_word(), "100");
    assert_eq!(WordTimestamp::new("   ", 0.0, 1.0).clean_word(), "");

    let short_time = WordTimestamp::new("olá", 1.250, 2.500);
    assert_eq!(short_time.formatted_time_range(), "[00:00:01.250 -> 00:00:02.500]");

    let long_time = WordTimestamp::new("fim", 3661.120, 3662.450);
    assert_eq!(long_time.formatted_time_range(), "[01:01:01.120 -> 01:01:02.450]");
}

#[test]
fn should_test_emoji_dictionary() {
    let test_cases = [
        ("dinheiro", "💰"),
        ("grana", "💰"),
        ("lucro", "💰"),
        ("money", "💰"),
        ("fogo", "🔥"),
        ("viral", "🔥"),
        ("top", "🔥"),
        ("incrivel", "🔥"),
        ("incrível", "🔥"),
        ("sucesso", "🔥"),
        ("ideia", "💡"),
        ("segredo", "💡"),
        ("dica", "💡"),
        ("atencao", "⚠️"),
        ("atenção", "⚠️"),
        ("cuidado", "⚠️"),
        ("perigo", "⚠️"),
        ("meta", "🏆"),
        ("campeao", "🏆"),
        ("campeão", "🏆"),
        ("amor", "❤️"),
        ("coracao", "❤️"),
        ("coração", "❤️"),
        ("tempo", "⏱️"),
        ("rapido", "⏱️"),
        ("rápido", "⏱️"),
        ("DINHEIRO!", "💰"),
        ("\"sucesso\"", "🔥"),
        ("  Atenção... ", "⚠️"),
        ("(grana)", "💰"),
        ("#viral", "🔥"),
    ];

    for (input, expected) in test_cases {
        assert_eq!(EmojiDictionary::find_emoji(input), Some(expected));
        assert!(EmojiDictionary::has_emoji_match(input));
    }

    assert_eq!(EmojiDictionary::find_emoji(""), None);
    assert_eq!(EmojiDictionary::find_emoji("   "), None);
    assert_eq!(EmojiDictionary::find_emoji("palavra_desconhecida_xyz"), None);
    assert!(!EmojiDictionary::has_emoji_match("palavra_desconhecida_xyz"));
}

#[test]
fn should_resolve_video_export_layout() {
    let temp_dir = std::env::temp_dir().join("clippr_test_export_layout");
    let layout = VideoExportLayout::resolve(&temp_dir, "10 Dicas Que Mudaram Minha Vida", "dQw4w9WgXcQ");

    assert_eq!(layout.sanitized_title, "10 Dicas Que Mudaram Minha Vida");
    assert_eq!(layout.video_id, "dQw4w9WgXcQ");

    let expected_video_dir = temp_dir.join("10 Dicas Que Mudaram Minha Vida [dQw4w9WgXcQ]");
    assert_eq!(layout.video_dir, expected_video_dir);
    assert_eq!(layout.cortes_dir, expected_video_dir.join("cortes"));
    assert_eq!(layout.thumbnails_dir, expected_video_dir.join("thumbnails"));
    assert_eq!(layout.dados_dir, expected_video_dir.join("dados"));

    // Title sanitization
    let accented = VideoExportLayout::sanitize_title("Ação, Coração & Emoção: O Guia dos Campeões");
    assert!(accented.contains("Ação"));
    assert!(accented.contains("Coração"));
    assert!(accented.contains("Emoção"));
    assert!(accented.contains("Campeões"));
    assert!(!accented.contains(':'));

    let forbidden = VideoExportLayout::sanitize_title("Video/With\\All:Forbidden*Chars?\"<And>|Pipes");
    for ch in ['/', '\\', ':', '*', '?', '"', '<', '>', '|'] {
        assert!(!forbidden.contains(ch));
    }

    let spaces = VideoExportLayout::sanitize_title("Vídeo    com    muitos      espaços     em branco");
    assert_eq!(spaces, "Vídeo com muitos espaços em branco");

    let long_title = "Este é um título extremamente longo que ultrapassa sessenta caracteres com certeza absoluta";
    let sanitized_long = VideoExportLayout::sanitize_title(long_title);
    assert!(sanitized_long.chars().count() <= 60);
    assert!(!sanitized_long.ends_with(' '));
    assert!(!sanitized_long.ends_with('-'));

    let giant = "A".repeat(80);
    assert_eq!(VideoExportLayout::sanitize_title(&giant).chars().count(), 60);

    let empty_title = VideoExportLayout::resolve(&temp_dir, "", "dQw4w9WgXcQ");
    assert_eq!(empty_title.video_dir.file_name().unwrap(), "video_dQw4w9WgXcQ");

    let unknown_id = VideoExportLayout::resolve(&temp_dir, "Título Bom", "");
    assert_eq!(unknown_id.video_id, "unknown");
    assert_eq!(unknown_id.video_dir.file_name().unwrap(), "Título Bom [unknown]");
}

#[tokio::test]
async fn should_publish_and_receive_events_on_event_bus() {
    let bus = EventBus::new(100);
    let mut rx = bus.subscribe();

    bus.publish_log("Teste de mensagem", false);
    bus.publish_progress("PIPELINE", 0.5, "Metade processada");

    let event1 = rx.recv().await.expect("Deveria receber log");
    match event1 {
        ClipprEvent::LogMessage { message, is_debug, .. } => {
            assert_eq!(message, "Teste de mensagem");
            assert!(!is_debug);
        }
        other => panic!("Esperava LogMessage, recebeu {:?}", other),
    }

    let event2 = rx.recv().await.expect("Deveria receber progresso");
    match event2 {
        ClipprEvent::ProgressUpdated { stage, progress, details, .. } => {
            assert_eq!(stage, "PIPELINE");
            assert_eq!(progress, 0.5);
            assert_eq!(details, "Metade processada");
        }
        other => panic!("Esperava ProgressUpdated, recebeu {:?}", other),
    }
}

#[test]
fn should_manage_app_settings() {
    let settings = AppSettings::default_settings();
    assert!(settings.output_directory.contains("ClipprExports"));
    assert_eq!(settings.min_virality_score, 90);
    assert!(!settings.auto_process_above_threshold);
    assert_eq!(settings.default_orientation, OrientationMode::Auto);
    assert_eq!(settings.auto_vertical_max_duration_seconds, 60.0);
    assert!(settings.burn_subtitles);
    assert_eq!(settings.cpu_threads, 0);
    assert_eq!(settings.gemini_model, "gemini-2.5-flash");
    assert_eq!(settings.ai_provider, AiProvider::Gemini);
    assert_eq!(settings.openai_model, "gpt-4o-mini");
    assert!(!settings.is_youtube_connected());

    let with_creds = settings.with_youtube_credentials("client-id-123", "client-secret-abc");
    assert_eq!(with_creds.youtube_client_id, "client-id-123");
    assert_eq!(with_creds.youtube_client_secret, "client-secret-abc");
    assert!(!with_creds.is_youtube_connected());

    let with_tokens = with_creds.with_youtube_tokens(
        "access-token-xyz",
        "refresh-token-999",
        1800000000,
        Some("Canal Clippr"),
        Some("UC123456789"),
        Some("https://clippr.com/avatar.jpg"),
    );
    assert!(with_tokens.is_youtube_connected());
    assert_eq!(with_tokens.youtube_access_token, "access-token-xyz");
    assert_eq!(with_tokens.youtube_channel_title, "Canal Clippr");

    let disconnected = with_tokens.without_youtube_auth();
    assert!(!disconnected.is_youtube_connected());
    assert_eq!(disconnected.youtube_client_id, "client-id-123");
    assert_eq!(disconnected.youtube_access_token, "");
}

fn create_sample_clip(start: f64, end: f64) -> ClipCandidate {
    ClipCandidate::new(
        Some(1),
        Some(1),
        "Título",
        "Gancho",
        start,
        end,
        95,
        "Reasoning",
        "Caption",
        vec![],
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
        None,
        None,
        None,
        None,
    )
}

#[test]
fn should_serialize_and_deserialize_app_settings_with_camel_case() {
    let settings = AppSettings::default_settings();
    let json = serde_json::to_string(&settings).expect("falha ao serializar AppSettings");

    assert!(json.contains("\"outputDirectory\":"));
    assert!(json.contains("\"minViralityScore\":"));
    assert!(json.contains("\"autoProcessAboveThreshold\":"));
    assert!(json.contains("\"defaultOrientation\":"));
    assert!(json.contains("\"autoVerticalMaxDurationSeconds\":"));
    assert!(json.contains("\"burnSubtitles\":"));
    assert!(json.contains("\"cpuThreads\":"));
    assert!(json.contains("\"geminiApiKey\":"));
    assert!(json.contains("\"geminiModel\":"));
    assert!(json.contains("\"aiProvider\":"));
    assert!(json.contains("\"openaiApiKey\":"));
    assert!(json.contains("\"openaiModel\":"));
    assert!(json.contains("\"youtubeClientId\":"));
    assert!(json.contains("\"youtubeClientSecret\":"));
    assert!(json.contains("\"youtubeAccessToken\":"));
    assert!(json.contains("\"youtubeRefreshToken\":"));
    assert!(json.contains("\"youtubeTokenExpiresAt\":"));
    assert!(json.contains("\"youtubeChannelTitle\":"));
    assert!(json.contains("\"youtubeChannelId\":"));
    assert!(json.contains("\"youtubeChannelAvatarUrl\":"));

    let deserialized: AppSettings =
        serde_json::from_str(&json).expect("falha ao desserializar AppSettings");
    assert_eq!(settings, deserialized);
}

#[test]
fn should_serialize_and_deserialize_video_source_with_camel_case() {
    let video = VideoSource::new(
        Some(42),
        "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
        "dQw4w9WgXcQ",
        "Rick Astley",
        "RickAstleyVEVO",
        213.0,
        Some("/tmp/transcript.vtt".to_string()),
        Some("/tmp/video.mp4".to_string()),
        ProcessingStatus::Completed,
        None,
        chrono::Utc::now(),
    );

    let json = serde_json::to_string(&video).expect("falha ao serializar VideoSource");

    assert!(json.contains("\"videoId\":"));
    assert!(json.contains("\"durationSeconds\":"));
    assert!(json.contains("\"transcriptPath\":"));
    assert!(json.contains("\"localVideoPath\":"));
    assert!(json.contains("\"errorMessage\":"));
    assert!(json.contains("\"createdAt\":"));

    let deserialized: VideoSource =
        serde_json::from_str(&json).expect("falha ao desserializar VideoSource");
    assert_eq!(video, deserialized);
}

#[test]
fn should_serialize_and_deserialize_clip_candidate_with_camel_case() {
    let clip = create_sample_clip(10.0, 45.0);
    let json = serde_json::to_string(&clip).expect("falha ao serializar ClipCandidate");

    assert!(json.contains("\"videoSourceId\":"));
    assert!(json.contains("\"startSeconds\":"));
    assert!(json.contains("\"endSeconds\":"));
    assert!(json.contains("\"viralityScore\":"));
    assert!(json.contains("\"suggestedCaption\":"));
    assert!(json.contains("\"hasSubtitles\":"));
    assert!(json.contains("\"hasEmojis\":"));
    assert!(json.contains("\"outputFilePath\":"));
    assert!(json.contains("\"metadataJsonPath\":"));
    assert!(json.contains("\"thumbnailTimestamp\":"));
    assert!(json.contains("\"thumbnailHeadline\":"));
    assert!(json.contains("\"thumbnailPath\":"));
    assert!(json.contains("\"rawFramePath\":"));
    assert!(json.contains("\"framingMode\":"));
    assert!(json.contains("\"subtitlePreset\":"));
    assert!(json.contains("\"emotionTag\":"));

    let deserialized: ClipCandidate =
        serde_json::from_str(&json).expect("falha ao desserializar ClipCandidate");
    assert_eq!(clip, deserialized);
}

#[test]
fn should_serialize_and_deserialize_word_timestamp_with_camel_case() {
    let word = WordTimestamp::with_confidence("clippr", 1.25, 2.50, 0.98);
    let json = serde_json::to_string(&word).expect("falha ao serializar WordTimestamp");

    assert!(json.contains("\"startSeconds\":"));
    assert!(json.contains("\"endSeconds\":"));
    assert!(json.contains("\"confidence\":"));

    let deserialized: WordTimestamp =
        serde_json::from_str(&json).expect("falha ao desserializar WordTimestamp");
    assert_eq!(word, deserialized);
}

#[test]
fn should_serialize_and_deserialize_score_breakdown_with_camel_case() {
    let scores = ScoreBreakdown::new(95, 90, 85, 80, 75, 70);
    let json = serde_json::to_string(&scores).expect("falha ao serializar ScoreBreakdown");

    assert!(json.contains("\"hook\":95"));
    assert!(json.contains("\"retention\":90"));
    assert!(json.contains("\"emotional\":85"));
    assert!(json.contains("\"storytelling\":80"));
    assert!(json.contains("\"standalone\":75"));
    assert!(json.contains("\"visual\":70"));

    let deserialized: ScoreBreakdown =
        serde_json::from_str(&json).expect("falha ao desserializar ScoreBreakdown");
    assert_eq!(scores, deserialized);
}
