use crate::error::{YouTubeAuthError, YouTubeError};
use crate::model::YouTubeOAuthTokens;
use reqwest::Client as HttpClient;
use serde_json::Value;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tracing::info;
use url::Url;

pub const DEFAULT_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
pub const DEFAULT_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";

pub const UPLOAD_SCOPE: &str = "https://www.googleapis.com/auth/youtube.upload";
pub const READONLY_SCOPE: &str = "https://www.googleapis.com/auth/youtube.readonly";
pub const COMBINED_SCOPES: &str = "https://www.googleapis.com/auth/youtube.upload https://www.googleapis.com/auth/youtube.readonly";

#[derive(Clone)]
pub struct YouTubeOAuthService {
    http_client: HttpClient,
    auth_base_url: String,
    token_endpoint_url: String,
}

impl Default for YouTubeOAuthService {
    fn default() -> Self {
        Self::new()
    }
}

impl YouTubeOAuthService {
    pub fn new() -> Self {
        Self::with_endpoints(DEFAULT_AUTH_URL, DEFAULT_TOKEN_URL)
    }

    pub fn with_endpoints(auth_base_url: impl Into<String>, token_endpoint_url: impl Into<String>) -> Self {
        let http_client = HttpClient::builder()
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(15))
            .build()
            .unwrap_or_default();

