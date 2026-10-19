use crate::error::{YouTubeApiError, YouTubeError};
use crate::model::{UploadProgress, UploadResult, VideoUploadRequest, YouTubeChannelInfo};
use bytes::Bytes;
use futures_util::stream::Stream;
use reqwest::Client as HttpClient;
use serde_json::{json, Value};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tracing::{info, warn};

pub const DEFAULT_API_BASE_URL: &str = "https://www.googleapis.com";
pub const DEFAULT_UPLOAD_BASE_URL: &str = "https://www.googleapis.com";

pub trait YouTubeClient: Send + Sync {
    fn get_channel_info(
        &self,
        access_token: &str,
    ) -> impl std::future::Future<Output = Result<YouTubeChannelInfo, YouTubeError>> + Send;

    fn upload_video(
        &self,
        access_token: &str,
        request: &VideoUploadRequest,
        progress_callback: Option<Arc<dyn Fn(UploadProgress) + Send + Sync>>,
    ) -> impl std::future::Future<Output = Result<UploadResult, YouTubeError>> + Send;
}

#[derive(Clone)]
pub struct YouTubeApiClient {
    http_client: HttpClient,
    api_base_url: String,
    upload_base_url: String,
}

impl Default for YouTubeApiClient {
    fn default() -> Self {
        Self::new()
    }
}

impl YouTubeApiClient {
    pub fn new() -> Self {
        Self::with_urls(DEFAULT_API_BASE_URL, DEFAULT_UPLOAD_BASE_URL)
    }

    pub fn with_urls(api_base_url: impl Into<String>, upload_base_url: impl Into<String>) -> Self {
        let http_client = HttpClient::builder()
            .timeout(Duration::from_secs(300))
            .connect_timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_default();

        Self {
            http_client,
            api_base_url: api_base_url.into(),
            upload_base_url: upload_base_url.into(),
        }
    }

    pub fn api_base_url(&self) -> &str {
        &self.api_base_url
    }

    pub fn upload_base_url(&self) -> &str {
        &self.upload_base_url
    }

    pub async fn get_channel_info(&self, access_token: &str) -> Result<YouTubeChannelInfo, YouTubeError> {
        let url = format!("{}/youtube/v3/channels?part=snippet,statistics&mine=true", self.api_base_url);

        let resp = self
            .http_client
            .get(&url)
            .header("Authorization", format!("Bearer {access_token}"))
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(YouTubeError::Network)?;

        let status = resp.status().as_u16();
        let body = resp.text().await.unwrap_or_default();

        if !(200..300).contains(&status) {
            return Err(YouTubeApiError::new(
                status,
                format!("Failed to fetch channel info: HTTP {status} - {body}"),
            )
            .into());
        }

        let root: Value = serde_json::from_str(&body).map_err(YouTubeError::Json)?;
        let items = root.get("items").and_then(|i| i.as_array());

        let item = match items.and_then(|arr| arr.first()) {
            Some(it) => it,
            None => {
                return Err(YouTubeApiError::with_message(
                    "No YouTube channel found for the authenticated account",
                )
                .into());
            }
        };

        let id = item.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let snippet = item.get("snippet");
        let title = snippet.and_then(|s| s.get("title")).and_then(|v| v.as_str()).unwrap_or("").to_string();
        let description = snippet.and_then(|s| s.get("description")).and_then(|v| v.as_str()).unwrap_or("").to_string();
        let custom_url = snippet.and_then(|s| s.get("customUrl")).and_then(|v| v.as_str()).map(|s| s.to_string());

        let avatar_url = snippet
            .and_then(|s| s.get("thumbnails"))
            .and_then(|t| {
                t.pointer("/high/url")
                    .or_else(|| t.pointer("/medium/url"))
                    .or_else(|| t.pointer("/default/url"))
            })
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let subscriber_count = item
            .pointer("/statistics/subscriberCount")
            .and_then(|v| {
                if let Some(n) = v.as_u64() {
                    Some(n)
                } else if let Some(s) = v.as_str() {
                    s.parse::<u64>().ok()
                } else {
                    None
                }
            })
            .unwrap_or(0);

        Ok(YouTubeChannelInfo::new(
            id,
            title,
            description,
            custom_url,
            avatar_url,
            subscriber_count,
        ))
    }

