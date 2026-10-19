use crate::cache::{AnalysisResult, TranscriptAnalysisCache};
use crate::client::AiClient;
use crate::error::{AiError, GeminiApiError};
use crate::prompt::PromptFactory;
use clipr_domain::{
    ClipCandidate, EmotionTag, EventBus, FramingMode, OrientationMode, ProcessingStatus,
    ScoreBreakdown, SubtitlePreset, TranscriptSegment,
};
use regex::Regex;
use reqwest::Client as HttpClient;
use serde_json::{json, Value};
use std::sync::LazyLock;
use std::time::Duration;
use tracing::{debug, error, info, warn};

pub const DEFAULT_MODEL: &str = "gemini-2.5-flash";
pub const DEFAULT_FALLBACK_MODEL: &str = "gemini-1.5-flash";
pub const DEFAULT_BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta/models";

static CODE_BLOCK_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)```(?:json|JSON)?\s*(.*?)\s*```").unwrap()
});

static UNQUOTED_TIMESTAMP_KEY_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)("(?:start|end|thumbnail)[a-z_]*"\s*:\s*)(\d{1,2}:\d{2}(?::\d{2})?(?:[.,]\d+)?)(\s*[,}\]\r\n])"#).unwrap()
});

static UNQUOTED_TIMESTAMP_GENERAL_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(:\s*)(\d{1,2}:\d{2}(?::\d{2})?(?:[.,]\d+)?)(\s*[,}\]\r\n])"#).unwrap()
});

static COMMA_DECIMAL_NUMBER_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(:\s*)(\d+),(\d+)(\s*[,}\]\r\n])"#).unwrap()
});

static TRAILING_COMMA_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#",\s*([}\]])"#).unwrap()
});

#[derive(Clone)]
pub struct GeminiClient {
    api_key: String,
    model_name: String,
    fallback_model_name: String,
    http_client: HttpClient,
    attempts_per_model: usize,
    max_cycles: usize,
    initial_backoff_ms: u64,
    max_backoff_ms: u64,
    initial_429_backoff_ms: u64,
    max_429_backoff_ms: u64,
    base_url: String,
}

