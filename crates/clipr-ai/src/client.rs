use crate::cache::AnalysisResult;
use crate::error::AiError;
use clipr_domain::TranscriptSegment;

pub trait AiClient: Send + Sync {
    fn analyze_transcript(
        &self,
        video_source_id: Option<i64>,
        video_title: &str,
        video_author: &str,
        segments: &[TranscriptSegment],
        min_virality_threshold: i32,
    ) -> impl std::future::Future<Output = Result<AnalysisResult, AiError>> + Send;

    fn analyze_video_url(
        &self,
        _video_source_id: Option<i64>,
        _video_url: &str,
        _video_title: &str,
        _video_author: &str,
        _optional_segments: Option<&[TranscriptSegment]>,
        _min_virality_threshold: i32,
    ) -> impl std::future::Future<Output = Result<AnalysisResult, AiError>> + Send {
        async {
            Err(AiError::Unsupported(
                "Análise multimodal de vídeo não suportada por este provedor de IA.".to_string(),
            ))
        }
    }
}
