use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Status de processamento de um vídeo ou clipe no pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProcessingStatus {
    #[default]
    Pending,
    FetchingTranscript,
    AnalyzingAi,
    PendingApproval,
    DownloadingVideo,
    CuttingFfmpeg,
    Completed,
    DiscardedLowScore,
    Failed,
}

impl ProcessingStatus {
    pub fn description(&self) -> &'static str {
        match self {
            Self::Pending => "Pendente",
            Self::FetchingTranscript => "Baixando transcrição",
            Self::AnalyzingAi => "Analisando com IA",
            Self::PendingApproval => "Aguardando aprovação",
            Self::DownloadingVideo => "Baixando vídeo",
            Self::CuttingFfmpeg => "Renderizando cortes",
            Self::Completed => "Concluído",
            Self::DiscardedLowScore => "Descartado (Nota < 90)",
            Self::Failed => "Falhou",
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed)
    }

    pub fn can_transition_to(&self, next: Self) -> bool {
        if self.is_terminal() {
            return false;
        }
        if next == Self::Failed {
            return true;
        }
        match self {
            Self::Pending => matches!(next, Self::FetchingTranscript | Self::DownloadingVideo),
            Self::FetchingTranscript => matches!(next, Self::AnalyzingAi),
            Self::AnalyzingAi => matches!(
                next,
                Self::PendingApproval | Self::DiscardedLowScore | Self::DownloadingVideo
            ),
            Self::PendingApproval => {
                matches!(next, Self::DownloadingVideo | Self::DiscardedLowScore)
            }
            Self::DownloadingVideo => matches!(next, Self::CuttingFfmpeg),
            Self::CuttingFfmpeg => matches!(next, Self::Completed),
            Self::DiscardedLowScore => matches!(next, Self::Pending | Self::AnalyzingAi),
            _ => false,
        }
    }

    pub fn as_db_str(&self) -> &'static str {
        match self {
            Self::Pending => "PENDING",
            Self::FetchingTranscript => "FETCHING_TRANSCRIPT",
            Self::AnalyzingAi => "ANALYZING_AI",
            Self::PendingApproval => "PENDING_APPROVAL",
            Self::DownloadingVideo => "DOWNLOADING_VIDEO",
            Self::CuttingFfmpeg => "CUTTING_FFMPEG",
            Self::Completed => "COMPLETED",
            Self::DiscardedLowScore => "DISCARDED_LOW_SCORE",
            Self::Failed => "FAILED",
        }
    }
}

impl fmt::Display for ProcessingStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

impl FromStr for ProcessingStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let clean = s.trim().to_uppercase();
        match clean.as_str() {
            "PENDING" | "PENDENTE" => Ok(Self::Pending),
            "FETCHING_TRANSCRIPT" | "FETCHINGTRANSCRIPT" | "BAIXANDO TRANSCRIÇÃO" | "BAIXANDO TRANSCRICAO" => {
                Ok(Self::FetchingTranscript)
            }
            "ANALYZING_AI" | "ANALYZINGAI" | "ANALISANDO COM IA" => Ok(Self::AnalyzingAi),
            "PENDING_APPROVAL" | "PENDINGAPPROVAL" | "AGUARDANDO APROVAÇÃO" | "AGUARDANDO APROVACAO" => {
                Ok(Self::PendingApproval)
            }
            "DOWNLOADING_VIDEO" | "DOWNLOADINGVIDEO" | "BAIXANDO VÍDEO" | "BAIXANDO VIDEO" => {
                Ok(Self::DownloadingVideo)
            }
            "CUTTING_FFMPEG" | "CUTTINGFFMPEG" | "RENDERIZANDO CORTES" => Ok(Self::CuttingFfmpeg),
            "COMPLETED" | "CONCLUÍDO" | "CONCLUIDO" => Ok(Self::Completed),
            "DISCARDED_LOW_SCORE" | "DISCARDEDLOWSCORE" | "DESCARTADO (NOTA < 90)" | "DESCARTADO" => {
                Ok(Self::DiscardedLowScore)
            }
            "FAILED" | "FALHOU" => Ok(Self::Failed),
            other => Err(format!("Status de processamento desconhecido: {}", other)),
        }
    }
}