impl GeminiClient {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self::with_model(api_key, DEFAULT_MODEL)
    }

    pub fn with_model(api_key: impl Into<String>, model_name: impl Into<String>) -> Self {
        let model = model_name.into();
        let fallback = if model == DEFAULT_FALLBACK_MODEL {
            DEFAULT_MODEL.to_string()
        } else {
            DEFAULT_FALLBACK_MODEL.to_string()
        };

        Self::with_config(
            api_key,
            if model.trim().is_empty() { DEFAULT_MODEL } else { &model },
            &fallback,
            10,
            5,
            800,
            3500,
            2000,
            60000,
            DEFAULT_BASE_URL,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn with_config(
        api_key: impl Into<String>,
        model_name: &str,
        fallback_model_name: &str,
        attempts_per_model: usize,
        max_cycles: usize,
        initial_backoff_ms: u64,
        max_backoff_ms: u64,
        initial_429_backoff_ms: u64,
        max_429_backoff_ms: u64,
        base_url: &str,
    ) -> Self {
        let http_client = HttpClient::builder()
            .timeout(Duration::from_secs(120))
            .connect_timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_default();

        Self {
            api_key: api_key.into().trim().to_string(),
            model_name: if model_name.trim().is_empty() {
                DEFAULT_MODEL.to_string()
            } else {
                model_name.trim().to_string()
            },
            fallback_model_name: if fallback_model_name.trim().is_empty() {
                DEFAULT_FALLBACK_MODEL.to_string()
            } else {
                fallback_model_name.trim().to_string()
            },
            http_client,
            attempts_per_model: attempts_per_model.max(1),
            max_cycles: max_cycles.max(1),
            initial_backoff_ms: initial_backoff_ms.max(1),
            max_backoff_ms: max_backoff_ms.max(1),
            initial_429_backoff_ms: initial_429_backoff_ms.max(1),
            max_429_backoff_ms: max_429_backoff_ms.max(1),
            base_url: if base_url.trim().is_empty() {
                DEFAULT_BASE_URL.to_string()
            } else {
                base_url.trim().to_string()
            },
        }
    }

    pub fn model_name(&self) -> &str {
        &self.model_name
    }

    pub fn fallback_model_name(&self) -> &str {
        &self.fallback_model_name
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn clean_json_content(text: &str) -> String {
        if text.trim().is_empty() {
            return String::new();
        }

        let mut cleaned = text.trim().to_string();

        if let Some(caps) = CODE_BLOCK_PATTERN.captures(&cleaned) {
            if let Some(m) = caps.get(1) {
                cleaned = m.as_str().trim().to_string();
            }
        } else {
            if cleaned.starts_with("```json") || cleaned.starts_with("```JSON") {
                cleaned = cleaned[7..].trim().to_string();
            } else if cleaned.starts_with("```") {
                cleaned = cleaned[3..].trim().to_string();
            }
            if cleaned.ends_with("```") {
                cleaned = cleaned[..cleaned.len() - 3].trim().to_string();
            }
        }

        let first_bracket = cleaned.find('[');
        let first_brace = cleaned.find('{');

        let start_idx = match (first_bracket, first_brace) {
            (Some(b), Some(r)) => Some(b.min(r)),
            (Some(b), None) => Some(b),
            (None, Some(r)) => Some(r),
            (None, None) => None,
        };

        if let Some(start) = start_idx {
            let is_array = Some(start) == first_bracket;
            let end_idx = if is_array {
                cleaned.rfind(']')
            } else {
                cleaned.rfind('}')
            };

            if let Some(end) = end_idx
                && end > start {
                    cleaned = cleaned[start..=end].trim().to_string();
                }
        }

        cleaned = UNQUOTED_TIMESTAMP_KEY_PATTERN.replace_all(&cleaned, "${1}\"${2}\"${3}").to_string();
        cleaned = UNQUOTED_TIMESTAMP_GENERAL_PATTERN.replace_all(&cleaned, "${1}\"${2}\"${3}").to_string();
        cleaned = COMMA_DECIMAL_NUMBER_PATTERN.replace_all(&cleaned, "${1}${2}.${3}${4}").to_string();
        cleaned = TRAILING_COMMA_PATTERN.replace_all(&cleaned, "$1").to_string();

        cleaned.trim().to_string()
    }

    pub fn parse_flexible_timestamp(text: &str, default_value: f64) -> f64 {
        let clean = text.trim().replace(',', ".");
        let clean = clean.trim_end_matches(['s', 'S']).trim();
        if clean.is_empty() {
            return default_value;
        }

        if let Ok(val) = clean.parse::<f64>() {
            return val;
        }

        let parts: Vec<&str> = clean.split(':').collect();
        if parts.len() == 3 {
            if let (Ok(h), Ok(m), Ok(s)) = (
                parts[0].trim().parse::<f64>(),
                parts[1].trim().parse::<f64>(),
                parts[2].trim().parse::<f64>(),
            ) {
                return (h * 3600.0) + (m * 60.0) + s;
            }
        } else if parts.len() == 2
            && let (Ok(m), Ok(s)) = (parts[0].trim().parse::<f64>(), parts[1].trim().parse::<f64>()) {
                return (m * 60.0) + s;
            }

        default_value
    }

    pub fn parse_retry_after(retry_after_header: Option<&str>) -> Option<Duration> {
        let raw = retry_after_header?.trim();
        if raw.is_empty() {
            return None;
        }

        let clean = raw.trim_end_matches(['s', 'S']).trim();
        if let Ok(secs) = clean.parse::<f64>() {
            return Some(Duration::from_millis((secs.max(0.0) * 1000.0) as u64));
        }

        if let Ok(dt) = chrono::DateTime::parse_from_rfc2822(raw) {
            let now = chrono::Utc::now();
            let diff = dt.with_timezone(&chrono::Utc) - now;
            let millis = diff.num_milliseconds().max(0);
            return Some(Duration::from_millis(millis as u64));
        }

        None
    }

    pub fn snap_to_segment_boundaries(
        &self,
        candidates: &[ClipCandidate],
        segments: &[TranscriptSegment],
    ) -> Vec<ClipCandidate> {
        if candidates.is_empty() {
            return Vec::new();
        }

        if segments.is_empty() {
            return candidates
                .iter()
                .map(|c| {
                    let start = c.start_seconds.max(0.0);
                    let mut end = c.end_seconds;
                    if end <= start {
                        end = start + c.duration_seconds().max(1.0);
                    }
                    let thumb = match c.thumbnail_timestamp {
                        Some(ts) if ts >= start && ts <= end => ts,
                        _ => start,
                    };
                    c.with_bounds(start, end).with_thumbnail(
                        Some(thumb),
                        c.thumbnail_headline.clone(),
                        c.thumbnail_path.clone(),
                        c.raw_frame_path.clone(),
                    )
                })
                .collect();
        }

        let max_seg_end = segments
            .iter()
            .map(|s| s.end_seconds)
            .fold(f64::MIN, f64::max);

        let find_closest_boundary = |target: f64, is_start: bool| -> f64 {
            let mut closest = target;
            let mut min_diff = f64::MAX;

            for seg in segments {
                let candidate = if is_start {
                    seg.start_seconds
                } else {
                    seg.end_seconds
                };
                let diff = (candidate - target).abs();
                if diff < min_diff && diff <= 3.0 {
                    min_diff = diff;
                    closest = candidate;
                }
            }
            closest
        };

        candidates
            .iter()
            .map(|c| {
                let raw_start = c.start_seconds.max(0.0);
                let mut raw_end = c.end_seconds;
                if raw_end <= raw_start {
                    raw_end = raw_start + c.duration_seconds().max(1.0);
                }

                let mut start = find_closest_boundary(raw_start, true).max(0.0);
                let mut end = find_closest_boundary(raw_end, false);

                if end > max_seg_end {
                    end = max_seg_end;
                }

                if end <= start {
                    if start + c.duration_seconds() <= max_seg_end {
                        end = start + c.duration_seconds().max(1.0);
                    } else {
                        end = max_seg_end;
                        start = (end - 1.0).max(0.0).min(start);
                    }
                }

                let thumb_ts = match c.thumbnail_timestamp {
                    Some(ts) if ts >= start && ts <= end => ts,
                    _ => start,
                };

                let mut updated = c.clone();
                updated.start_seconds = start;
                updated.end_seconds = end;
                updated.thumbnail_timestamp = Some(thumb_ts);
                updated
            })
            .collect()
    }

    pub fn parse_gemini_response(
        &self,
        video_source_id: Option<i64>,
        json_response: &str,
    ) -> Result<Vec<ClipCandidate>, AiError> {
        if json_response.trim().is_empty() {
            return Err(GeminiApiError::new(400, "Resposta da API Gemini vazia ou nula", None).into());
        }

        let root: Value = serde_json::from_str(json_response).map_err(|e| {
            error!("[Gemini] Falha ao fazer parse do payload externo da API: {e}");
            AiError::Parsing(format!("Falha no parse do payload externo: {e}"))
        })?;

        if root.pointer("/promptFeedback/blockReason")
            .and_then(|v| v.as_str())
            .map(|s| s.eq_ignore_ascii_case("SAFETY"))
            .unwrap_or(false)
        {
            warn!("Prompt bloqueado pelas diretrizes de segurança da IA (promptFeedback.blockReason: SAFETY).");
            return Ok(Vec::new());
        }

        let candidates = root.get("candidates").and_then(|c| c.as_array());
        let first_candidate = match candidates.and_then(|arr| arr.first()) {
            Some(fc) => fc,
            None => {
                return Err(AiError::Parsing("Resposta da IA não contém candidatos válidos".to_string()));
            }
        };

        if first_candidate.get("finishReason")
            .and_then(|f| f.as_str())
            .map(|s| s.eq_ignore_ascii_case("SAFETY"))
            .unwrap_or(false)
        {
            warn!("Resposta da IA bloqueada por políticas de segurança (finishReason: SAFETY).");
            return Ok(Vec::new());
        }

        let text_content = first_candidate
            .pointer("/content/parts/0/text")
            .and_then(|t| t.as_str())
            .unwrap_or("");

        if text_content.trim().is_empty() {
            return Ok(Vec::new());
        }

        let cleaned_text = Self::clean_json_content(text_content);
        let parsed_json: Value = serde_json::from_str(&cleaned_text).map_err(|e| {
            error!("[Gemini] Falha no parse do JSON interno dos candidatos: {e}. Texto limpo: {cleaned_text}");
            AiError::Parsing(format!("Falha ao realizar parse do JSON dos candidatos: {e}"))
        })?;

        let clips_array = if parsed_json.is_array() {
            parsed_json.as_array()
        } else if let Some(clips) = parsed_json.get("clips").and_then(|c| c.as_array()) {
            Some(clips)
        } else {
            parsed_json.get("candidates").and_then(|c| c.as_array())
        };

        let clips_array = match clips_array {
            Some(arr) => arr,
            None => {
                warn!("Estrutura JSON não esperada da IA: {text_content}");
                return Ok(Vec::new());
            }
        };

        let mut list = Vec::new();
        for item in clips_array {
            let title = get_str_field(item, &["title"]).unwrap_or("Corte Viral").to_string();
            let hook = get_str_field(item, &["hook"]).unwrap_or("").to_string();
            let start_seconds = get_timestamp_field(item, &["startSeconds", "start_seconds", "startTime", "start_time", "start"], 0.0);
            let end_seconds = get_timestamp_field(item, &["endSeconds", "end_seconds", "endTime", "end_time", "end"], 0.0);
            let raw_virality = get_i64_field(item, &["viralityScore", "virality_score", "score"]);

            let (score_breakdown, final_virality_score) = if let Some(scores_val) = get_field(item, &["scores", "scoreBreakdown", "score_breakdown"]) {
                if scores_val.is_object() {
                    let default_val = raw_virality.unwrap_or(50) as i32;
                    let hook_score = get_i32_field(scores_val, &["hook"]).unwrap_or(default_val);
                    let retention_score = get_i32_field(scores_val, &["retention"]).unwrap_or(default_val);
                    let emotional_score = get_i32_field(scores_val, &["emotional", "impact"]).unwrap_or(default_val);
                    let storytelling_score = get_i32_field(scores_val, &["storytelling", "narrative"]).unwrap_or(default_val);
                    let standalone_score = get_i32_field(scores_val, &["standalone"]).unwrap_or(default_val);
                    let visual_score = get_i32_field(scores_val, &["visual"]).unwrap_or(default_val);

                    let sb = ScoreBreakdown::new(
                        hook_score,
                        retention_score,
                        emotional_score,
                        storytelling_score,
                        standalone_score,
                        visual_score,
                    );
                    let weighted = sb.calculate_weighted_virality_score();
                    (sb, weighted)
                } else {
                    let val = raw_virality.unwrap_or(50) as i32;
                    (ScoreBreakdown::from_uniform(val), val)
                }
            } else {
                let val = raw_virality.unwrap_or(50) as i32;
                (ScoreBreakdown::from_uniform(val), val)
            };

            let emotion_tag = if let Some(e_str) = get_str_field(item, &["emotionTag", "emotion_tag", "emotion"]) {
                EmotionTag::from_str_loose(e_str, EmotionTag::AltaEnergia)
            } else {
                EmotionTag::AltaEnergia
            };

            let reasoning = get_str_field(item, &["reasoning", "description"]).unwrap_or("").to_string();
            let caption = get_str_field(item, &["suggestedCaption", "suggested_caption", "caption"]).unwrap_or(&title).to_string();

            let mut hashtags = Vec::new();
            if let Some(tags_val) = get_field(item, &["hashtags", "tags"]).and_then(|v| v.as_array()) {
                for t in tags_val {
                    if let Some(tag_str) = t.as_str() {
                        hashtags.push(tag_str.to_string());
                    }
                }
            }

            let thumbnail_timestamp = if let Some(thumb_val) = get_field(item, &["thumbnailTimestamp", "thumbnail_timestamp", "thumbnailTime", "thumbnail_time"]) {
                let ts = parse_node_timestamp(thumb_val, start_seconds);
                if ts >= start_seconds && ts <= end_seconds {
                    Some(ts)
                } else {
                    Some(start_seconds)
                }
            } else {
                Some(start_seconds)
            };

            let thumbnail_headline = get_str_field(item, &["thumbnailHeadline", "thumbnail_headline", "headline"])
                .unwrap_or(&title)
                .to_string();

            let framing_mode = if let Some(fm_str) = get_str_field(item, &["framingMode", "framing_mode"]) {
                FramingMode::from_str_loose(fm_str, FramingMode::AutoCrop)
            } else {
                FramingMode::AutoCrop
            };

            let subtitle_preset = if let Some(sp_str) = get_str_field(item, &["subtitlePreset", "subtitle_preset", "preset"]) {
                SubtitlePreset::from_str_loose(sp_str, SubtitlePreset::HormoziYellow)
            } else {
                SubtitlePreset::HormoziYellow
            };

            let orientation = if framing_mode == FramingMode::Horizontal169 {
                OrientationMode::Horizontal169
            } else {
                OrientationMode::Vertical916
            };

            if end_seconds > start_seconds {
                list.push(ClipCandidate::new(
                    None,
                    video_source_id,
                    title,
                    hook,
                    start_seconds,
                    end_seconds,
                    final_virality_score,
                    reasoning,
                    caption,
                    hashtags,
                    orientation,
                    true,
                    true,
                    None,
                    None,
                    ProcessingStatus::PendingApproval,
                    thumbnail_timestamp,
                    Some(thumbnail_headline),
                    None,
                    None,
                    Some(framing_mode),
                    Some(subtitle_preset),
                    Some(emotion_tag),
                    Some(score_breakdown),
                ));
            }
        }

        Ok(list)
    }

    pub fn partition_and_filter(
        &self,
        candidates: Vec<ClipCandidate>,
        min_virality_threshold: i32,
    ) -> AnalysisResult {
        let mut approved = Vec::new();
        let mut discarded = Vec::new();

        for (i, candidate) in candidates.into_iter().enumerate() {
            let is_approved = candidate.virality_score >= min_virality_threshold;
            let status_str = if is_approved { "APROVADO" } else { "DESCARTADO" };

            let clip_log = format!(
                "[AI-GEMINI] Corte #{}: '{}' | Gancho: '{}' | Nota: {}/100 [Retenção: {}, Emoção: {}, Coesão: {}, Clareza: {}, Ineditismo: {}] | Status: {}",
                i + 1,
                candidate.title,
                candidate.hook,
                candidate.virality_score,
                candidate.scores.retention,
                candidate.scores.emotional,
                candidate.scores.storytelling,
                candidate.scores.standalone,
                candidate.scores.visual,
                status_str,
            );
            EventBus::global().publish_log(&clip_log, false);
            info!("{}", clip_log);

            if is_approved {
                approved.push(candidate);
            } else {
                let discard_msg = format!(
                    "[AI-GEMINI] Corte #{} descartado: Abaixo da nota de corte {}",
                    i + 1,
                    min_virality_threshold
                );
                EventBus::global().publish_log(&discard_msg, false);
                info!("{}", discard_msg);

                let mut discarded_c = candidate;
                discarded_c.status = ProcessingStatus::DiscardedLowScore;
                discarded.push(discarded_c);
            }
        }

        AnalysisResult::new(approved, discarded)
    }

    pub fn build_request_body(&self, user_prompt: &str) -> Value {
        json!({
            "system_instruction": {
                "parts": [
                    { "text": PromptFactory::SYSTEM_INSTRUCTION }
                ]
            },
            "contents": [
                {
                    "parts": [
                        { "text": user_prompt }
                    ]
                }
            ],
            "generationConfig": {
                "responseMimeType": "application/json",
                "response_mime_type": "application/json",
                "temperature": 0.2
            }
        })
    }

    pub fn build_multimodal_request_body(&self, user_prompt: &str, video_url: &str) -> Value {
        json!({
            "system_instruction": {
                "parts": [
                    { "text": PromptFactory::MULTIMODAL_SYSTEM_INSTRUCTION }
                ]
            },
            "contents": [
                {
                    "parts": [
                        {
                            "file_data": {
                                "mime_type": "video/mp4",
                                "file_uri": video_url
                            }
                        },
                        {
                            "text": user_prompt
                        }
                    ]
                }
            ],
            "generationConfig": {
                "responseMimeType": "application/json",
                "response_mime_type": "application/json",
                "temperature": 0.2
            }
        })
    }

    async fn execute_with_retry(
        &self,
        request_body: &Value,
        operation_description: &str,
    ) -> Result<String, AiError> {
        let prompt_summary = serde_json::to_string(request_body).unwrap_or_default();
        let debug_prompt_msg = format!(
            "[AI-GEMINI] [DEBUG] Resumo do payload enviado ({} bytes): {}",
            prompt_summary.len(),
            if prompt_summary.len() > 1000 {
                format!("{}...", &prompt_summary[..1000])
            } else {
                prompt_summary.clone()
            }
        );
        EventBus::global().publish_log(&debug_prompt_msg, true);
        debug!("{}", debug_prompt_msg);

        let mut models = vec![self.model_name.as_str()];
        if !self.fallback_model_name.eq_ignore_ascii_case(&self.model_name) {
            models.push(self.fallback_model_name.as_str());
        }

        let mut last_error_msg = String::new();

        for cycle in 1..=self.max_cycles {
            for (m_idx, current_model) in models.iter().enumerate() {
                let conn_msg = format!(
                    "[AI-GEMINI] Conectando ao modelo {current_model} para {operation_description} (Ciclo {cycle} de {})...",
                    self.max_cycles
                );
                EventBus::global().publish_log(&conn_msg, false);
                info!("{}", conn_msg);

                let endpoint = format!("{}/{current_model}:generateContent?key={}", self.base_url, self.api_key);

                for attempt in 1..=self.attempts_per_model {
                    let req = self.http_client
                        .post(&endpoint)
                        .header("Content-Type", "application/json")
                        .json(request_body);

                    let start_req = std::time::Instant::now();
                    let resp = match req.send().await {
                        Ok(r) => r,
                        Err(e) => {
                            last_error_msg = e.to_string();
                            if attempt == self.attempts_per_model {
                                let err_m = format!("[AI-GEMINI] Falha de conexão no modelo {current_model} na tentativa {attempt}: {e}");
                                EventBus::global().publish_log(&err_m, false);
                                warn!("{}", err_m);
                                break;
                            }
                            let base = self.max_backoff_ms.min((self.initial_backoff_ms as f64 * 1.3_f64.powi(attempt as i32 - 1)) as u64);
                            let backoff = Duration::from_millis(base);
                            let retry_m = format!("[AI-GEMINI] Falha de conexão ({e}). Tentativa {} de {} em {:.1}s...", attempt + 1, self.attempts_per_model, backoff.as_secs_f64());
                            EventBus::global().publish_log(&retry_m, false);
                            warn!("{}", retry_m);
                            tokio::time::sleep(backoff).await;
                            continue;
                        }
                    };

                    let elapsed_ms = start_req.elapsed().as_millis();
                    let status = resp.status().as_u16();

                    if status == 200 {
                        let ok_msg = format!(
                            "[AI-GEMINI] Resposta recebida da API com sucesso (HTTP 200) em {}ms na tentativa {} do ciclo {}!",
                            elapsed_ms, attempt, cycle
                        );
                        EventBus::global().publish_log(&ok_msg, false);
                        info!("{}", ok_msg);

                        let resp_body = resp.text().await.map_err(AiError::Network)?;
                        let debug_resp_msg = format!("[AI-GEMINI] [DEBUG] Resposta bruta da API:\n{resp_body}");
                        EventBus::global().publish_log(&debug_resp_msg, true);
                        debug!("{}", debug_resp_msg);

                        return Ok(resp_body);
                    }

                    let resp_body = resp.text().await.unwrap_or_default();

                    if status == 429 {
                        let retry_after = Self::parse_retry_after(None);
                        let backoff = retry_after.unwrap_or_else(|| {
                            let base = self.max_429_backoff_ms.min((self.initial_429_backoff_ms as f64 * 2.0_f64.powi(attempt as i32 - 1)) as u64);
                            Duration::from_millis(base)
                        });
                        last_error_msg = format!("Limite de requisições excedido (429): {resp_body}");

                        if attempt < self.attempts_per_model {
                            let retry_msg = format!(
                                "[AI-GEMINI] Limite de requisições excedido (HTTP 429) no modelo {current_model} na tentativa {} de {}. Aguardando retry em {:.1}s...",
                                attempt, self.attempts_per_model, backoff.as_secs_f64()
                            );
                            EventBus::global().publish_log(&retry_msg, false);
                            warn!("{}", retry_msg);
                            tokio::time::sleep(backoff).await;
                        } else {
                            let next_model = if m_idx + 1 < models.len() { models[m_idx + 1] } else { models[0] };
                            let switch_msg = format!("[AI-GEMINI] {} tentativas esgotadas por 429 no modelo {current_model}. Alternando para {next_model}...", self.attempts_per_model);
                            EventBus::global().publish_log(&switch_msg, false);
                            warn!("{}", switch_msg);
                            tokio::time::sleep(Duration::from_millis(self.initial_429_backoff_ms.min(500))).await;
                        }
                    } else if status == 503 {
                        last_error_msg = format!("Alta demanda temporária (503): {resp_body}");
                        if attempt < self.attempts_per_model {
                            let base = self.max_backoff_ms.min((self.initial_backoff_ms as f64 * 1.3_f64.powi(attempt as i32 - 1)) as u64);
                            let backoff = Duration::from_millis(base);
                            let retry_msg = format!(
                                "[AI-GEMINI] Servidor sobrecarregado (HTTP 503) no modelo {current_model} na tentativa {} de {}. Aguardando retry em {:.1}s...",
                                attempt, self.attempts_per_model, backoff.as_secs_f64()
                            );
                            EventBus::global().publish_log(&retry_msg, false);
                            warn!("{}", retry_msg);
                            tokio::time::sleep(backoff).await;
                        } else {
                            let next_model = if m_idx + 1 < models.len() { models[m_idx + 1] } else { models[0] };
                            let switch_msg = format!("[AI-GEMINI] {} tentativas esgotadas por 503 no modelo {current_model}. Alternando para {next_model}...", self.attempts_per_model);
                            EventBus::global().publish_log(&switch_msg, false);
                            warn!("{}", switch_msg);
                            tokio::time::sleep(Duration::from_millis(self.initial_backoff_ms.min(500))).await;
                        }
                    } else if status == 401 {
                        let err_msg = format!("[AI-GEMINI] Erro na API Gemini (HTTP 401): Chave da API inválida ou não autorizada. Detalhes: {resp_body}");
                        EventBus::global().publish_log(&err_msg, false);
                        error!("{err_msg}");
                        return Err(GeminiApiError::new(401, err_msg, Some(resp_body)).into());
                    } else if status == 400 {
                        let err_msg = format!("[AI-GEMINI] Erro na API Gemini (HTTP 400): Requisição inválida. Detalhes: {resp_body}");
                        EventBus::global().publish_log(&err_msg, false);
                        error!("{err_msg}");
                        return Err(GeminiApiError::new(400, err_msg, Some(resp_body)).into());
                    } else if status == 403 {
                        let err_msg = format!("[AI-GEMINI] Erro na API Gemini (HTTP 403): Acesso proibido. Detalhes: {resp_body}");
                        EventBus::global().publish_log(&err_msg, false);
                        error!("{err_msg}");
                        return Err(GeminiApiError::new(403, err_msg, Some(resp_body)).into());
                    } else if status == 404 {
                        if m_idx + 1 < models.len() {
                            let next_model = models[m_idx + 1];
                            let switch_msg = format!(
                                "[AI-GEMINI] Modelo '{current_model}' retornou 404 (Não Encontrado). Alternando para modelo de fallback '{next_model}'..."
                            );
                            EventBus::global().publish_log(&switch_msg, false);
                            warn!("{}", switch_msg);
                            break;
                        } else {
                            let fatal = format!("[AI-GEMINI] Erro na API Gemini (HTTP 404 - Modelo '{current_model}' não encontrado): {resp_body}");
                            EventBus::global().publish_log(&fatal, false);
                            error!("{fatal}");
                            return Err(GeminiApiError::new(status, fatal, Some(resp_body)).into());
                        }
                    } else {
                        let fatal = format!("[AI-GEMINI] Erro na API Gemini (HTTP {status}): {resp_body}");
                        EventBus::global().publish_log(&fatal, false);
                        error!("{fatal}");
                        return Err(GeminiApiError::new(status, fatal, Some(resp_body)).into());
                    }
                }
            }
        }

        let final_status = if last_error_msg.contains("429") { 429 } else { 503 };
        let msg = format!(
            "[AI-GEMINI] Todos os modelos testados ({}) apresentaram sobrecarga persistente após {} ciclos. Último erro: {last_error_msg}",
            models.join(", "),
            self.max_cycles
        );
        EventBus::global().publish_log(&msg, false);
        Err(GeminiApiError::new(final_status, msg, Some(last_error_msg)).into())
    }
}

impl AiClient for GeminiClient {
    async fn analyze_transcript(
        &self,
        video_source_id: Option<i64>,
        video_title: &str,
        video_author: &str,
        segments: &[TranscriptSegment],
        min_virality_threshold: i32,
    ) -> Result<AnalysisResult, AiError> {
        if self.api_key.trim().is_empty() {
            return Err(AiError::ApiKeyMissing(
                "Chave da API do Gemini não configurada. Configure na tela de configurações ou via GEMINI_API_KEY.".to_string(),
            ));
        }

        if segments.is_empty() {
            let warn_msg = "[AI-GEMINI] Nenhum segmento de transcrição para analisar.";
            EventBus::global().publish_log(warn_msg, false);
            warn!("{warn_msg}");
            return Ok(AnalysisResult::default());
        }

        let estimated_words: usize = segments.iter().map(|s| s.text.split_whitespace().count()).sum();
        let start_msg = format!(
            "[AI-GEMINI] Iniciando análise de transcrição | Modelo: {} | Segmentos: {} | Palavras estimadas: {} | Limiar de nota: {}",
            self.model_name,
            segments.len(),
            estimated_words,
            min_virality_threshold
        );
        EventBus::global().publish_log(&start_msg, false);
        info!("{}", start_msg);

        let formatted_transcript = segments
            .iter()
            .map(|s| format!("{} {}\n", s.formatted_time_range(), s.text))
            .collect::<String>();

        let cache_key = TranscriptAnalysisCache::compute_key(
            video_title,
            video_author,
            &formatted_transcript,
            min_virality_threshold,
            &self.model_name,
        );

        if let Some(cached) = TranscriptAnalysisCache::global().get(&cache_key, video_source_id) {
            let hit_msg = format!("[AI-GEMINI] [Cache Hit] Reutilizando análise de transcrição em cache ({cache_key})");
            EventBus::global().publish_log(&hit_msg, false);
            info!("{}", hit_msg);
            return Ok(cached);
        }

        let user_prompt = PromptFactory::build_user_prompt(video_title, video_author, &formatted_transcript);
        let req_msg = format!("[AI-GEMINI] Enviando requisição para API Gemini ({})", self.model_name);
        EventBus::global().publish_log(&req_msg, false);
        info!("{}", req_msg);

        let request_body = self.build_request_body(&user_prompt);
        let response_body = self.execute_with_retry(&request_body, "análise de transcrição").await?;

        let raw_candidates = self.parse_gemini_response(video_source_id, &response_body)?;
        let snapped_candidates = self.snap_to_segment_boundaries(&raw_candidates, segments);
        let final_result = self.partition_and_filter(snapped_candidates, min_virality_threshold);

        TranscriptAnalysisCache::global().put(cache_key, final_result.clone());
        Ok(final_result)
    }

    async fn analyze_video_url(
        &self,
        video_source_id: Option<i64>,
        video_url: &str,
        video_title: &str,
        video_author: &str,
        optional_segments: Option<&[TranscriptSegment]>,
        min_virality_threshold: i32,
    ) -> Result<AnalysisResult, AiError> {
        if self.api_key.trim().is_empty() {
            return Err(AiError::ApiKeyMissing(
                "Chave da API do Gemini não configurada. Configure na tela de configurações ou via GEMINI_API_KEY.".to_string(),
            ));
        }

        if video_url.trim().is_empty() {
            return Err(AiError::Other("URL do vídeo não pode ser nula ou vazia para análise multimodal.".to_string()));
        }

        let seg_count = optional_segments.map(|segs| segs.len()).unwrap_or(0);
        let estimated_words: usize = optional_segments
            .map(|segs| segs.iter().map(|s| s.text.split_whitespace().count()).sum())
            .unwrap_or(0);
        let start_msg = format!(
            "[AI-GEMINI] Iniciando análise multimodal | Modelo: {} | URL: {} | Segmentos: {} | Palavras estimadas: {} | Limiar de nota: {}",
            self.model_name,
            video_url,
            seg_count,
            estimated_words,
            min_virality_threshold
        );
        EventBus::global().publish_log(&start_msg, false);
        info!("{}", start_msg);

        let formatted_transcript = optional_segments.map(|segs| {
            segs.iter()
                .map(|s| format!("{} {}\n", s.formatted_time_range(), s.text))
                .collect::<String>()
        }).unwrap_or_default();

        let cache_payload = format!("VIDEO_URL:{video_url}:{formatted_transcript}");
        let cache_key = TranscriptAnalysisCache::compute_key(
            video_title,
            video_author,
            &cache_payload,
            min_virality_threshold,
            &self.model_name,
        );

        if let Some(cached) = TranscriptAnalysisCache::global().get(&cache_key, video_source_id) {
            let hit_msg = format!("[AI-GEMINI] [Cache Hit] Reutilizando análise multimodal em cache ({cache_key})");
            EventBus::global().publish_log(&hit_msg, false);
            info!("{}", hit_msg);
            return Ok(cached);
        }

        let user_prompt = PromptFactory::build_multimodal_user_prompt(
            video_title,
            video_author,
            if formatted_transcript.trim().is_empty() { None } else { Some(&formatted_transcript) },
        );

        let req_msg = format!("[AI-GEMINI] Enviando vídeo multimodal para avaliação do Gemini ({}) via URL: {video_url}", self.model_name);
        EventBus::global().publish_log(&req_msg, false);
        info!("{}", req_msg);

        let request_body = self.build_multimodal_request_body(&user_prompt, video_url);
        let response_body = self.execute_with_retry(&request_body, "análise multimodal de vídeo").await?;

        let raw_candidates = self.parse_gemini_response(video_source_id, &response_body)?;
        let snapped_candidates = if let Some(segs) = optional_segments {
            self.snap_to_segment_boundaries(&raw_candidates, segs)
        } else {
            self.snap_to_segment_boundaries(&raw_candidates, &[])
        };

        let final_result = self.partition_and_filter(snapped_candidates, min_virality_threshold);
        TranscriptAnalysisCache::global().put(cache_key, final_result.clone());
        Ok(final_result)
    }
}

fn get_field<'a>(node: &'a Value, names: &[&str]) -> Option<&'a Value> {
    for name in names {
        if let Some(val) = node.get(*name)
            && !val.is_null() {
                return Some(val);
            }
    }
    None
}

fn get_str_field<'a>(node: &'a Value, names: &[&str]) -> Option<&'a str> {
    get_field(node, names).and_then(|v| v.as_str())
}

fn get_i64_field(node: &Value, names: &[&str]) -> Option<i64> {
    get_field(node, names).and_then(|v| {
        if let Some(n) = v.as_i64() {
            Some(n)
        } else if let Some(s) = v.as_str() {
            s.trim().parse::<i64>().ok()
        } else {
            None
        }
    })
}

fn get_i32_field(node: &Value, names: &[&str]) -> Option<i32> {
    get_i64_field(node, names).map(|n| n.clamp(0, 100) as i32)
}

fn get_timestamp_field(node: &Value, names: &[&str], default_val: f64) -> f64 {
    match get_field(node, names) {
        Some(v) => parse_node_timestamp(v, default_val),
        None => default_val,
    }
}

fn parse_node_timestamp(node: &Value, default_val: f64) -> f64 {
    if let Some(n) = node.as_f64() {
        return n;
    }
    if let Some(s) = node.as_str() {
        return GeminiClient::parse_flexible_timestamp(s, default_val);
    }
    default_val
}
