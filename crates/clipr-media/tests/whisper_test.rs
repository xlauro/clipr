use clipr_domain::TranscriptSegment;
use clipr_media::transcription::{
    AudioExtractor, CloudWhisperClient, WhisperCppManager, WhisperService,
    WhisperTranscriptionService,
};
use std::path::Path;
use tempfile::tempdir;

#[test]
fn test_parse_cloud_whisper_verbose_json_with_root_words() {
    let json = r#"{
      "task": "transcribe",
      "language": "portuguese",
      "duration": 4.5,
      "text": "Olá mundo viral",
      "words": [
        { "word": "Olá", "start": 0.20, "end": 0.85 },
        { "word": "mundo", "start": 0.86, "end": 1.40 },
        { "word": "viral", "start": 1.45, "end": 2.10 }
      ]
    }"#;

    let words = CloudWhisperClient::parse_whisper_verbose_json_response(json).unwrap();

    assert_eq!(words.len(), 3);
    assert_eq!(words[0].word, "Olá");
    assert!((words[0].start_seconds - 0.20).abs() < 0.001);
    assert!((words[0].end_seconds - 0.85).abs() < 0.001);

    assert_eq!(words[1].word, "mundo");
    assert!((words[1].start_seconds - 0.86).abs() < 0.001);
    assert!((words[1].end_seconds - 1.40).abs() < 0.001);

    assert_eq!(words[2].word, "viral");
    assert!((words[2].start_seconds - 1.45).abs() < 0.001);
    assert!((words[2].end_seconds - 2.10).abs() < 0.001);
}

#[test]
fn test_parse_cloud_whisper_json_with_nested_segments() {
    let json = r#"{
      "task": "transcribe",
      "segments": [
        {
          "id": 0,
          "start": 0.0,
          "end": 2.0,
          "words": [
            { "word": "Primeiro", "start": 0.1, "end": 0.6 },
            { "word": "trecho", "start": 0.65, "end": 1.1 }
          ]
        },
        {
          "id": 1,
          "start": 2.0,
          "end": 3.5,
          "words": [
            { "word": "Segundo", "start": 2.1, "end": 2.8 }
          ]
        }
      ]
    }"#;

    let words = CloudWhisperClient::parse_whisper_verbose_json_response(json).unwrap();

    assert_eq!(words.len(), 3);
    assert_eq!(words[0].word, "Primeiro");
    assert_eq!(words[1].word, "trecho");
    assert_eq!(words[2].word, "Segundo");
    assert!((words[2].start_seconds - 2.1).abs() < 0.001);
    assert!((words[2].end_seconds - 2.8).abs() < 0.001);
}

#[test]
fn test_handle_empty_or_null_cloud_json() {
    assert!(CloudWhisperClient::parse_whisper_verbose_json_response("").unwrap().is_empty());
    assert!(CloudWhisperClient::parse_whisper_verbose_json_response("   ").unwrap().is_empty());
    assert!(CloudWhisperClient::parse_whisper_verbose_json_response("{}").unwrap().is_empty());
}

#[test]
fn test_parse_whisper_cpp_json_with_tokens() {
    let json = r#"{
      "transcription": [
        {
          "timestamps": { "from": "00:00:01,000", "to": "00:00:03,500" },
          "offsets": { "from": 1000, "to": 3500 },
          "text": " Corte insano",
          "tokens": [
            {
              "text": " Corte",
              "timestamps": { "from": "00:00:01,000", "to": "00:00:02,000" },
              "offsets": { "from": 1000, "to": 2000 }
            },
            {
              "text": " insano",
              "timestamps": { "from": "00:00:02,100", "to": "00:00:03,200" },
              "offsets": { "from": 2100, "to": 3200 }
            }
          ]
        }
      ]
    }"#;

    let words = WhisperCppManager::parse_whisper_cpp_json(json).unwrap();

    assert_eq!(words.len(), 2);
    assert_eq!(words[0].word, "Corte");
    assert!((words[0].start_seconds - 1.0).abs() < 0.001);
    assert!((words[0].end_seconds - 2.0).abs() < 0.001);

    assert_eq!(words[1].word, "insano");
    assert!((words[1].start_seconds - 2.1).abs() < 0.001);
    assert!((words[1].end_seconds - 3.2).abs() < 0.001);
}

#[test]
fn test_parse_whisper_cpp_json_using_numeric_offsets() {
    let json = r#"{
      "transcription": [
        {
          "offsets": { "from": 500, "to": 1500 },
          "text": "Fallback",
          "tokens": []
        }
      ]
    }"#;

    let words = WhisperCppManager::parse_whisper_cpp_json(json).unwrap();

    assert_eq!(words.len(), 1);
    assert_eq!(words[0].word, "Fallback");
    assert!((words[0].start_seconds - 0.5).abs() < 0.001);
    assert!((words[0].end_seconds - 1.5).abs() < 0.001);
}

#[test]
fn test_whisper_service_fallback_ladder() {
    let empty_cloud = CloudWhisperClient::custom("", "", "");
    assert!(!empty_cloud.is_available());

    let empty_local = WhisperCppManager::with_custom_binary(Path::new("/caminho/falso/whisper-cli"));
    assert!(!empty_local.is_available());

    let service = WhisperService::custom(empty_cloud, empty_local);
    assert!(!service.is_available());
    assert_eq!(service.get_active_backend_name(), "INTERPOLATION_FALLBACK");

    // Test graceful fallback transcription
    let segments = vec![TranscriptSegment::new(10.0, 15.0, "Ganhe dinheiro rápido na internet")];

    let fallback_words = service.transcribe_clip_with_fallback(
        Path::new("inexistente.mp4"),
        10.0,
        15.0,
        &segments,
    );

    assert!(!fallback_words.is_empty(), "Algorithmic fallback must provide interpolated words");
    assert_eq!(fallback_words[0].word, "Ganhe");
    assert!((fallback_words[0].start_seconds - 0.0).abs() < 0.001); // Relative to clip start
    assert!(fallback_words.last().unwrap().end_seconds <= 5.01);
}

#[test]
fn test_audio_extractor_resolve_binary() {
    let binary = AudioExtractor::resolve_ffmpeg_binary();
    assert!(!binary.is_empty());
    assert!(binary.contains("ffmpeg"));
}

#[test]
fn test_audio_extractor_throws_when_video_does_not_exist() {
    let temp_dir = tempdir().unwrap();
    let non_existent = temp_dir.path().join("inexistente.mp4");
    let output_wav = temp_dir.path().join("out.wav");

    let res = AudioExtractor::extract_wav_16k_mono(&non_existent, 0.0, 5.0, &output_wav);
    assert!(res.is_err());
}