/// Modo de orientação de tela para renderização do corte.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrientationMode {
    #[default]
    Auto,
    #[serde(rename = "VERTICAL_9_16")]
    Vertical916,
    #[serde(rename = "HORIZONTAL_16_9")]
    Horizontal169,
}

impl OrientationMode {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Auto => "Automático (Vídeos < 1min = 9:16 Vertical, outros = 16:9)",
            Self::Vertical916 => "Retrato Vertical (9:16 - TikTok/Reels/Shorts)",
            Self::Horizontal169 => "Paisagem Horizontal (16:9 - YouTube Padrão)",
        }
    }

    pub fn resolve_for_duration(
        configured_mode: Self,
        duration_seconds: f64,
        auto_threshold_seconds: f64,
    ) -> Self {
        if configured_mode == Self::Auto {
            if duration_seconds <= auto_threshold_seconds {
                Self::Vertical916
            } else {
                Self::Horizontal169
            }
        } else {
            configured_mode
        }
    }
}

impl fmt::Display for OrientationMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}

impl FromStr for OrientationMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let clean = s.trim().to_uppercase().replace("-", "_").replace(" ", "_");
        if clean.contains("AUTO") {
            Ok(Self::Auto)
        } else if clean.contains("VERTICAL") || clean.contains("9_16") || clean.contains("9:16") {
            Ok(Self::Vertical916)
        } else if clean.contains("HORIZONTAL") || clean.contains("16_9") || clean.contains("16:9") {
            Ok(Self::Horizontal169)
        } else {
            Err(format!("Modo de orientação desconhecido: {}", s))
        }
    }
}

/// Modos de enquadramento de vídeo para clipping e renderização de alta retenção.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FramingMode {
    #[default]
    AutoCrop,
    SplitScreen,
    BlurredBg,
    #[serde(rename = "HORIZONTAL_16_9")]
    Horizontal169,
}

impl FramingMode {
    pub fn description(&self) -> &'static str {
        match self {
            Self::AutoCrop => "Auto-Crop 9:16 (Tela Cheia Centralizada)",
            Self::SplitScreen => "Split-Screen 9:16 (Entrevista Empilhada Topo/Base)",
            Self::BlurredBg => "Fundo Desfocado 9:16 (Blurred Background)",
            Self::Horizontal169 => "Paisagem Horizontal 16:9 (Padrão)",
        }
    }

    pub fn is_vertical(&self) -> bool {
        !matches!(self, Self::Horizontal169)
    }

    pub fn to_orientation_mode(&self) -> OrientationMode {
        if self.is_vertical() {
            OrientationMode::Vertical916
        } else {
            OrientationMode::Horizontal169
        }
    }

    pub fn from_str_loose(value: &str, default_mode: Self) -> Self {
        if value.trim().is_empty() {
            return default_mode;
        }
        let clean = value.trim().to_uppercase().replace("-", "_").replace(" ", "_");
        match clean.as_str() {
            "AUTO_CROP" => Self::AutoCrop,
            "SPLIT_SCREEN" => Self::SplitScreen,
            "BLURRED_BG" => Self::BlurredBg,
            "HORIZONTAL_16_9" => Self::Horizontal169,
            _ => {
                if clean.contains("SPLIT") {
                    Self::SplitScreen
                } else if clean.contains("BLUR") {
                    Self::BlurredBg
                } else if clean.contains("HORIZONTAL")
                    || clean.contains("16_9")
                    || clean.contains("16:9")
                    || clean.contains("LANDSCAPE")
                {
                    Self::Horizontal169
                } else if clean.contains("CROP")
                    || clean.contains("AUTO")
                    || clean.contains("VERTICAL")
                    || clean.contains("9_16")
                    || clean.contains("9:16")
                {
                    Self::AutoCrop
                } else {
                    default_mode
                }
            }
        }
    }
}