        Self {
            http_client,
            auth_base_url: auth_base_url.into(),
            token_endpoint_url: token_endpoint_url.into(),
        }
    }

    pub fn build_authorization_url(&self, client_id: &str, redirect_uri: &str) -> String {
        let mut url = Url::parse(&self.auth_base_url).unwrap_or_else(|_| Url::parse(DEFAULT_AUTH_URL).unwrap());
        url.query_pairs_mut()
            .append_pair("client_id", client_id)
            .append_pair("redirect_uri", redirect_uri)
            .append_pair("response_type", "code")
            .append_pair("scope", COMBINED_SCOPES)
            .append_pair("access_type", "offline")
            .append_pair("prompt", "consent");
        url.to_string()
    }

    pub async fn authorize(
        &self,
        client_id: &str,
        client_secret: &str,
        timeout: Duration,
        auth_url_sender: Option<tokio::sync::oneshot::Sender<String>>,
    ) -> Result<YouTubeOAuthTokens, YouTubeError> {
        let listener = match TcpListener::bind("127.0.0.1:8089").await {
            Ok(l) => l,
            Err(_) => TcpListener::bind("127.0.0.1:0")
                .await
                .map_err(|e| YouTubeAuthError::new(format!("Failed to bind loopback TCP server: {e}")))?,
        };

        let local_addr: SocketAddr = listener
            .local_addr()
            .map_err(|e| YouTubeAuthError::new(format!("Failed to determine local socket address: {e}")))?;

        let port = local_addr.port();
        let redirect_uri = format!("http://127.0.0.1:{port}/oauth2callback");
        let auth_url = self.build_authorization_url(client_id, &redirect_uri);

        info!("Local OAuth loopback server listening on port {port}");

        if let Some(sender) = auth_url_sender {
            let _ = sender.send(auth_url.clone());
        }

        Self::open_browser_safely(&auth_url);

        let (code_tx, code_rx) = tokio::sync::oneshot::channel::<Result<String, YouTubeAuthError>>();

        let server_task = tokio::spawn(async move {
            loop {
                let (mut socket, _) = match listener.accept().await {
                    Ok(conn) => conn,
                    Err(e) => {
                        let _ = code_tx.send(Err(YouTubeAuthError::new(format!("TCP accept error: {e}"))));
                        break;
                    }
                };

                let mut buffer = [0u8; 4096];
                let bytes_read = match socket.read(&mut buffer).await {
                    Ok(n) if n > 0 => n,
                    _ => continue,
                };

                let request_str = String::from_utf8_lossy(&buffer[..bytes_read]);
                let first_line = request_str.lines().next().unwrap_or("");
                let parts: Vec<&str> = first_line.split_whitespace().collect();

                if parts.len() < 2 {
                    continue;
                }

                let path_and_query = parts[1];
                if path_and_query == "/favicon.ico" {
                    let resp = "HTTP/1.1 204 No Content\r\nConnection: close\r\n\r\n";
                    let _ = socket.write_all(resp.as_bytes()).await;
                    let _ = socket.shutdown().await;
                    continue;
                }

                let query_string = path_and_query.split_once('?').map(|(_, q)| q).unwrap_or("");
                let params = Self::parse_query_params(query_string);

                if let Some(err) = params.get("error") {
                    let desc = params.get("error_description").cloned().unwrap_or_else(|| err.clone());
                    let html = Self::build_error_html(&desc);
                    let resp = format!(
                        "HTTP/1.1 400 Bad Request\r\nContent-Type: text/html; charset=UTF-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        html.len(),
                        html
                    );
                    let _ = socket.write_all(resp.as_bytes()).await;
                    let _ = socket.shutdown().await;
                    let _ = code_tx.send(Err(YouTubeAuthError::new(format!("OAuth rejected: {desc}"))));
                    break;
                }

                if let Some(code) = params.get("code") {
                    let html = Self::build_success_html();
                    let resp = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=UTF-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        html.len(),
                        html
                    );
                    let _ = socket.write_all(resp.as_bytes()).await;
                    let _ = socket.shutdown().await;
                    let _ = code_tx.send(Ok(code.clone()));
                    break;
                }

                let html = Self::build_error_html("Nenhum código de autorização recebido");
                let resp = format!(
                    "HTTP/1.1 400 Bad Request\r\nContent-Type: text/html; charset=UTF-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    html.len(),
                    html
                );
                let _ = socket.write_all(resp.as_bytes()).await;
                let _ = socket.shutdown().await;
                let _ = code_tx.send(Err(YouTubeAuthError::new("Missing code parameter in callback")));
                break;
            }
        });

        let auth_code = match tokio::time::timeout(timeout, code_rx).await {
            Ok(Ok(Ok(code))) => code,
            Ok(Ok(Err(auth_err))) => return Err(YouTubeError::Auth(auth_err)),
            Ok(Err(_)) => return Err(YouTubeAuthError::new("OAuth callback channel closed").into()),
            Err(_) => {
                server_task.abort();
                return Err(YouTubeAuthError::new(format!(
                    "Timed out waiting for OAuth authorization callback ({}s)",
                    timeout.as_secs()
                ))
                .into());
            }
        };

        self.exchange_code_for_tokens(client_id, client_secret, &auth_code, &redirect_uri)
            .await
    }

    pub async fn exchange_code_for_tokens(
        &self,
        client_id: &str,
        client_secret: &str,
        auth_code: &str,
        redirect_uri: &str,
    ) -> Result<YouTubeOAuthTokens, YouTubeError> {
        let params = [
            ("code", auth_code),
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("redirect_uri", redirect_uri),
            ("grant_type", "authorization_code"),
        ];

        let response = self
            .http_client
            .post(&self.token_endpoint_url)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("Accept", "application/json")
            .form(&params)
            .send()
            .await
            .map_err(YouTubeError::Network)?;

        let status = response.status().as_u16();
        let body = response.text().await.unwrap_or_default();

        if !(200..300).contains(&status) {
            return Err(YouTubeAuthError::new(format!(
                "Failed to exchange authorization code for tokens: HTTP {status} - {body}"
            ))
            .into());
        }

        let root: Value = serde_json::from_str(&body).map_err(YouTubeError::Json)?;
        let access_token = root
            .get("access_token")
            .and_then(|v| v.as_str())
            .filter(|s| !s.trim().is_empty())
            .ok_or_else(|| YouTubeAuthError::new(format!("Token response missing access_token: {body}")))?;

        let refresh_token = root.get("refresh_token").and_then(|v| v.as_str()).map(|s| s.to_string());
        let expires_in = root.get("expires_in").and_then(|v| v.as_u64()).unwrap_or(3600);
        let scope = root.get("scope").and_then(|v| v.as_str()).unwrap_or("").to_string();

        Ok(YouTubeOAuthTokens::new(
            access_token,
            refresh_token,
            expires_in,
            Some(chrono::Utc::now()),
            Some(scope),
        ))
    }

    pub async fn refresh_access_token(
        &self,
        client_id: &str,
        client_secret: &str,
        refresh_token: &str,
    ) -> Result<YouTubeOAuthTokens, YouTubeError> {
        let params = [
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
        ];

        let response = self
            .http_client
            .post(&self.token_endpoint_url)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("Accept", "application/json")
            .form(&params)
            .send()
            .await
            .map_err(YouTubeError::Network)?;

        let status = response.status().as_u16();
        let body = response.text().await.unwrap_or_default();

        if !(200..300).contains(&status) {
            return Err(YouTubeAuthError::new(format!(
                "Failed to refresh OAuth access token: HTTP {status} - {body}"
            ))
            .into());
        }

        let root: Value = serde_json::from_str(&body).map_err(YouTubeError::Json)?;
        let new_access_token = root
            .get("access_token")
            .and_then(|v| v.as_str())
            .filter(|s| !s.trim().is_empty())
            .ok_or_else(|| YouTubeAuthError::new(format!("Token refresh missing access_token: {body}")))?;

        let returned_refresh_token = root
            .get("refresh_token")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| Some(refresh_token.to_string()));

        let expires_in = root.get("expires_in").and_then(|v| v.as_u64()).unwrap_or(3600);
        let scope = root.get("scope").and_then(|v| v.as_str()).unwrap_or("").to_string();

        Ok(YouTubeOAuthTokens::new(
            new_access_token,
            returned_refresh_token,
            expires_in,
            Some(chrono::Utc::now()),
            Some(scope),
        ))
    }

    fn parse_query_params(query: &str) -> HashMap<String, String> {
        let mut map = HashMap::new();
        for (k, v) in url::form_urlencoded::parse(query.as_bytes()) {
            map.insert(k.into_owned(), v.into_owned());
        }
        map
    }


    fn open_browser_safely(url: &str) {
        #[cfg(target_os = "linux")]
        {
            let _ = std::process::Command::new("xdg-open").arg(url).spawn();
        }
        #[cfg(target_os = "macos")]
        {
            let _ = std::process::Command::new("open").arg(url).spawn();
        }
        #[cfg(target_os = "windows")]
        {
            let _ = std::process::Command::new("cmd").args(["/C", "start", url]).spawn();
        }
        info!("Opened browser or printed auth URL: {url}");
    }

    pub fn build_success_html() -> &'static str {
        r##"<!DOCTYPE html>
