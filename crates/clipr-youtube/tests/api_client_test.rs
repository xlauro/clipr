use clipr_youtube::{UploadProgress, VideoPrivacy, VideoUploadRequest, YouTubeApiClient};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tempfile::NamedTempFile;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[tokio::test]
async fn test_get_channel_info_success() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let base_url = format!("http://{addr}");

    let mock_server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buffer = [0u8; 4096];
        let n = socket.read(&mut buffer).await.unwrap();
        let req = String::from_utf8_lossy(&buffer[..n]);

        assert!(req.contains("GET /youtube/v3/channels?part=snippet,statistics&mine=true HTTP/1.1"));
        assert!(req.contains("Authorization: Bearer mock_access_token") || req.contains("authorization: Bearer mock_access_token"));

        let body = r#"{
          "items": [
            {
              "id": "UC_TEST_CHANNEL_42",
              "snippet": {
                "title": "Canal Clippr Rust",
                "description": "Automação e IA para vídeos",
                "customUrl": "@clipprrust",
                "thumbnails": {
                  "high": {
                    "url": "https://yt3.ggpht.com/avatar.jpg"
                  }
                }
              },
              "statistics": {
                "subscriberCount": "12800"
              }
            }
          ]
        }"#;

        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        socket.write_all(response.as_bytes()).await.unwrap();
    });

    let client = YouTubeApiClient::with_urls(&base_url, &base_url);
    let channel = client.get_channel_info("mock_access_token").await.unwrap();

    mock_server.await.unwrap();

    assert_eq!(channel.id, "UC_TEST_CHANNEL_42");
    assert_eq!(channel.title, "Canal Clippr Rust");
    assert_eq!(channel.description, "Automação e IA para vídeos");
    assert_eq!(channel.custom_url.as_deref(), Some("@clipprrust"));
    assert_eq!(channel.avatar_url.as_deref(), Some("https://yt3.ggpht.com/avatar.jpg"));
    assert_eq!(channel.subscriber_count, 12800);
}

#[tokio::test]
async fn test_get_channel_info_empty_items() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let base_url = format!("http://{addr}");

    let mock_server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buffer = [0u8; 4096];
        let _ = socket.read(&mut buffer).await.unwrap();

        let body = r#"{"items": []}"#;
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        socket.write_all(response.as_bytes()).await.unwrap();
    });

    let client = YouTubeApiClient::with_urls(&base_url, &base_url);
    let result = client.get_channel_info("mock_token").await;

    mock_server.await.unwrap();

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("No YouTube channel found"));
}

#[tokio::test]
async fn test_upload_video_resumable_and_thumbnail() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let base_url = format!("http://{addr}");

    // Arquivo temporário de vídeo
    let mut temp_video = NamedTempFile::new().unwrap();
    let video_bytes = vec![0xAA; 4096]; // 4096 bytes
    std::io::Write::write_all(&mut temp_video, &video_bytes).unwrap();
    let video_path = temp_video.into_temp_path();

    // Arquivo temporário de thumbnail
    let mut temp_thumb = NamedTempFile::new().unwrap();
    let thumb_bytes = b"fake_jpeg_data";
    std::io::Write::write_all(&mut temp_thumb, thumb_bytes).unwrap();
    let thumb_path = temp_thumb.into_temp_path();

    let server_addr = addr;
    let mock_server = tokio::spawn(async move {
        // Conexão 1: Iniciar resumable upload (POST)
        let (mut socket1, _) = listener.accept().await.unwrap();
        let mut buffer = [0u8; 4096];
        let n1 = socket1.read(&mut buffer).await.unwrap();
        let req1 = String::from_utf8_lossy(&buffer[..n1]);

        assert!(req1.contains("POST /upload/youtube/v3/videos?uploadType=resumable"));
        assert!(req1.contains("x-upload-content-type: video/mp4") || req1.contains("X-Upload-Content-Type: video/mp4"));

        let session_url = format!("http://{server_addr}/upload_session_abc123");
        let response1 = format!(
            "HTTP/1.1 200 OK\r\nLocation: {}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            session_url
        );
        socket1.write_all(response1.as_bytes()).await.unwrap();

        // Conexão 2: Upload binário (PUT no upload_session)
        let (mut socket2, _) = listener.accept().await.unwrap();
        let mut buf2 = Vec::new();
        let mut chunk = [0u8; 4096];
        loop {
            let n = socket2.read(&mut chunk).await.unwrap();
            if n == 0 {
                break;
            }
            buf2.extend_from_slice(&chunk[..n]);
            // Se já leu cabeçalhos + conteúdo
            let text = String::from_utf8_lossy(&buf2);
            if text.contains("PUT /upload_session_abc123") && buf2.len() >= 4096 {
                break;
            }
        }

        let body2 = r#"{"id": "vid_xyz_12345"}"#;
        let response2 = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body2.len(),
            body2
        );
        socket2.write_all(response2.as_bytes()).await.unwrap();

        // Conexão 3: Upload de thumbnail (POST)
        let (mut socket3, _) = listener.accept().await.unwrap();
        let mut buf3 = [0u8; 4096];
        let n3 = socket3.read(&mut buf3).await.unwrap();
        let req3 = String::from_utf8_lossy(&buf3[..n3]);

        assert!(req3.contains("/upload/youtube/v3/thumbnails/set?videoId=vid_xyz_12345"));

        let body3 = r#"{"items": []}"#;
        let response3 = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body3.len(),
            body3
        );
        socket3.write_all(response3.as_bytes()).await.unwrap();
    });

    let progress_events = Arc::new(AtomicUsize::new(0));
    let progress_events_clone = progress_events.clone();

    let callback = Arc::new(move |p: UploadProgress| {
        progress_events_clone.fetch_add(1, Ordering::SeqCst);
        assert!(p.percentage >= 0.0 && p.percentage <= 100.0);
    });

    let client = YouTubeApiClient::with_urls(&base_url, &base_url);
    let request = VideoUploadRequest::with_options(
        video_path.to_path_buf(),
        "Meu Corte Viral #Shorts",
        "Descrição incrível",
        vec!["shorts".to_string(), "viral".to_string()],
        VideoPrivacy::Public,
        false,
        Some(thumb_path.to_path_buf()),
    );

    let result = client
        .upload_video("mock_access_token", &request, Some(callback))
        .await
        .expect("Upload deve ser concluído com sucesso");

    mock_server.await.unwrap();

    assert_eq!(result.video_id, "vid_xyz_12345");
    assert_eq!(result.video_url, "https://www.youtube.com/watch?v=vid_xyz_12345");
    assert_eq!(result.title, "Meu Corte Viral #Shorts");
    assert_eq!(result.privacy, VideoPrivacy::Public);

    // Deve ter registrado eventos de progresso (started + chunks + completed)
    assert!(progress_events.load(Ordering::SeqCst) >= 2);
}