    pub async fn upload_video(
        &self,
        access_token: &str,
        request: &VideoUploadRequest,
        progress_callback: Option<Arc<dyn Fn(UploadProgress) + Send + Sync>>,
    ) -> Result<UploadResult, YouTubeError> {
        if !request.video_file.exists() {
            return Err(YouTubeApiError::with_message(format!(
                "Video file does not exist: {}",
                request.video_file.display()
            ))
            .into());
        }

        let file_size = std::fs::metadata(&request.video_file)
            .map_err(|e| YouTubeApiError::with_message(format!("Unable to determine video file size: {e}")))?
            .len();

        if file_size == 0 {
            return Err(YouTubeApiError::with_message("Video file is empty (0 bytes)").into());
        }

        if let Some(ref cb) = progress_callback {
            cb(UploadProgress::started(file_size));
        }

        // 1. Iniciar sessão de upload resumable
        let upload_url = self.initiate_resumable_upload(access_token, request, file_size).await?;

        // 2. Fazer PUT com stream e tracking de progresso
        let video_id = self.upload_video_binary(&upload_url, &request.video_file, file_size, progress_callback.clone()).await?;

        // 3. Upload de thumbnail customizada se fornecida
        if let Some(ref thumb_path) = request.custom_thumbnail
            && thumb_path.exists() {
                self.upload_thumbnail(access_token, &video_id, thumb_path).await;
            }

        if let Some(ref cb) = progress_callback {
            cb(UploadProgress::complete(file_size, &video_id));
        }

        Ok(UploadResult::new(video_id, &request.title, request.privacy))
    }

    async fn initiate_resumable_upload(
        &self,
        access_token: &str,
        request: &VideoUploadRequest,
        file_size: u64,
    ) -> Result<String, YouTubeError> {
        let init_url = format!("{}/upload/youtube/v3/videos?uploadType=resumable&part=snippet,status", self.upload_base_url);

        let metadata = json!({
            "snippet": {
                "title": request.title,
                "description": request.description,
                "categoryId": "22",
                "tags": request.tags,
            },
            "status": {
                "privacyStatus": request.privacy.api_value(),
                "selfDeclaredMadeForKids": request.made_for_kids,
                "embeddable": true,
            }
        });

        let resp = self
            .http_client
            .post(&init_url)
            .header("Authorization", format!("Bearer {access_token}"))
            .header("Content-Type", "application/json; charset=UTF-8")
            .header("X-Upload-Content-Type", "video/mp4")
            .header("X-Upload-Content-Length", file_size.to_string())
            .json(&metadata)
            .send()
            .await
            .map_err(YouTubeError::Network)?;

        let status = resp.status().as_u16();
        if !(200..300).contains(&status) {
            let body = resp.text().await.unwrap_or_default();
            return Err(YouTubeApiError::new(
                status,
                format!("Failed to initiate resumable video upload: HTTP {status} - {body}"),
            )
            .into());
        }

        let location = resp
            .headers()
            .get("Location")
            .or_else(|| resp.headers().get("location"))
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| {
                YouTubeApiError::with_message("YouTube API did not return Location header for resumable upload session")
            })?
            .to_string();

