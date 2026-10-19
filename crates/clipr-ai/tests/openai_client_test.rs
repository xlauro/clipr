use clipr_ai::{AiClient, OpenAiClient, TranscriptAnalysisCache};
use clipr_domain::TranscriptSegment;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[test]
fn test_openai_client_initialization_defaults() {
    let client = OpenAiClient::new("fake_api_key");
    assert_eq!(client.model_name(), "gpt-4o-mini");
    assert_eq!(client.base_url(), "https://api.openai.com/v1/chat/completions");
}

#[test]
fn test_parse_openai_response() {
    let client = OpenAiClient::new("fake_key");

    let simulated_response = r##"{
      "choices": [
        {
          "message": {
            "content": "{\"clips\": [{\"title\": \"Dica de Ouro\", \"hook\": \"Quer saber o segredo?\", \"startSeconds\": 5.0, \"endSeconds\": 35.0, \"viralityScore\": 96, \"reasoning\": \"Gancho imediato\", \"suggestedCaption\": \"Dica imperdível #dicas\", \"hashtags\": [\"#dicas\", \"#shorts\"], \"framingMode\": \"AUTO_CROP\", \"subtitlePreset\": \"HORMOZI_YELLOW\"}]}"
          },
          "finish_reason": "stop"
        }
      ]
    }"##;

    let parsed = client.parse_openai_response(Some(10), simulated_response).expect("Parsing deve funcionar");
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].title, "Dica de Ouro");
    assert_eq!(parsed[0].virality_score, 96);
    assert_eq!(parsed[0].start_seconds, 5.0);
    assert_eq!(parsed[0].end_seconds, 35.0);
}

#[tokio::test]
async fn test_openai_client_e2e_with_mock_server() {
    TranscriptAnalysisCache::global().clear();

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let mock_server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buffer = [0u8; 4096];
        let _ = socket.read(&mut buffer).await.unwrap();

        let response_body = r#"{
          "choices": [
            {
              "message": {
                "content": "{\"clips\": [{\"title\": \"Corte OpenAI Aprovado\", \"hook\": \"Gancho\", \"startSeconds\": 10.0, \"endSeconds\": 40.0, \"viralityScore\": 92}]}"
              },
              "finish_reason": "stop"
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

    let client = OpenAiClient::with_config(
        "test_api_key",
        "gpt-4o-mini",
        2,
        10,
        &format!("http://{}", addr),
    );

    let segments = vec![
        TranscriptSegment::new(10.0, 40.0, "Fala OpenAI aprovada"),
    ];

    let result = client
        .analyze_transcript(Some(55), "Título Vídeo", "Criador", &segments, 85)
        .await
        .expect("Deve analisar via mock OpenAI");

    mock_server.await.unwrap();

    assert_eq!(result.approved_clips.len(), 1);
    assert_eq!(result.approved_clips[0].title, "Corte OpenAI Aprovado");
    assert_eq!(result.approved_clips[0].virality_score, 92);
}
