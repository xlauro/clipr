use clipr_ai::{AnalysisResult, TranscriptAnalysisCache};
use clipr_domain::{ClipCandidate, EmotionTag, FramingMode, OrientationMode, ProcessingStatus, ScoreBreakdown, SubtitlePreset};

#[test]
fn test_compute_key_deterministic() {
    let key1 = TranscriptAnalysisCache::compute_key("Título A", "Autor B", "Transcrição C", 85, "gemini-2.5-flash");
    let key2 = TranscriptAnalysisCache::compute_key("Título A", "Autor B", "Transcrição C", 85, "gemini-2.5-flash");
    assert_eq!(key1, key2);
    assert_eq!(key1.len(), 64); // SHA-256 em hex

    let key_diff = TranscriptAnalysisCache::compute_key("Título A", "Autor B", "Transcrição C", 90, "gemini-2.5-flash");
    assert_ne!(key1, key_diff);
}

#[test]
fn test_cache_put_get_and_adapt_video_source_id() {
    let cache = TranscriptAnalysisCache::new();

    let clip = ClipCandidate::new(
        Some(1),
        Some(10),
        "Clipe Viral",
        "Gancho",
        10.0,
        40.0,
        95,
        "Motivo",
        "Legenda",
        vec!["#viral".to_string()],
        OrientationMode::Vertical916,
        true,
        true,
        None,
        None,
        ProcessingStatus::Completed,
        Some(15.0),
        Some("HEADLINE".to_string()),
        None,
        None,
        Some(FramingMode::AutoCrop),
        Some(SubtitlePreset::HormoziYellow),
        Some(EmotionTag::AltaEnergia),
        Some(ScoreBreakdown::from_uniform(95)),
    );

    let result = AnalysisResult::new(vec![clip], vec![]);
    let key = "test_key_123";

    cache.put(key, result);
    assert!(cache.contains_key(key));
    assert_eq!(cache.size(), 1);

    // Recupera adaptando video_source_id para 99
    let fetched = cache.get(key, Some(99)).expect("Cache deve conter a chave");
    assert_eq!(fetched.approved_clips.len(), 1);
    assert_eq!(fetched.approved_clips[0].video_source_id, Some(99));

    cache.clear();
    assert_eq!(cache.size(), 0);
    assert!(!cache.contains_key(key));
}