impl fmt::Display for FramingMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

/// Predefinições de estilo de legendas de alta retenção (padrão Hormozi e variações modernas).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SubtitlePreset {
    #[default]
    HormoziYellow,
    CyberGreen,
    CleanWhite,
    MrbeastBlueYellow,
    NeonMagenta,
    MinimalistElegant,
}

impl SubtitlePreset {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::HormoziYellow => "Hormozi Yellow",
            Self::CyberGreen => "Cyber Green",
            Self::CleanWhite => "Clean White",
            Self::MrbeastBlueYellow => "MrBeast",
            Self::NeonMagenta => "Neon Magenta",
            Self::MinimalistElegant => "Minimalist Elegant",
        }
    }

    pub fn font_name(&self) -> &'static str {
        "Montserrat ExtraBold"
    }

    pub fn primary_color(&self) -> &'static str {
        match self {
            Self::HormoziYellow => "&H0000FFFF",
            Self::CyberGreen => "&H0000FF66",
            Self::CleanWhite => "&H00FFFFFF",
            Self::MrbeastBlueYellow => "&H0000D7FF",
            Self::NeonMagenta => "&H00FF00FF",
            Self::MinimalistElegant => "&H00FFFFFF",
        }
    }

    pub fn highlight_color(&self) -> &'static str {
        match self {
            Self::HormoziYellow => "&H0000FF00",
            Self::CyberGreen => "&H00FFFF00",
            Self::CleanWhite => "&H0000D7FF",
            Self::MrbeastBlueYellow => "&H00FFFF00",
            Self::NeonMagenta => "&H00FFFF00",
            Self::MinimalistElegant => "&H0000E5FF",
        }
    }

    pub fn secondary_highlight_color(&self) -> &'static str {
        match self {
            Self::HormoziYellow => "&H00FFFFFF",
            Self::CyberGreen => "&H00FFFFFF",
            Self::CleanWhite => "&H0000FFFF",
            Self::MrbeastBlueYellow => "&H00FFFFFF",
            Self::NeonMagenta => "&H00FFFFFF",
            Self::MinimalistElegant => "&H00D0D0D0",
        }
    }

    pub fn outline_color(&self) -> &'static str {
        match self {
            Self::MinimalistElegant => "&H001A1A1A",
            _ => "&H00000000",
        }
    }

    pub fn shadow_color(&self) -> &'static str {
        "&H80000000"
    }

    pub fn outline_width(&self) -> i32 {
        match self {
            Self::MinimalistElegant => 3,
            _ => 8,
        }
    }

    pub fn shadow_depth(&self) -> i32 {
        match self {
            Self::MinimalistElegant => 3,
            _ => 2,
        }
    }

    pub fn alignment(&self) -> i32 {
        2
    }

    pub fn vertical_margin_vertical(&self) -> i32 {
        620
    }

    pub fn vertical_margin_horizontal(&self) -> i32 {
        100
    }

    pub fn font_size_vertical(&self) -> i32 {
        64
    }

    pub fn font_size_horizontal(&self) -> i32 {
        48
    }

    pub fn get_margin_v(&self, orientation: OrientationMode) -> i32 {
        if orientation == OrientationMode::Horizontal169 {
            self.vertical_margin_horizontal()
        } else {
            self.vertical_margin_vertical()
        }
    }

    pub fn get_font_size(&self, orientation: OrientationMode) -> i32 {
        if orientation == OrientationMode::Horizontal169 {
            self.font_size_horizontal()
        } else {
            self.font_size_vertical()
        }
    }

    pub fn get_margin_v_framing(&self, framing: FramingMode) -> i32 {
        if framing == FramingMode::Horizontal169 {
            self.vertical_margin_horizontal()
        } else {
            self.vertical_margin_vertical()
        }
    }

    pub fn get_font_size_framing(&self, framing: FramingMode) -> i32 {
        if framing == FramingMode::Horizontal169 {
            self.font_size_horizontal()
        } else {
            self.font_size_vertical()
        }
    }

    pub fn from_str_loose(value: &str, default_preset: Self) -> Self {
        if value.trim().is_empty() {
            return default_preset;
        }
        let clean = value.trim().to_uppercase().replace("-", "_").replace(" ", "_");
        match clean.as_str() {
            "HORMOZI_YELLOW" | "HORMOZI" => Self::HormoziYellow,
            "CYBER_GREEN" => Self::CyberGreen,
            "CLEAN_WHITE" => Self::CleanWhite,
            "MRBEAST" | "MRBEAST_BLUE_YELLOW" => Self::MrbeastBlueYellow,
            "NEON_MAGENTA" => Self::NeonMagenta,
            "MINIMALIST_ELEGANT" => Self::MinimalistElegant,
            _ => {
                if clean.contains("BEAST") || clean.contains("MRBEAST") {
                    Self::MrbeastBlueYellow
                } else if clean.contains("MAGENTA") || clean.contains("NEON") {
                    Self::NeonMagenta
                } else if clean.contains("MINIMAL") || clean.contains("ELEGANT") {
                    Self::MinimalistElegant
                } else if clean.contains("GREEN") || clean.contains("CYBER") {
                    Self::CyberGreen
                } else if clean.contains("WHITE") || clean.contains("CLEAN") {
                    Self::CleanWhite
                } else if clean.contains("YELLOW") || clean.contains("HORMOZI") {
                    Self::HormoziYellow
                } else {
                    default_preset
                }
            }
        }
    }
}

