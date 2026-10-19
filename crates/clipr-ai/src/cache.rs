use clipr_domain::ClipCandidate;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::{LazyLock, RwLock};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AnalysisResult {
    pub approved_clips: Vec<ClipCandidate>,
    pub discarded_clips: Vec<ClipCandidate>,
}

impl AnalysisResult {
    pub fn new(approved_clips: Vec<ClipCandidate>, discarded_clips: Vec<ClipCandidate>) -> Self {
        Self {
            approved_clips,
            discarded_clips,
        }
    }
}

pub static GLOBAL_CACHE: LazyLock<TranscriptAnalysisCache> = LazyLock::new(TranscriptAnalysisCache::new);

#[derive(Debug, Default)]
pub struct TranscriptAnalysisCache {
    cache: RwLock<HashMap<String, AnalysisResult>>,
}

impl TranscriptAnalysisCache {
    pub fn new() -> Self {
        Self {
            cache: RwLock::new(HashMap::new()),
        }
    }

    pub fn global() -> &'static Self {
        &GLOBAL_CACHE
    }

    pub fn compute_key(
        video_title: &str,
        video_author: &str,
        formatted_transcript: &str,
        min_virality_threshold: i32,
        model_name: &str,
    ) -> String {
        let raw = format!(
            "{}|{}|{}|{}|{}",
            video_title.trim(),
            video_author.trim(),
            min_virality_threshold,
            model_name.trim(),
            formatted_transcript.trim()
        );
        let mut hasher = Sha256::new();
        hasher.update(raw.as_bytes());
        let result = hasher.finalize();
        hex::encode(result)
    }

    pub fn get(&self, cache_key: &str, video_source_id: Option<i64>) -> Option<AnalysisResult> {
        if cache_key.trim().is_empty() {
            return None;
        }

        let guard = self.cache.read().ok()?;
        let cached = guard.get(cache_key)?;
        Some(Self::adapt_video_source_id(cached, video_source_id))
    }

    pub fn put(&self, cache_key: impl Into<String>, result: AnalysisResult) {
        let key = cache_key.into();
        if key.trim().is_empty() {
            return;
        }
        if let Ok(mut guard) = self.cache.write() {
            guard.insert(key, result);
        }
    }

    pub fn contains_key(&self, cache_key: &str) -> bool {
        if cache_key.trim().is_empty() {
            return false;
        }
        self.cache.read().is_ok_and(|guard| guard.contains_key(cache_key))
    }

    pub fn clear(&self) {
        if let Ok(mut guard) = self.cache.write() {
            guard.clear();
        }
    }

    pub fn size(&self) -> usize {
        self.cache.read().map_or(0, |guard| guard.len())
    }

    fn adapt_video_source_id(result: &AnalysisResult, video_source_id: Option<i64>) -> AnalysisResult {
        if video_source_id.is_none() {
            return result.clone();
        }

        let target_id = video_source_id;
        let adapt_list = |list: &[ClipCandidate]| -> Vec<ClipCandidate> {
            list.iter()
                .map(|c| {
                    if c.video_source_id == target_id {
                        c.clone()
                    } else {
                        let mut copy = c.clone();
                        copy.video_source_id = target_id;
                        copy
                    }
                })
                .collect()
        };

        AnalysisResult {
            approved_clips: adapt_list(&result.approved_clips),
            discarded_clips: adapt_list(&result.discarded_clips),
        }
    }
}
