use clipr_ai::{AiClient, GeminiClient, TranscriptAnalysisCache};
use clipr_domain::{ClipCandidate, EmotionTag, FramingMode, OrientationMode, ScoreBreakdown, SubtitlePreset, TranscriptSegment};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[test]
fn test_gemini_client_initialization_defaults() {
    let client = GeminiClient::new("fake_api_key");
    assert_eq!(client.model_name(), "gemini-2.5-flash");
    assert_eq!(client.fallback_model_name(), "gemini-1.5-flash");
}

#[test]
fn test_clean_json_content() {
    let raw = r#"```json
    [
      {
        "title": "Corte 1",
        "startSeconds": 00:01:23.450,
        "endSeconds": 00:02:10,
        "score": 90,
      }
    ]
    ```"#;

    let cleaned = GeminiClient::clean_json_content(raw);
    assert!(!cleaned.contains("```"));
    assert!(cleaned.contains(r#""startSeconds": "00:01:23.450""#));
    assert!(cleaned.contains(r#""endSeconds": "00:02:10""#));
    assert!(!cleaned.contains(",\n    ]")); // Trailing comma removido
}

#[test]
fn test_parse_flexible_timestamp() {
    assert_eq!(GeminiClient::parse_flexible_timestamp("75.5", 0.0), 75.5);
    assert_eq!(GeminiClient::parse_flexible_timestamp("75,5s", 0.0), 75.5);
    assert_eq!(GeminiClient::parse_flexible_timestamp("01:30", 0.0), 90.0);
    assert_eq!(GeminiClient::parse_flexible_timestamp("01:02:03", 0.0), 3723.0);
}

#[test]
fn test_parse_gemini_response_with_weighted_scores_and_emotions() {
    let client = GeminiClient::new("fake_key");

    let api_response = r##"{
      "candidates": [
        {
          "content": {
            "parts": [
              {
                "text": "[{\"title\":\"O Erro de 1 Milhão\",\"hook\":\"Eu perdi tudo em 24h\",\"startSeconds\":10.0,\"endSeconds\":45.0,\"scores\":{\"hook\":95,\"retention\":90,\"emotional\":95,\"storytelling\":85,\"standalone\":90,\"visual\":80},\"emotionTag\":\"ALTA_ENERGIA\",\"reasoning\":\"Arco dramático\",\"suggestedCaption\":\"Nunca cometa esse erro\",\"hashtags\":[\"#viral\",\"#dev\"],\"thumbnailTimestamp\":20.5,\"thumbnailHeadline\":\"O ERRO FINAL\",\"framingMode\":\"AUTO_CROP\",\"subtitlePreset\":\"HORMOZI_YELLOW\"}]"
              }
            ]
          }
        }
      ]
    }"##;


    let candidates = client.parse_gemini_response(Some(42), api_response).expect("Parsing deve funcionar");
    assert_eq!(candidates.len(), 1);

    let clip = &candidates[0];
    assert_eq!(clip.video_source_id, Some(42));
    assert_eq!(clip.title, "O Erro de 1 Milhão");
    assert_eq!(clip.hook, "Eu perdi tudo em 24h");
    assert_eq!(clip.start_seconds, 10.0);
    assert_eq!(clip.end_seconds, 45.0);
    assert_eq!(clip.emotion_tag, EmotionTag::AltaEnergia);
    assert_eq!(clip.framing_mode, FramingMode::AutoCrop);
    assert_eq!(clip.subtitle_preset, SubtitlePreset::HormoziYellow);
    assert_eq!(clip.thumbnail_timestamp, Some(20.5));
    assert_eq!(clip.thumbnail_headline.as_deref(), Some("O ERRO FINAL"));

    // Rubrica calculada
    assert_eq!(clip.scores.hook, 95);
    assert_eq!(clip.scores.retention, 90);
    assert!(clip.virality_score >= 89 && clip.virality_score <= 92);
}

#[test]
fn test_snap_to_segment_boundaries() {
    let client = GeminiClient::new("fake_key");

    let segments = vec![
        TranscriptSegment::new(10.0, 15.0, "Parte 1"),
        TranscriptSegment::new(15.0, 30.0, "Parte 2"),
        TranscriptSegment::new(30.0, 46.0, "Parte 3"),
    ];

    let candidate = ClipCandidate::new(
        None,
        Some(1),
        "Clip 1",
        "Hook",
        10.8, // Gemini deu 10.8 -> snap para 10.0
        45.2, // Gemini deu 45.2 -> snap para 46.0
        95,
        "Motivo",
        "Caption",
        vec![],
        OrientationMode::Vertical916,
        true,
        true,
        None,
        None,
        clipr_domain::ProcessingStatus::PendingApproval,
        Some(12.0),
        Some("HL".to_string()),
        None,
        None,
        Some(FramingMode::AutoCrop),
        Some(SubtitlePreset::HormoziYellow),
        Some(EmotionTag::AltaEnergia),
        Some(ScoreBreakdown::from_uniform(95)),
    );

    let snapped = client.snap_to_segment_boundaries(&[candidate], &segments);
    assert_eq!(snapped.len(), 1);
    assert_eq!(snapped[0].start_seconds, 10.0);
    assert_eq!(snapped[0].end_seconds, 46.0);
}

#[test]
fn test_snap_extreme_tolerances() {
    let client = GeminiClient::new("fake_key");

    let segments = vec![
        TranscriptSegment::new(0.0, 10.0, "Intro"),
        TranscriptSegment::new(10.0, 60.0, "Fim"),
    ];

    // Início negativo
    let neg = ClipCandidate::new(
        None, Some(1), "Neg", "H", -10.0, 30.0, 90, "R", "C", vec![],
        OrientationMode::Vertical916, true, true, None, None,
        clipr_domain::ProcessingStatus::PendingApproval, Some(-5.0), Some("HL".to_string()), None, None,
        Some(FramingMode::AutoCrop), Some(SubtitlePreset::HormoziYellow), Some(EmotionTag::AltaEnergia), Some(ScoreBreakdown::from_uniform(90)),
    );

    // Além do fim (999.0 > 60.0)
    let beyond = ClipCandidate::new(
        None, Some(1), "Beyond", "H", 20.0, 999.0, 90, "R", "C", vec![],
        OrientationMode::Vertical916, true, true, None, None,
        clipr_domain::ProcessingStatus::PendingApproval, Some(100.0), Some("HL".to_string()), None, None,
        Some(FramingMode::AutoCrop), Some(SubtitlePreset::HormoziYellow), Some(EmotionTag::AltaEnergia), Some(ScoreBreakdown::from_uniform(90)),
    );

    let snapped = client.snap_to_segment_boundaries(&[neg, beyond], &segments);
    assert_eq!(snapped.len(), 2);
    assert!(snapped[0].start_seconds >= 0.0);
    assert!(snapped[1].end_seconds <= 60.0);
}

#[tokio::test]
async fn test_gemini_client_e2e_with_mock_server() {
    TranscriptAnalysisCache::global().clear();

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let mock_server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buffer = [0u8; 4096];
        let _ = socket.read(&mut buffer).await.unwrap();

        let response_body = r#"{
          "candidates": [
            {
              "content": {
                "parts": [
                  {
                    "text": "[{\"title\":\"Corte Aprovado Mock\",\"hook\":\"Gancho\",\"startSeconds\":10.0,\"endSeconds\":30.0,\"viralityScore\":95,\"emotionTag\":\"ALTA_ENERGIA\",\"scores\":{\"hook\":95,\"retention\":95,\"emotional\":95,\"storytelling\":95,\"standalone\":95,\"visual\":95}},{\"title\":\"Corte Descartado Mock\",\"hook\":\"Gancho B\",\"startSeconds\":30.0,\"endSeconds\":50.0,\"viralityScore\":60,\"emotionTag\":\"HUMOR_GARGALHADA\",\"scores\":{\"hook\":60,\"retention\":60,\"emotional\":60,\"storytelling\":60,\"standalone\":60,\"visual\":60}}]"
                  }
                ]
              }
            }
          ]
        }"#;

        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            response_body.len(),
            response_body
        );

        socket.write_all(response.as_bytes()).await.unwrap();
        socket.shutdown().await.unwrap();
    });

    let client = GeminiClient::with_config(
        "test_api_key",
        "mock-model",
        "mock-fallback",
        2,
        1,
        10,
        50,
        10,
        50,
        &format!("http://{}", addr),
    );

    let segments = vec![
        TranscriptSegment::new(10.0, 30.0, "Fala aprovada"),
        TranscriptSegment::new(30.0, 50.0, "Fala descartada"),
    ];

    let result = client
        .analyze_transcript(Some(123), "Vídeo Teste", "Autor Teste", &segments, 85)
        .await
        .expect("Deve analisar transcrição com sucesso via mock server");

    mock_server.await.unwrap();

    assert_eq!(result.approved_clips.len(), 1);
    assert_eq!(result.discarded_clips.len(), 1);
    assert_eq!(result.approved_clips[0].title, "Corte Aprovado Mock");
    assert_eq!(result.discarded_clips[0].title, "Corte Descartado Mock");
    assert_eq!(result.discarded_clips[0].status, clipr_domain::ProcessingStatus::DiscardedLowScore);
}
