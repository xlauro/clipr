use chrono::{Duration as ChronoDuration, Utc};
use clipr_youtube::{UploadProgress, UploadResult, VideoPrivacy, VideoUploadRequest, YouTubeChannelInfo, YouTubeOAuthTokens};
use std::path::PathBuf;
use std::time::Duration;

#[test]
fn test_tokens_expiration_and_refresh() {
    let now = Utc::now();
    // Token obtido há 3500 segundos atrás com validade de 3600s -> resta 100s
    let tokens = YouTubeOAuthTokens::new(
        "sample_access_token",
        Some("sample_refresh_token".to_string()),
        3600,
        Some(now - ChronoDuration::seconds(3500)),
        Some("https://www.googleapis.com/auth/youtube.upload".to_string()),
    );

    // Com buffer padrão de 60s, 100s > 60s -> ainda não está expirado
    assert!(!tokens.is_expired());
    // Com buffer de 120s, 100s < 120s -> expirado pelo buffer
    assert!(tokens.is_expired_with_buffer(Duration::from_secs(120)));
    assert!(tokens.remaining_seconds() <= 100 && tokens.remaining_seconds() >= 95);

    // Atualiza o token
    let refreshed = tokens.with_refreshed_token("new_access_token", 3600, None);
    assert_eq!(refreshed.access_token, "new_access_token");
    assert_eq!(refreshed.refresh_token, Some("sample_refresh_token".to_string()));
    assert_eq!(refreshed.scope, tokens.scope);
    assert!(!refreshed.is_expired());
    assert!(refreshed.remaining_seconds() >= 3590);
}

#[test]
fn test_expired_token_lifetime() {
    let now = Utc::now();
    let expired = YouTubeOAuthTokens::new(
        "expired_token",
        None,
        3600,
        Some(now - ChronoDuration::seconds(4000)),
        None,
    );

    assert!(expired.is_expired());
    assert_eq!(expired.remaining_lifetime(), Duration::ZERO);
    assert_eq!(expired.remaining_seconds(), 0);
}

#[test]
fn test_video_privacy() {
    assert_eq!(VideoPrivacy::Public.api_value(), "public");
    assert_eq!(VideoPrivacy::Unlisted.api_value(), "unlisted");
    assert_eq!(VideoPrivacy::Private.api_value(), "private");

    assert_eq!(VideoPrivacy::Public.display_name(), "Público");
    assert_eq!(VideoPrivacy::Unlisted.display_name(), "Não listado");
    assert_eq!(VideoPrivacy::Private.display_name(), "Privado");

    assert_eq!(VideoPrivacy::from_str_loose("public"), VideoPrivacy::Public);
    assert_eq!(VideoPrivacy::from_str_loose("PÚBLICO"), VideoPrivacy::Public);
    assert_eq!(VideoPrivacy::from_str_loose("unlisted"), VideoPrivacy::Unlisted);
    assert_eq!(VideoPrivacy::from_str_loose("não listado"), VideoPrivacy::Unlisted);
    assert_eq!(VideoPrivacy::from_str_loose("anything_else"), VideoPrivacy::Private);

    // Serialization
    let json_str = serde_json::to_string(&VideoPrivacy::Public).unwrap();
    assert_eq!(json_str, "\"public\"");
    let parsed: VideoPrivacy = serde_json::from_str("\"unlisted\"").unwrap();
    assert_eq!(parsed, VideoPrivacy::Unlisted);
}

#[test]
fn test_upload_progress() {
    let started = UploadProgress::started(1000);
    assert_eq!(started.bytes_uploaded, 0);
    assert_eq!(started.total_bytes, 1000);
    assert_eq!(started.percentage, 0.0);
    assert!(!started.is_complete);
    assert!(started.video_id.is_none());

    let in_progress = UploadProgress::in_progress(500, 1000);
    assert_eq!(in_progress.bytes_uploaded, 500);
    assert_eq!(in_progress.percentage, 50.0);
    assert!(!in_progress.is_complete);

    let complete = UploadProgress::complete(1000, "vid123");
    assert_eq!(complete.bytes_uploaded, 1000);
    assert_eq!(complete.percentage, 100.0);
    assert!(complete.is_complete);
    assert_eq!(complete.video_id.as_deref(), Some("vid123"));
}

#[test]
fn test_upload_result() {
    let res = UploadResult::new("xyz890", "Meu Short Viral", VideoPrivacy::Public);
    assert_eq!(res.video_id, "xyz890");
    assert_eq!(res.video_url, "https://www.youtube.com/watch?v=xyz890");
    assert_eq!(res.title, "Meu Short Viral");
    assert_eq!(res.privacy, VideoPrivacy::Public);

    let custom = UploadResult::with_custom_url("xyz890", "https://youtu.be/xyz890", "Short", VideoPrivacy::Unlisted);
    assert_eq!(custom.video_url, "https://youtu.be/xyz890");
    assert_eq!(custom.privacy, VideoPrivacy::Unlisted);
}

#[test]
fn test_channel_info_and_upload_request() {
    let channel = YouTubeChannelInfo::new(
        "UC123456",
        "Canal de Cortes",
        "Melhores cortes e podcasts",
        Some("@canalcortes".to_string()),
        Some("https://example.com/avatar.jpg".to_string()),
        50000,
    );
    assert_eq!(channel.id, "UC123456");
    assert_eq!(channel.subscriber_count, 50000);

    let req = VideoUploadRequest::with_options(
        PathBuf::from("/tmp/video.mp4"),
        "Corte 1",
        "Descrição do vídeo",
        vec!["shorts".to_string(), "viral".to_string()],
        VideoPrivacy::Public,
        false,
        Some(PathBuf::from("/tmp/thumb.jpg")),
    );
    assert_eq!(req.title, "Corte 1");
    assert_eq!(req.tags.len(), 2);
    assert_eq!(req.privacy, VideoPrivacy::Public);
    assert!(!req.made_for_kids);
    assert_eq!(req.custom_thumbnail, Some(PathBuf::from("/tmp/thumb.jpg")));
}