impl fmt::Display for SubtitlePreset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Tags emocionais para classificação e categorização de clipes virais.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EmotionTag {
    PolemicaQuente,
    HumorGargalhada,
    InsightChocante,
    TensaoClimax,
    EmocaoVulneravel,
    #[default]
    AltaEnergia,
    CuriosidadeInstigante,
}

impl EmotionTag {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::PolemicaQuente => "🔥 Polêmica Quente",
            Self::HumorGargalhada => "😂 Gargalhada & Humor",
            Self::InsightChocante => "🤯 Insight & Choque",
            Self::TensaoClimax => "⚡ Tensão & Clímax",
            Self::EmocaoVulneravel => "❤️ Emoção & Vulnerabilidade",
            Self::AltaEnergia => "🚀 Alta Energia",
            Self::CuriosidadeInstigante => "👀 Curiosidade",
        }
    }

    pub fn emoji(&self) -> &'static str {
        match self {
            Self::PolemicaQuente => "🔥",
            Self::HumorGargalhada => "😂",
            Self::InsightChocante => "🤯",
            Self::TensaoClimax => "⚡",
            Self::EmocaoVulneravel => "❤️",
            Self::AltaEnergia => "🚀",
            Self::CuriosidadeInstigante => "👀",
        }
    }

    pub fn badge_color(&self) -> &'static str {
        match self {
            Self::PolemicaQuente => "#FF4500",
            Self::HumorGargalhada => "#FFD700",
            Self::InsightChocante => "#9370DB",
            Self::TensaoClimax => "#FF8C00",
            Self::EmocaoVulneravel => "#FF69B4",
            Self::AltaEnergia => "#00FA9A",
            Self::CuriosidadeInstigante => "#00BFFF",
        }
    }

    pub fn from_str_loose(value: &str, default_tag: Self) -> Self {
        if value.trim().is_empty() {
            return default_tag;
        }
        let clean = value.trim();
        for tag in [
            Self::PolemicaQuente,
            Self::HumorGargalhada,
            Self::InsightChocante,
            Self::TensaoClimax,
            Self::EmocaoVulneravel,
            Self::AltaEnergia,
            Self::CuriosidadeInstigante,
        ] {
            if tag.display_name().eq_ignore_ascii_case(clean) {
                return tag;
            }
        }

        let normalized = clean.to_uppercase().replace("-", "_").replace(" ", "_");
        match normalized.as_str() {
            "POLEMICA_QUENTE" => Self::PolemicaQuente,
            "HUMOR_GARGALHADA" => Self::HumorGargalhada,
            "INSIGHT_CHOCANTE" => Self::InsightChocante,
            "TENSAO_CLIMAX" => Self::TensaoClimax,
            "EMOCAO_VULNERAVEL" => Self::EmocaoVulneravel,
            "ALTA_ENERGIA" => Self::AltaEnergia,
            "CURIOSIDADE_INSTIGANTE" => Self::CuriosidadeInstigante,
            _ => {
                if normalized.contains("POLEMICA")
                    || normalized.contains("QUENTE")
                    || normalized.contains("HOT")
                {
                    Self::PolemicaQuente
                } else if normalized.contains("HUMOR")
                    || normalized.contains("GARGALHADA")
                    || normalized.contains("FUNNY")
                    || normalized.contains("LAUGH")
                {
                    Self::HumorGargalhada
                } else if normalized.contains("INSIGHT")
                    || normalized.contains("CHOCANTE")
                    || normalized.contains("CHOQUE")
                    || normalized.contains("SHOCK")
                {
                    Self::InsightChocante
                } else if normalized.contains("TENSAO")
                    || normalized.contains("CLIMAX")
                    || normalized.contains("TENSION")
                {
                    Self::TensaoClimax
                } else if normalized.contains("VULNERAVEL")
                    || normalized.contains("EMOCAO")
                    || normalized.contains("EMOTIONAL")
                    || normalized.contains("VULNERABILITY")
                {
                    Self::EmocaoVulneravel
                } else if normalized.contains("ENERGIA")
                    || normalized.contains("ALTA")
                    || normalized.contains("ENERGY")
                {
                    Self::AltaEnergia
                } else if normalized.contains("CURIOSIDADE")
                    || normalized.contains("INSTIGANTE")
                    || normalized.contains("CURIOUS")
                    || normalized.contains("CURIOSITY")
                {
                    Self::CuriosidadeInstigante
                } else {
                    default_tag
                }
            }
        }
    }
}

