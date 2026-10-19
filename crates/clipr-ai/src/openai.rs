use crate::cache::{AnalysisResult, TranscriptAnalysisCache};
use crate::client::AiClient;
use crate::error::{AiError, OpenAiApiError};
use crate::gemini::GeminiClient;
use crate::prompt::PromptFactory;
use clipr_domain::{
    ClipCandidate, EmotionTag, EventBus, FramingMode, OrientationMode, ProcessingStatus,
    ScoreBreakdown, SubtitlePreset, TranscriptSegment,
};
use reqwest::Client as HttpClient;
use serde_json::{json, Value};
use std::time::Duration;
use tracing::{debug, error, info, warn};

pub const DEFAULT_MODEL: &str = "gpt-4o-mini";
pub const DEFAULT_BASE_URL: &str = "https://api.openai.com/v1/chat/completions";

#[derive(Clone)]
pub struct OpenAiClient {
    api_key: String,
    model_name: String,
    http_client: HttpClient,
    max_attempts: usize,
    initial_backoff_ms: u64,
    max_backoff_ms: u64,
    base_url: String,
}

impl OpenAiClient {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self::with_model(api_key, DEFAULT_MODEL)
    }

    pub fn with_model(api_key: impl Into<String>, model_name: impl Into<String>) -> Self {
        Self::with_config(
            api_key,
            model_name,
            5,
            1000,
            DEFAULT_BASE_URL,
        )
    }

    pub fn with_config(
        api_key: impl Into<String>,
        model_name: impl Into<String>,
        max_attempts: usize,
        initial_backoff_ms: u64,
        base_url: &str,
    ) -> Self {
        let http_client = HttpClient::builder()
            .timeout(Duration::from_secs(120))
            .connect_timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_default();

        let model = model_name.into();
        Self {
            api_key: api_key.into().trim().to_string(),
            model_name: if model.trim().is_empty() {
                DEFAULT_MODEL.to_string()
            } else {
                model.trim().to_string()
            },
            http_client,
            max_attempts: max_attempts.max(1),
            initial_backoff_ms: initial_backoff_ms.max(1),
            max_backoff_ms: if initial_backoff_ms <= 100 { 500 } else { 60000 },
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

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn build_request_body(&self, user_prompt: &str) -> Value {
        json!({
            "model": self.model_name,
            "messages": [
                {
                    "role": "system",
                    "content": PromptFactory::SYSTEM_INSTRUCTION
                },
                {
                    "role": "user",
                    "content": user_prompt
                }
            ],
            "response_format": {
                "type": "json_object"
            },
            "temperature": 0.2
        })
    }

    pub fn parse_openai_response(
        &self,
        video_source_id: Option<i64>,
        json_response: &str,
    ) -> Result<Vec<ClipCandidate>, AiError> {
        if json_response.trim().is_empty() {
            return Err(OpenAiApiError::new(400, "Resposta da API OpenAI vazia ou nula", None).into());
        }

        let root: Value = serde_json::from_str(json_response).map_err(|e| {
            error!("[OpenAI] Falha ao fazer parse do payload externo da API: {e}");
            AiError::Parsing(format!("Falha no parse do payload externo: {e}"))
        })?;

        let choices = root.get("choices").and_then(|c| c.as_array());
        let first_choice = match choices.and_then(|arr| arr.first()) {
            Some(fc) => fc,
            None => {
                return Err(AiError::Parsing("Resposta da OpenAI não contém escolhas válidas".to_string()));
            }
        };

        let finish_reason = first_choice
            .get("finish_reason")
            .or_else(|| first_choice.get("finishReason"))
            .and_then(|f| f.as_str())
            .unwrap_or("");

        if finish_reason.eq_ignore_ascii_case("content_filter") {
            warn!("Resposta da OpenAI bloqueada por filtro de conteúdo (content_filter).");
            return Ok(Vec::new());
        }

        let text_content = first_choice
            .pointer("/message/content")
            .and_then(|t| t.as_str())
            .unwrap_or("");

        if text_content.trim().is_empty() {
            return Ok(Vec::new());
        }

        let cleaned_text = GeminiClient::clean_json_content(text_content);
        let parsed_json: Value = serde_json::from_str(&cleaned_text).map_err(|e| {
            error!("[OpenAI] Falha no parse do JSON interno dos candidatos: {e}");
            AiError::Parsing(format!("Falha ao realizar parse do JSON dos candidatos: {e}"))
        })?;

        let mut clips_array = if parsed_json.is_array() {
            parsed_json.as_array()
        } else if let Some(clips) = parsed_json.get("clips").and_then(|c| c.as_array()) {
            Some(clips)
        } else {
            parsed_json.get("candidates").and_then(|c| c.as_array())
        };

        if clips_array.is_none() && parsed_json.is_object()
            && let Some(obj) = parsed_json.as_object() {
                for (_key, val) in obj {
                    if let Some(arr) = val.as_array() {
                        clips_array = Some(arr);
                        break;
                    }
                }
            }

        let clips_array = match clips_array {
            Some(arr) => arr,
            None => {
                warn!("Estrutura JSON não esperada da OpenAI: {text_content}");
                return Ok(Vec::new());
            }
        };

        let mut list = Vec::new();
        for item in clips_array {
            let title = item.get("title").and_then(|t| t.as_str()).unwrap_or("Corte Viral").to_string();
            let hook = item.get("hook").and_then(|h| h.as_str()).unwrap_or("").to_string();
            let start_seconds = item.get("startSeconds")
                .or_else(|| item.get("start_seconds"))
                .and_then(|s| s.as_f64())
                .unwrap_or(0.0);
            let end_seconds = item.get("endSeconds")
                .or_else(|| item.get("end_seconds"))
                .and_then(|e| e.as_f64())
                .unwrap_or(0.0);
            let score = item.get("viralityScore")
                .or_else(|| item.get("score"))
                .and_then(|s| s.as_i64())
                .unwrap_or(50)
                .clamp(0, 100) as i32;

            let reasoning = item.get("reasoning").and_then(|r| r.as_str()).unwrap_or("").to_string();
            let caption = item.get("suggestedCaption").and_then(|c| c.as_str()).unwrap_or(&title).to_string();

            let mut hashtags = Vec::new();
            if let Some(tags_val) = item.get("hashtags").and_then(|h| h.as_array()) {
                for t in tags_val {
                    if let Some(tag_str) = t.as_str() {
                        hashtags.push(tag_str.to_string());
                    }
                }
            }

            let thumbnail_timestamp = if let Some(thumb_val) = item.get("thumbnailTimestamp") {
                let ts = thumb_val.as_f64().unwrap_or(start_seconds);
                if ts >= start_seconds && ts <= end_seconds {
                    Some(ts)
                } else {
                    Some(start_seconds)
                }
            } else {
                Some(start_seconds)
            };

            let thumbnail_headline = item.get("thumbnailHeadline")
                .and_then(|h| h.as_str())
                .filter(|s| !s.trim().is_empty())
                .unwrap_or(&title)
                .to_string();

            let framing_mode = if let Some(fm_str) = item.get("framingMode").and_then(|f| f.as_str()) {
                FramingMode::from_str_loose(fm_str, FramingMode::AutoCrop)
            } else {
                FramingMode::AutoCrop
            };

            let subtitle_preset = if let Some(sp_str) = item.get("subtitlePreset").and_then(|s| s.as_str()) {
                SubtitlePreset::from_str_loose(sp_str, SubtitlePreset::HormoziYellow)
            } else {
                SubtitlePreset::HormoziYellow
            };

            let orientation = if framing_mode == FramingMode::Horizontal169 {
                OrientationMode::Horizontal169
            } else {
                OrientationMode::Vertical916
            };

            let emotion_tag = if let Some(e_str) = item.get("emotionTag").and_then(|e| e.as_str()) {
                EmotionTag::from_str_loose(e_str, EmotionTag::AltaEnergia)
            } else {
                EmotionTag::AltaEnergia
            };

            let (score_breakdown, final_virality_score) = if let Some(scores_val) = item.get("scores").or_else(|| item.get("scoreBreakdown")) {
                if scores_val.is_object() {
                    let hook_score = scores_val.get("hook").and_then(|v| v.as_i64()).unwrap_or(score as i64).clamp(0, 100) as i32;
                    let retention_score = scores_val.get("retention").and_then(|v| v.as_i64()).unwrap_or(score as i64).clamp(0, 100) as i32;
                    let emotional_score = scores_val.get("emotional").or_else(|| scores_val.get("impact")).and_then(|v| v.as_i64()).unwrap_or(score as i64).clamp(0, 100) as i32;
                    let storytelling_score = scores_val.get("storytelling").or_else(|| scores_val.get("narrative")).and_then(|v| v.as_i64()).unwrap_or(score as i64).clamp(0, 100) as i32;
                    let standalone_score = scores_val.get("standalone").and_then(|v| v.as_i64()).unwrap_or(score as i64).clamp(0, 100) as i32;
                    let visual_score = scores_val.get("visual").and_then(|v| v.as_i64()).unwrap_or(score as i64).clamp(0, 100) as i32;

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
                    (ScoreBreakdown::from_uniform(score), score)
                }
            } else {
                (ScoreBreakdown::from_uniform(score), score)
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
}

impl AiClient for OpenAiClient {
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
                "Chave da API da OpenAI não configurada. Configure na tela de configurações ou via OPENAI_API_KEY.".to_string(),
            ));
        }

        if segments.is_empty() {
            let warn_msg = "[AI-OPENAI] Nenhum segmento de transcrição para analisar.";
            EventBus::global().publish_log(warn_msg, false);
            warn!("{warn_msg}");
            return Ok(AnalysisResult::default());
        }

        let estimated_words: usize = segments.iter().map(|s| s.text.split_whitespace().count()).sum();
        let start_msg = format!(
            "[AI-OPENAI] Iniciando análise de transcrição | Modelo: {} | Segmentos: {} | Palavras estimadas: {} | Limiar de nota: {}",
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
            let hit_msg = format!("[AI-OPENAI] [Cache Hit] Reutilizando análise de transcrição em cache ({cache_key})");
            EventBus::global().publish_log(&hit_msg, false);
            info!("{}", hit_msg);
            return Ok(cached);
        }

        let user_prompt = PromptFactory::build_user_prompt(video_title, video_author, &formatted_transcript);
        let req_msg = format!("[AI-OPENAI] Enviando requisição para API OpenAI ({})", self.model_name);
        EventBus::global().publish_log(&req_msg, false);
        info!("{}", req_msg);

        let request_body = self.build_request_body(&user_prompt);

        let prompt_summary = serde_json::to_string(&request_body).unwrap_or_default();
        let debug_prompt_msg = format!(
            "[AI-OPENAI] [DEBUG] Resumo do payload enviado ({} bytes): {}",
            prompt_summary.len(),
            if prompt_summary.len() > 1000 {
                format!("{}...", &prompt_summary[..1000])
            } else {
                prompt_summary.clone()
            }
        );
        EventBus::global().publish_log(&debug_prompt_msg, true);
        debug!("{}", debug_prompt_msg);

        let mut last_error_msg = String::new();

        for attempt in 1..=self.max_attempts {
            let req = self.http_client
                .post(&self.base_url)
                .header("Authorization", format!("Bearer {}", self.api_key))
                .header("Content-Type", "application/json")
                .json(&request_body);

            let start_req = std::time::Instant::now();
            let resp = match req.send().await {
                Ok(r) => r,
                Err(e) => {
                    last_error_msg = e.to_string();
                    if attempt == self.max_attempts {
                        let err_msg = format!("[AI-OPENAI] Falha de conexão na tentativa {attempt} no modelo {}: {e}", self.model_name);
                        EventBus::global().publish_log(&err_msg, false);
                        error!("{err_msg}");
                        return Err(OpenAiApiError::new(500, err_msg, None).into());
                    }
                    let base = self.max_backoff_ms.min((self.initial_backoff_ms as f64 * 1.5_f64.powi(attempt as i32 - 1)) as u64);
                    let backoff = Duration::from_millis(base);
                    let retry_msg = format!("[AI-OPENAI] Falha de conexão ({e}). Tentativa {} de {} em {:.1}s...", attempt + 1, self.max_attempts, backoff.as_secs_f64());
                    EventBus::global().publish_log(&retry_msg, false);
                    warn!("{}", retry_msg);
                    tokio::time::sleep(backoff).await;
                    continue;
                }
            };

            let elapsed_ms = start_req.elapsed().as_millis();
            let status = resp.status().as_u16();
            if status == 200 {
                let ok_msg = format!(
                    "[AI-OPENAI] Resposta recebida da API com sucesso (HTTP 200) em {}ms na tentativa {}!",
                    elapsed_ms, attempt
                );
                EventBus::global().publish_log(&ok_msg, false);
                info!("{}", ok_msg);

                let response_body = resp.text().await.map_err(AiError::Network)?;
                let debug_resp_msg = format!("[AI-OPENAI] [DEBUG] Resposta bruta da API:\n{response_body}");
                EventBus::global().publish_log(&debug_resp_msg, true);
                debug!("{}", debug_resp_msg);

                let raw_candidates = self.parse_openai_response(video_source_id, &response_body)?;
                let snapped = GeminiClient::with_model("", "").snap_to_segment_boundaries(&raw_candidates, segments);

                let mut approved = Vec::new();
                let mut discarded = Vec::new();

                for (i, c) in snapped.into_iter().enumerate() {
                    let is_approved = c.virality_score >= min_virality_threshold;
                    let status_str = if is_approved { "APROVADO" } else { "DESCARTADO" };

                    let clip_log = format!(
                        "[AI-OPENAI] Corte #{}: '{}' | Gancho: '{}' | Nota: {}/100 [Retenção: {}, Emoção: {}, Coesão: {}, Clareza: {}, Ineditismo: {}] | Status: {}",
                        i + 1,
                        c.title,
                        c.hook,
                        c.virality_score,
                        c.scores.retention,
                        c.scores.emotional,
                        c.scores.storytelling,
                        c.scores.standalone,
                        c.scores.visual,
                        status_str,
                    );
                    EventBus::global().publish_log(&clip_log, false);
                    info!("{}", clip_log);

                    if is_approved {
                        approved.push(c);
                    } else {
                        let discard_msg = format!(
                            "[AI-OPENAI] Corte #{} descartado: Abaixo da nota de corte {}",
                            i + 1,
                            min_virality_threshold
                        );
                        EventBus::global().publish_log(&discard_msg, false);
                        info!("{}", discard_msg);

                        let mut discarded_c = c;
                        discarded_c.status = ProcessingStatus::DiscardedLowScore;
                        discarded.push(discarded_c);
                    }
                }

                let final_result = AnalysisResult::new(approved, discarded);
                TranscriptAnalysisCache::global().put(cache_key, final_result.clone());
                return Ok(final_result);
            }

            let resp_body = resp.text().await.unwrap_or_default();

            if status == 429 {
                let retry_after = GeminiClient::parse_retry_after(None);
                let backoff = retry_after.unwrap_or_else(|| {
                    let base = self.max_backoff_ms.min((self.initial_backoff_ms as f64 * 2.0_f64.powi(attempt as i32 - 1)) as u64);
                    Duration::from_millis(base)
                });
                last_error_msg = format!("Limite de requisições ou cota excedida (429): {resp_body}");

                if attempt < self.max_attempts {
                    let retry_msg = format!("[AI-OPENAI] Rate limit (HTTP 429) no modelo {} na tentativa {} de {}. Aguardando retry em {:.1}s...", self.model_name, attempt + 1, self.max_attempts, backoff.as_secs_f64());
                    EventBus::global().publish_log(&retry_msg, false);
                    warn!("{}", retry_msg);
                    tokio::time::sleep(backoff).await;
                } else {
                    let msg = format!("[AI-OPENAI] [{}] {} tentativas esgotadas por 429.", self.model_name, self.max_attempts);
                    EventBus::global().publish_log(&msg, false);
                    error!("{msg}");
                    return Err(OpenAiApiError::new(429, msg, Some(resp_body)).into());
                }
            } else if (500..600).contains(&status) {
                last_error_msg = format!("Erro no servidor OpenAI ({status}): {resp_body}");
                if attempt < self.max_attempts {
                    let base = self.max_backoff_ms.min((self.initial_backoff_ms as f64 * 1.5_f64.powi(attempt as i32 - 1)) as u64);
                    let backoff = Duration::from_millis(base);
                    let retry_msg = format!("[AI-OPENAI] Erro no servidor (HTTP {status}) no modelo {} na tentativa {} de {}. Aguardando retry em {:.1}s...", self.model_name, attempt + 1, self.max_attempts, backoff.as_secs_f64());
                    EventBus::global().publish_log(&retry_msg, false);
                    warn!("{}", retry_msg);
                    tokio::time::sleep(backoff).await;
                } else {
                    let msg = format!("[AI-OPENAI] [{}] {} tentativas esgotadas por {status}.", self.model_name, self.max_attempts);
                    EventBus::global().publish_log(&msg, false);
                    error!("{msg}");
                    return Err(OpenAiApiError::new(status, msg, Some(resp_body)).into());
                }
            } else if status == 401 {
                let err_msg = format!("[AI-OPENAI] Erro na API OpenAI (HTTP 401): Chave inválida ou não autorizada. Detalhes: {resp_body}");
                EventBus::global().publish_log(&err_msg, false);
                error!("{err_msg}");
                return Err(OpenAiApiError::new(401, err_msg, Some(resp_body)).into());
            } else if status == 400 {
                let err_msg = format!("[AI-OPENAI] Erro na API OpenAI (HTTP 400): Requisição inválida. Detalhes: {resp_body}");
                EventBus::global().publish_log(&err_msg, false);
                error!("{err_msg}");
                return Err(OpenAiApiError::new(400, err_msg, Some(resp_body)).into());
            } else if status == 403 {
                let err_msg = format!("[AI-OPENAI] Erro na API OpenAI (HTTP 403): Acesso proibido. Detalhes: {resp_body}");
                EventBus::global().publish_log(&err_msg, false);
                error!("{err_msg}");
                return Err(OpenAiApiError::new(403, err_msg, Some(resp_body)).into());
            } else {
                let err_msg = format!("[AI-OPENAI] Erro na API OpenAI (HTTP {status}): {resp_body}");
                EventBus::global().publish_log(&err_msg, false);
                error!("{err_msg}");
                return Err(OpenAiApiError::new(status, err_msg, Some(resp_body)).into());
            }
        }

        let msg = format!("[AI-OPENAI] Falha persistente na API OpenAI após {} tentativas. Último erro: {last_error_msg}", self.max_attempts);
        EventBus::global().publish_log(&msg, false);
        Err(OpenAiApiError::new(500, msg, Some(last_error_msg)).into())
    }
}
