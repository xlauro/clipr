use crate::clip::ClipCandidate;
use crate::video_source::VideoSource;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub const SUBDIR_CORTES: &str = "cortes";
pub const SUBDIR_THUMBNAILS: &str = "thumbnails";
pub const SUBDIR_DADOS: &str = "dados";
pub const MAX_TITLE_LENGTH: usize = 60;

/// Representa a estrutura organizada de pastas de exportação para um vídeo no Clippr.
///
/// Padrão de layout:
/// [rootOutputDir]/
///   └── [Titulo Sanitizado] [[videoId]]/
///       ├── cortes/         (arquivos .mp4 renderizados)
///       ├── thumbnails/     (capas e frames limpos gerados)
///       └── dados/          (metadados .json dos cortes e transcrições .vtt/.txt)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VideoExportLayout {
    pub root_output_dir: PathBuf,
    pub raw_title: String,
    pub sanitized_title: String,
    pub video_id: String,
    pub video_dir: PathBuf,
    pub cortes_dir: PathBuf,
    pub thumbnails_dir: PathBuf,
    pub dados_dir: PathBuf,
}

impl VideoExportLayout {
    pub fn new(root_output_dir: impl AsRef<Path>, video_id: &str, video_title: &str) -> Self {
        Self::resolve(root_output_dir, video_title, video_id)
    }

    pub fn resolve(root_output_dir: impl AsRef<Path>, video_title: &str, video_id: &str) -> Self {
        let root = root_output_dir.as_ref().to_path_buf();
        let sanitized = Self::sanitize_title(video_title);
        let safe_video_id = Self::sanitize_video_id(video_id);

        let folder_name = if sanitized.is_empty() {
            format!("video_{}", safe_video_id)
        } else {
            format!("{} [{}]", sanitized, safe_video_id)
        };

        let video_dir = root.join(&folder_name);
        let cortes_dir = video_dir.join(SUBDIR_CORTES);
        let thumbnails_dir = video_dir.join(SUBDIR_THUMBNAILS);
        let dados_dir = video_dir.join(SUBDIR_DADOS);

        Self {
            root_output_dir: root,
            raw_title: video_title.to_string(),
            sanitized_title: sanitized,
            video_id: safe_video_id,
            video_dir,
            cortes_dir,
            thumbnails_dir,
            dados_dir,
        }
    }

    pub fn resolve_source(root_output_dir: impl AsRef<Path>, source: &VideoSource) -> Self {
        Self::resolve(root_output_dir, &source.title, &source.video_id)
    }

    pub fn sanitize_title(title: &str) -> String {
        if title.trim().is_empty() {
            return String::new();
        }

        // Substitui caracteres proibidos pelo SO
        let re_forbidden = Regex::new(r#"[\\/:*?"<>|\x00-\x1F]"#).unwrap();
        let sanitized = re_forbidden.replace_all(title, "-");

        // Normaliza múltiplos espaços
        let re_spaces = Regex::new(r#"\s+"#).unwrap();
        let mut sanitized = re_spaces.replace_all(&sanitized, " ").trim().to_string();

        if sanitized.is_empty() {
            return String::new();
        }

        // Limita a no máximo MAX_TITLE_LENGTH caracteres sem quebrar palavras ao meio
        if sanitized.chars().count() > MAX_TITLE_LENGTH {
            let candidate: String = sanitized.chars().take(MAX_TITLE_LENGTH).collect();
            if let Some(last_space) = candidate.rfind(' ') {
                sanitized = candidate[..last_space].to_string();
            } else {
                sanitized = candidate;
            }
        }

        // Remove resíduos de pontuação, traços e espaços nas extremidades
        let re_trim = Regex::new(r#"^[\s\-_.,;:]+|[\s\-_.,;:]+$"#).unwrap();
        re_trim.replace_all(&sanitized, "").trim().to_string()
    }

    pub fn sanitize_video_id(video_id: &str) -> String {
        if video_id.trim().is_empty() {
            return "unknown".to_string();
        }
        let mut extracted = VideoSource::extract_video_id(video_id);
        if extracted.is_empty() {
            extracted = video_id.trim().to_string();
        }
        let re = Regex::new(r#"[^a-zA-Z0-9_-]"#).unwrap();
        let safe = re.replace_all(&extracted, "_").trim().to_string();
        if safe.is_empty() {
            "unknown".to_string()
        } else {
            safe
        }
    }

    pub fn ensure_directories_exist(&self) -> io::Result<()> {
        fs::create_dir_all(&self.video_dir)?;
        fs::create_dir_all(&self.cortes_dir)?;
        fs::create_dir_all(&self.thumbnails_dir)?;
        fs::create_dir_all(&self.dados_dir)?;
        Ok(())
    }

    pub fn resolve_clip(&self, file_name: &str) -> PathBuf {
        self.cortes_dir.join(file_name)
    }

    pub fn resolve_clip_candidate(&self, clip: &ClipCandidate) -> PathBuf {
        self.resolve_clip(&self.format_clip_file_name(clip))
    }

    pub fn format_clip_file_name(&self, clip: &ClipCandidate) -> String {
        let re = Regex::new(r#"[^a-zA-Z0-9_-]"#).unwrap();
        let mut safe_title = re.replace_all(&clip.title, "_").to_string();
        if safe_title.chars().count() > 50 {
            safe_title = safe_title.chars().take(50).collect();
        }
        format!(
            "clip_{}_{:03}_{}.mp4",
            self.video_id, clip.virality_score, safe_title
        )
    }

    pub fn resolve_thumbnail(&self, file_name: &str) -> PathBuf {
        self.thumbnails_dir.join(file_name)
    }

    pub fn resolve_data(&self, file_name: &str) -> PathBuf {
        self.dados_dir.join(file_name)
    }

    pub fn resolve_clip_metadata(&self, clip: &ClipCandidate) -> PathBuf {
        self.resolve_data(&self.format_clip_metadata_file_name(clip))
    }

    pub fn format_clip_metadata_file_name(&self, clip: &ClipCandidate) -> String {
        self.format_clip_file_name(clip).replace(".mp4", ".json")
    }

    pub fn resolve_transcription_vtt(&self) -> PathBuf {
        self.dados_dir.join("transcricao.vtt")
    }

    pub fn resolve_transcription_txt(&self) -> PathBuf {
        self.dados_dir.join("transcricao.txt")
    }
}
