use clipr_youtube::{YouTubeOAuthService, READONLY_SCOPE, UPLOAD_SCOPE};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use url::Url;

async fn read_full_http_request(socket: &mut TcpStream) -> String {
    let mut data = Vec::new();
    let mut buf = [0u8; 1024];
    loop {
        let n = socket.read(&mut buf).await.unwrap();
        if n == 0 {
            break;
        }
        data.extend_from_slice(&buf[..n]);
        let s = String::from_utf8_lossy(&data);
        if let Some(pos) = s.find("\r\n\r\n") {
            let headers = &s[..pos];
            if let Some(cl_line) = headers.lines().find(|l| l.to_ascii_lowercase().starts_with("content-length:")) {
                if let Ok(cl) = cl_line.split(':').nth(1).unwrap_or("").trim().parse::<usize>() {
                    let body_len = data.len() - (pos + 4);
                    if body_len >= cl {
                        break;
                    }
                }
            } else {
                break;
            }
        }
    }
    String::from_utf8_lossy(&data).to_string()
}

#[test]
fn test_build_authorization_url() {
    let service = YouTubeOAuthService::new();
    let auth_url = service.build_authorization_url("my-client-id.apps.googleusercontent.com", "http://127.0.0.1:8089/oauth2callback");

    let parsed = Url::parse(&auth_url).unwrap();
    assert_eq!(parsed.scheme(), "https");
    assert_eq!(parsed.host_str(), Some("accounts.google.com"));
    assert_eq!(parsed.path(), "/o/oauth2/v2/auth");

    let pairs: std::collections::HashMap<_, _> = parsed.query_pairs().into_owned().collect();
    assert_eq!(pairs.get("client_id").unwrap(), "my-client-id.apps.googleusercontent.com");
    assert_eq!(pairs.get("redirect_uri").unwrap(), "http://127.0.0.1:8089/oauth2callback");
    assert_eq!(pairs.get("response_type").unwrap(), "code");
    assert_eq!(pairs.get("access_type").unwrap(), "offline");
    assert_eq!(pairs.get("prompt").unwrap(), "consent");

    let scope = pairs.get("scope").unwrap();
    assert!(scope.contains(UPLOAD_SCOPE));
    assert!(scope.contains(READONLY_SCOPE));
}

#[tokio::test]
async fn test_exchange_code_for_tokens() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let token_endpoint = format!("http://{addr}/token");

    let mock_server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let req = read_full_http_request(&mut socket).await;

        assert!(req.contains("POST /token HTTP/1.1"));
        assert!(req.contains("grant_type=authorization_code"));
        assert!(req.contains("code=auth_code_123"));
        assert!(req.contains("client_id=test_client_id"));

        let body = r#"{
            "access_token": "ya29.test_access_token",
            "refresh_token": "1//test_refresh_token",
            "expires_in": 3600,
            "scope": "https://www.googleapis.com/auth/youtube.upload",
            "token_type": "Bearer"
        }"#;

        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        socket.write_all(response.as_bytes()).await.unwrap();
    });

    let service = YouTubeOAuthService::with_endpoints("https://accounts.google.com/auth", &token_endpoint);
    let tokens = service
        .exchange_code_for_tokens("test_client_id", "test_secret", "auth_code_123", "http://127.0.0.1:8089/oauth2callback")
        .await
        .expect("Troca de código deve suceder");

    mock_server.await.unwrap();

    assert_eq!(tokens.access_token, "ya29.test_access_token");
    assert_eq!(tokens.refresh_token.as_deref(), Some("1//test_refresh_token"));
    assert_eq!(tokens.expires_in_seconds, 3600);
    assert!(!tokens.is_expired());
}

#[tokio::test]
async fn test_refresh_access_token() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let token_endpoint = format!("http://{addr}/token");

    let mock_server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let req = read_full_http_request(&mut socket).await;

        assert!(req.contains("POST /token HTTP/1.1"));
        assert!(req.contains("grant_type=refresh_token"));
        assert!(req.contains("refresh_token=1%2F%2Ftest_refresh_token") || req.contains("refresh_token=1//test_refresh_token"));

        let body = r#"{
            "access_token": "ya29.refreshed_access_token",
            "expires_in": 1800,
            "scope": "upload_scope",
            "token_type": "Bearer"
        }"#;

        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        socket.write_all(response.as_bytes()).await.unwrap();
    });

    let service = YouTubeOAuthService::with_endpoints("https://accounts.google.com/auth", &token_endpoint);

    let refreshed = service
        .refresh_access_token("test_client_id", "test_secret", "1//test_refresh_token")
        .await
        .expect("Refresh deve suceder");

    mock_server.await.unwrap();

    assert_eq!(refreshed.access_token, "ya29.refreshed_access_token");
    assert_eq!(refreshed.refresh_token.as_deref(), Some("1//test_refresh_token"));
    assert_eq!(refreshed.expires_in_seconds, 1800);
    assert_eq!(refreshed.scope, "upload_scope");
}