        Ok(location)
    }

    async fn upload_video_binary(
        &self,
        upload_url: &str,
        video_path: &Path,
        file_size: u64,
        progress_callback: Option<Arc<dyn Fn(UploadProgress) + Send + Sync>>,
    ) -> Result<String, YouTubeError> {
        let stream = create_progress_stream(video_path, file_size, progress_callback).await?;
        let body = reqwest::Body::wrap_stream(stream);

        let resp = self
            .http_client
            .put(upload_url)
            .header("Content-Type", "video/mp4")
            .header("Content-Length", file_size.to_string())
            .body(body)
            .send()
            .await
            .map_err(YouTubeError::Network)?;

        let status = resp.status().as_u16();
        let body_str = resp.text().await.unwrap_or_default();

        if !(200..300).contains(&status) {
            return Err(YouTubeApiError::new(
                status,
                format!("Failed to upload video media: HTTP {status} - {body_str}"),
            )
            .into());
        }

        let root: Value = serde_json::from_str(&body_str).map_err(YouTubeError::Json)?;
        let video_id = root
            .get("id")
            .and_then(|v| v.as_str())
            .filter(|s| !s.trim().is_empty())
            .ok_or_else(|| YouTubeApiError::with_message(format!("Upload response missing video ID: {body_str}")))?;

        Ok(video_id.to_string())
    }

    async fn upload_thumbnail(&self, access_token: &str, video_id: &str, thumbnail_path: &Path) {
        let content_type = match thumbnail_path.extension().and_then(|e| e.to_str()).map(|s| s.to_lowercase()) {
            Some(ext) if ext == "png" => "image/png",
            _ => "image/jpeg",
        };

        let thumb_url = format!(
            "{}/upload/youtube/v3/thumbnails/set?videoId={}",
            self.upload_base_url,
            urlencoding_simple(video_id)
        );

        let file_bytes = match tokio::fs::read(thumbnail_path).await {
            Ok(b) => b,
            Err(e) => {
                warn!("Could not read thumbnail file for video {video_id}: {e}");
                return;
            }
        };

        let res = self
            .http_client
            .post(&thumb_url)
            .header("Authorization", format!("Bearer {access_token}"))
            .header("Content-Type", content_type)
            .body(file_bytes)
            .send()
            .await;

        match res {
            Ok(resp) if resp.status().is_success() => {
                info!("Successfully uploaded custom thumbnail for video {video_id}");
            }
            Ok(resp) => {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                warn!("Failed to set custom thumbnail for video {video_id} (HTTP {status}): {body}");
            }
            Err(e) => {
                warn!("Error uploading thumbnail for video {video_id}: {e}");
            }
        }
    }
}

impl YouTubeClient for YouTubeApiClient {
    async fn get_channel_info(&self, access_token: &str) -> Result<YouTubeChannelInfo, YouTubeError> {
        self.get_channel_info(access_token).await
    }

    async fn upload_video(
        &self,
        access_token: &str,
        request: &VideoUploadRequest,
        progress_callback: Option<Arc<dyn Fn(UploadProgress) + Send + Sync>>,
    ) -> Result<UploadResult, YouTubeError> {
        self.upload_video(access_token, request, progress_callback).await
    }
}

async fn create_progress_stream(
    path: &Path,
    total_bytes: u64,
    progress_callback: Option<Arc<dyn Fn(UploadProgress) + Send + Sync>>,
) -> Result<impl Stream<Item = Result<Bytes, std::io::Error>> + Send + 'static, YouTubeError> {
    let file = tokio::fs::File::open(path)
        .await
        .map_err(YouTubeError::Io)?;

    let bytes_uploaded = Arc::new(AtomicU64::new(0));

    // Lê chunks de 512 KB
    const CHUNK_SIZE: usize = 512 * 1024;
    let stream = futures_util::stream::unfold((file, bytes_uploaded, progress_callback), move |(mut file, uploaded, cb)| async move {
        let mut buf = vec![0u8; CHUNK_SIZE];
        match file.read(&mut buf).await {
            Ok(0) => None,
            Ok(n) => {
                buf.truncate(n);
                let current = uploaded.fetch_add(n as u64, Ordering::Relaxed) + n as u64;
                if let Some(ref callback) = cb {
                    callback(UploadProgress::in_progress(current, total_bytes));
                }
                Some((Ok(Bytes::from(buf)), (file, uploaded, cb)))
            }
            Err(e) => Some((Err(e), (file, uploaded, cb))),
        }
    });

    Ok(stream)
}

fn urlencoding_simple(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}