impl fmt::Display for EmotionTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Proporções de aspecto suportadas para capas e thumbnails do Clippr.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ThumbnailAspectRatio {
    #[default]
    #[serde(rename = "HORIZONTAL_16_9")]
    Horizontal169,
    #[serde(rename = "VERTICAL_9_16")]
    Vertical916,
}

impl ThumbnailAspectRatio {
    pub fn width(&self) -> u32 {
        match self {
            Self::Horizontal169 => 1280,
            Self::Vertical916 => 1080,
        }
    }

    pub fn height(&self) -> u32 {
        match self {
            Self::Horizontal169 => 720,
            Self::Vertical916 => 1920,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Horizontal169 => "16:9 Horizontal (YouTube)",
            Self::Vertical916 => "9:16 Vertical (TikTok / Reels / Shorts)",
        }
    }

    pub fn aspect_ratio(&self) -> f64 {
        self.width() as f64 / self.height() as f64
    }

    pub fn from_label(label: &str) -> Self {
        if label.contains("9:16")
            || label.contains("Vertical")
            || label.contains("TikTok")
            || label.contains("Reels")
            || label.contains("Shorts")
        {
            Self::Vertical916
        } else {
            Self::Horizontal169
        }
    }
}

impl fmt::Display for ThumbnailAspectRatio {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Provedor de IA ativo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AiProvider {
    #[default]
    Gemini,
    #[serde(rename = "OPENAI")]
    OpenAi,
}

impl fmt::Display for AiProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Gemini => write!(f, "GEMINI"),
            Self::OpenAi => write!(f, "OPENAI"),
        }
    }
}

impl FromStr for AiProvider {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_uppercase().as_str() {
            "GEMINI" => Ok(Self::Gemini),
            "OPENAI" => Ok(Self::OpenAi),
            other => Err(format!("Provedor desconhecido: {}", other)),
        }
    }
}
