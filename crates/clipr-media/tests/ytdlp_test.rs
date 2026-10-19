use clipr_media::ytdlp::YtDlpManager;
use serde_json::json;

#[test]
fn test_prioritize_manual_portuguese_subtitles() {
    let root = json!({
        "id": "test123",
        "title": "Vídeo Teste",
        "subtitles": {
            "pt": [
                {"ext": "srv1", "url": "https://example.com/srv1"},
                {"ext": "vtt", "url": "https://example.com/manual-pt.vtt"}
            ],
            "en": [
                {"ext": "vtt", "url": "https://example.com/manual-en.vtt"}
            ]
        },
        "automatic_captions": {
            "pt-orig": [
                {"ext": "vtt", "url": "https://example.com/auto-pt.vtt"}
            ]
        }
    });

    let track = YtDlpManager::find_best_subtitle_track(&root);
    assert!(track.is_some());
    let t = track.unwrap();
    assert_eq!(t.language, "pt");
    assert_eq!(t.format, "vtt");
    assert_eq!(t.url, "https://example.com/manual-pt.vtt");
}

#[test]
fn test_prioritize_pt_br_over_english() {
    let root = json!({
        "id": "test_br",
        "subtitles": {
            "en": [
                {"ext": "vtt", "url": "https://example.com/en.vtt"}
            ],
            "pt-BR": [
                {"ext": "vtt", "url": "https://example.com/pt-br.vtt"}
            ]
        }
    });

    let track = YtDlpManager::find_best_subtitle_track(&root);
    assert!(track.is_some());
    let t = track.unwrap();
    assert_eq!(t.language, "pt-BR");
    assert_eq!(t.url, "https://example.com/pt-br.vtt");
}

#[test]
fn test_fallback_to_auto_portuguese_subtitles() {
    let root = json!({
        "id": "test456",
        "title": "Vídeo Teste Auto",
        "subtitles": {},
        "automatic_captions": {
            "pt-orig": [
                {"ext": "json3", "url": "https://example.com/json3"},
                {"ext": "vtt", "url": "https://example.com/auto-pt-orig.vtt"}
            ],
            "en": [
                {"ext": "vtt", "url": "https://example.com/auto-en.vtt"}
            ]
        }
    });

    let track = YtDlpManager::find_best_subtitle_track(&root);
    assert!(track.is_some());
    let t = track.unwrap();
    assert_eq!(t.language, "pt-orig");
    assert_eq!(t.format, "vtt");
    assert_eq!(t.url, "https://example.com/auto-pt-orig.vtt");
}

#[test]
fn test_fallback_to_english_when_no_portuguese() {
    let root = json!({
        "id": "test_en",
        "subtitles": {},
        "automatic_captions": {
            "en": [
                {"ext": "vtt", "url": "https://example.com/en-auto.vtt"}
            ],
            "es": [
                {"ext": "vtt", "url": "https://example.com/es-auto.vtt"}
            ]
        }
    });

    let track = YtDlpManager::find_best_subtitle_track(&root);
    assert!(track.is_some());
    let t = track.unwrap();
    assert_eq!(t.language, "en");
    assert_eq!(t.url, "https://example.com/en-auto.vtt");
}

#[test]
fn test_fallback_to_any_other_language() {
    let root = json!({
        "id": "test_other",
        "subtitles": {
            "es": [
                {"ext": "vtt", "url": "https://example.com/es.vtt"}
            ]
        },
        "automatic_captions": {}
    });

    let track = YtDlpManager::find_best_subtitle_track(&root);
    assert!(track.is_some());
    let t = track.unwrap();
    assert_eq!(t.language, "es");
    assert_eq!(t.url, "https://example.com/es.vtt");
}

#[test]
fn test_prioritize_vtt_over_srt() {
    let root = json!({
        "id": "test_fmt",
        "subtitles": {
            "pt": [
                {"ext": "srt", "url": "https://example.com/pt.srt"},
                {"ext": "vtt", "url": "https://example.com/pt.vtt"}
            ]
        }
    });

    let track = YtDlpManager::find_best_subtitle_track(&root);
    assert!(track.is_some());
    let t = track.unwrap();
    assert_eq!(t.format, "vtt");
    assert_eq!(t.url, "https://example.com/pt.vtt");
}

#[test]
fn test_select_srt_when_vtt_not_available() {
    let root = json!({
        "id": "test_srt",
        "subtitles": {
            "pt": [
                {"ext": "srv1", "url": "https://example.com/pt.srv1"},
                {"ext": "srt", "url": "https://example.com/pt.srt"}
            ]
        }
    });

    let track = YtDlpManager::find_best_subtitle_track(&root);
    assert!(track.is_some());
    let t = track.unwrap();
    assert_eq!(t.format, "srt");
    assert_eq!(t.url, "https://example.com/pt.srt");
}

#[test]
fn test_return_none_when_only_unsupported_formats() {
    let root = json!({
        "id": "test_unsupported",
        "subtitles": {
            "pt": [
                {"ext": "json3", "url": "https://example.com/pt.json3"},
                {"ext": "ttml", "url": "https://example.com/pt.ttml"}
            ]
        }
    });

    let track = YtDlpManager::find_best_subtitle_track(&root);
    assert!(track.is_none());
}

#[test]
fn test_skip_entries_with_blank_or_missing_url() {
    let root = json!({
        "id": "test_blank_url",
        "subtitles": {
            "pt": [
                {"ext": "vtt", "url": ""},
                {"ext": "vtt", "url": "   "}
            ],
            "en": [
                {"ext": "vtt", "url": "https://example.com/en.vtt"}
            ]
        }
    });

    let track = YtDlpManager::find_best_subtitle_track(&root);
    assert!(track.is_some());
    let t = track.unwrap();
    assert_eq!(t.language, "en");
    assert_eq!(t.url, "https://example.com/en.vtt");
}

#[test]
fn test_handle_empty_json_safely() {
    let empty_root = json!({});
    assert!(YtDlpManager::find_best_subtitle_track(&empty_root).is_none());

    let null_root = json!(null);
    assert!(YtDlpManager::find_best_subtitle_track(&null_root).is_none());
}

#[test]
fn test_return_none_when_no_subtitles_available() {
    let root = json!({
        "id": "test789",
        "title": "Sem Legendas",
        "subtitles": {},
        "automatic_captions": {}
    });

    assert!(YtDlpManager::find_best_subtitle_track(&root).is_none());
}

#[test]
fn test_resolve_binary_path() {
    let manager = YtDlpManager::new();
    let path = manager.get_binary_path();
    assert!(!path.as_os_str().is_empty());
}