<html lang="pt-BR">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Clippr - YouTube Conectado</title>
    <style>
        body {
            margin: 0;
            padding: 0;
            min-height: 100vh;
            display: flex;
            align-items: center;
            justify-content: center;
            background: linear-gradient(135deg, #0b0f19 0%, #111827 100%);
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
            color: #f8fafc;
        }
        .card {
            background: rgba(30, 41, 59, 0.85);
            backdrop-filter: blur(12px);
            border: 1px solid rgba(255, 255, 255, 0.1);
            border-radius: 16px;
            padding: 40px;
            max-width: 440px;
            width: 90%;
            text-align: center;
            box-shadow: 0 20px 40px rgba(0, 0, 0, 0.4);
        }
        .icon-circle {
            width: 72px;
            height: 72px;
            border-radius: 50%;
            background: rgba(34, 197, 94, 0.15);
            border: 2px solid #22c55e;
            display: inline-flex;
            align-items: center;
            justify-content: center;
            margin-bottom: 24px;
            color: #22c55e;
        }
        h1 {
            font-size: 22px;
            font-weight: 700;
            margin: 0 0 12px;
            letter-spacing: -0.5px;
        }
        p {
            font-size: 15px;
            color: #94a3b8;
            line-height: 1.6;
            margin: 0 0 24px;
        }
        .badge {
            display: inline-block;
            padding: 8px 16px;
            background: rgba(99, 102, 241, 0.15);
            border: 1px solid rgba(99, 102, 241, 0.3);
            border-radius: 8px;
            font-size: 13px;
            color: #a5b4fc;
            font-weight: 500;
        }
    </style>
</head>
<body>
    <div class="card">
        <div class="icon-circle">
            <svg width="36" height="36" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
                <polyline points="20 6 9 17 4 12"></polyline>
            </svg>
        </div>
        <h1>Autenticação Concluída!</h1>
        <p>Sua conta do YouTube foi conectada com sucesso ao <strong>Clippr</strong>. O aplicativo agora está autorizado a gerenciar envios de vídeo.</p>
        <div class="badge">Você já pode fechar esta aba do navegador.</div>
    </div>
</body>
</html>"##
    }

    pub fn build_error_html(error_message: &str) -> String {
        let safe_error = error_message.replace('<', "&lt;").replace('>', "&gt;");
        format!(
            r##"<!DOCTYPE html>
<html lang="pt-BR">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Clippr - Falha na Autenticação</title>
    <style>
        body {{
            margin: 0;
            padding: 0;
            min-height: 100vh;
            display: flex;
            align-items: center;
            justify-content: center;
            background: linear-gradient(135deg, #0b0f19 0%, #111827 100%);
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
            color: #f8fafc;
        }}
        .card {{
            background: rgba(30, 41, 59, 0.85);
            backdrop-filter: blur(12px);
            border: 1px solid rgba(239, 68, 68, 0.2);
            border-radius: 16px;
            padding: 40px;
            max-width: 440px;
            width: 90%;
            text-align: center;
            box-shadow: 0 20px 40px rgba(0, 0, 0, 0.4);
        }}
        .icon-circle {{
            width: 72px;
            height: 72px;
            border-radius: 50%;
            background: rgba(239, 68, 68, 0.15);
            border: 2px solid #ef4444;
            display: inline-flex;
            align-items: center;
            justify-content: center;
            margin-bottom: 24px;
            color: #ef4444;
        }}
        h1 {{
            font-size: 22px;
            font-weight: 700;
            margin: 0 0 12px;
            letter-spacing: -0.5px;
        }}
        p {{
            font-size: 15px;
            color: #94a3b8;
            line-height: 1.6;
            margin: 0 0 20px;
        }}
        .error-details {{
            background: rgba(15, 23, 42, 0.6);
            border: 1px solid rgba(255, 255, 255, 0.05);
            border-radius: 8px;
            padding: 12px;
            font-family: monospace;
            font-size: 13px;
            color: #f87171;
            margin-bottom: 24px;
            word-break: break-all;
        }}
        .badge {{
            display: inline-block;
            padding: 8px 16px;
            background: rgba(148, 163, 184, 0.1);
            border-radius: 8px;
            font-size: 13px;
            color: #94a3b8;
        }}
    </style>
</head>
<body>
    <div class="card">
        <div class="icon-circle">
            <svg width="36" height="36" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
                <circle cx="12" cy="12" r="10"></circle>
                <line x1="15" y1="9" x2="9" y2="15"></line>
                <line x1="9" y1="9" x2="15" y2="15"></line>
            </svg>
        </div>
        <h1>Falha na Autenticação</h1>
        <p>Não foi possível concluir a autorização do YouTube no Clippr.</p>
        <div class="error-details">{safe_error}</div>
        <div class="badge">Você pode fechar esta aba e tentar novamente no Clippr.</div>
    </div>
</body>
</html>"##
        )
    }
}
