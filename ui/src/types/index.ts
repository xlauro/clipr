export type ProcessingStatus =
  | 'PENDING'
  | 'FETCHING_TRANSCRIPT'
  | 'ANALYZING_AI'
  | 'PENDING_APPROVAL'
  | 'DOWNLOADING_VIDEO'
  | 'CUTTING_FFMPEG'
  | 'COMPLETED'
  | 'DISCARDED_LOW_SCORE'
  | 'FAILED';

export const ProcessingStatusLabels: Record<ProcessingStatus, string> = {
  PENDING: 'Pendente',
  FETCHING_TRANSCRIPT: 'Baixando transcrição',
  ANALYZING_AI: 'Analisando com IA',
  PENDING_APPROVAL: 'Aguardando aprovação',
  DOWNLOADING_VIDEO: 'Baixando vídeo',
  CUTTING_FFMPEG: 'Renderizando cortes',
  COMPLETED: 'Concluído',
  DISCARDED_LOW_SCORE: 'Descartado (Nota < 90)',
  FAILED: 'Falhou',
};

export type OrientationMode = 'AUTO' | 'VERTICAL_9_16' | 'HORIZONTAL_16_9';

export type FramingMode =
  | 'AUTO_CROP'
  | 'SPLIT_SCREEN'
  | 'BLURRED_BG'
  | 'HORIZONTAL_16_9';

export const FramingModeDetails: Record<
  FramingMode,
  { label: string; description: string; icon: string }
> = {
  AUTO_CROP: {
    label: 'Auto-Crop 9:16 (Tela Cheia)',
    description: 'Recorte vertical inteligente centralizado na ação principal.',
    icon: 'Maximize2',
  },
  SPLIT_SCREEN: {
    label: 'Split-Screen 9:16 (Podcasts)',
    description: 'Divisão em dois planos verticais focando host e convidado empilhados.',
    icon: 'Columns2',
  },
  BLURRED_BG: {
    label: 'Fundo Desfocado (9:16)',
    description: 'Vídeo original 16:9 no centro com preenchimento em blur superior/inferior.',
    icon: 'Layers',
  },
  HORIZONTAL_16_9: {
    label: 'Horizontal 16:9 (Padrão)',
    description: 'Mantém a proporção nativa sem corte para o YouTube tradicional.',
    icon: 'Monitor',
  },
};

export type SubtitlePreset =
  | 'HORMOZI_YELLOW'
  | 'CYBER_GREEN'
  | 'CLEAN_WHITE'
  | 'MRBEAST_BLUE_YELLOW'
  | 'NEON_MAGENTA'
  | 'MINIMALIST_ELEGANT';

export const SubtitlePresetDetails: Record<
  SubtitlePreset,
  { label: string; primaryColor: string; highlightColor: string }
> = {
  HORMOZI_YELLOW: {
    label: 'Hormozi Amarelo',
    primaryColor: '#FFFF00',
    highlightColor: '#00FF00',
  },
  MRBEAST_BLUE_YELLOW: {
    label: 'MrBeast (Azul/Amarelo)',
    primaryColor: '#00D7FF',
    highlightColor: '#FFFF00',
  },
  CYBER_GREEN: {
    label: 'Cyber Green',
    primaryColor: '#00FF66',
    highlightColor: '#00FFFF',
  },
  NEON_MAGENTA: {
    label: 'Neon Magenta',
    primaryColor: '#FF00FF',
    highlightColor: '#FFFF00',
  },
  MINIMALIST_ELEGANT: {
    label: 'Minimalista Elegante',
    primaryColor: '#FFFFFF',
    highlightColor: '#00E5FF',
  },
  CLEAN_WHITE: {
    label: 'Clean White',
    primaryColor: '#FFFFFF',
    highlightColor: '#00D7FF',
  },
};

export type EmotionTag =
  | 'POLEMICA_QUENTE'
  | 'HUMOR_GARGALHADA'
  | 'INSIGHT_CHOCANTE'
  | 'TENSAO_CLIMAX'
  | 'EMOCAO_VULNERAVEL'
  | 'ALTA_ENERGIA'
  | 'CURIOSIDADE_INSTIGANTE';

export const EmotionTagDetails: Record<
  EmotionTag,
  { label: string; emoji: string; color: string }
> = {
  POLEMICA_QUENTE: { label: 'Polêmica Quente', emoji: '🔥', color: '#FF4500' },
  HUMOR_GARGALHADA: { label: 'Gargalhada & Humor', emoji: '😂', color: '#FFD700' },
  INSIGHT_CHOCANTE: { label: 'Insight & Choque', emoji: '🤯', color: '#9370DB' },
  TENSAO_CLIMAX: { label: 'Tensão & Clímax', emoji: '⚡', color: '#FF8C00' },
  EMOCAO_VULNERAVEL: { label: 'Emoção & Vulnerabilidade', emoji: '❤️', color: '#FF69B4' },
  ALTA_ENERGIA: { label: 'Alta Energia', emoji: '🚀', color: '#00FA9A' },
  CURIOSIDADE_INSTIGANTE: { label: 'Curiosidade', emoji: '👀', color: '#00BFFF' },
};

export type AiProvider = 'GEMINI' | 'OPENAI';

export interface ScoreBreakdown {
  hookStrength: number;
  emotionalArousal: number;
  storyCohesion: number;
  retentionPotential: number;
  standaloneClarity: number;
  topicNovelty: number;
}

export interface WordTimestamp {
  word: string;
  startSeconds: number;
  endSeconds: number;
}

export interface VideoSource {
  id: number;
  url: string;
  videoId: string;
  title: string;
  author: string;
  durationSeconds: number;
  transcriptPath?: string | null;
  localVideoPath?: string | null;
  status: ProcessingStatus;
  errorMessage?: string | null;
  createdAt: string;
}

export interface ClipCandidate {
  id: number;
  videoSourceId: number;
  title: string;
  hook: string;
  startSeconds: number;
  endSeconds: number;
  viralityScore: number;
  reasoning: string;
  suggestedCaption: string;
  hashtags: string[];
  orientation: OrientationMode;
  hasSubtitles: boolean;
  hasEmojis: boolean;
  outputFilePath?: string | null;
  metadataJsonPath?: string | null;
  status: ProcessingStatus;
  thumbnailTimestamp?: number | null;
  thumbnailHeadline?: string | null;
  thumbnailPath?: string | null;
  rawFramePath?: string | null;
  framingMode: FramingMode;
  subtitlePreset: SubtitlePreset;
  emotionTag: EmotionTag;
  scores: ScoreBreakdown;
}

export interface AppSettings {
  outputDirectory: string;
  minViralityScore: number;
  autoProcessAboveThreshold: boolean;
  defaultOrientation: OrientationMode;
  autoVerticalMaxDurationSeconds: number;
  burnSubtitles: boolean;
  cpuThreads: number;
  geminiApiKey: string;
  geminiModel: string;
  aiProvider: AiProvider;
  openaiApiKey: string;
  openaiModel: string;
  youtubeClientId: string;
  youtubeClientSecret: string;
  youtubeAccessToken: string;
  youtubeRefreshToken: string;
  youtubeTokenExpiresAt: number;
  youtubeChannelTitle: string;
  youtubeChannelId: string;
  youtubeChannelAvatarUrl: string;
}

export interface SystemDiagnostics {
  sqliteConnected: boolean;
  sqliteMessage: string;
  ffmpegOk: boolean;
  ffmpegEncoder: string;
  ffmpegDetails: string;
  ytDlpOk: boolean;
  ytDlpVersion: string;
  geminiOk: boolean;
  geminiMessage: string;
  openAiOk: boolean;
  openAiMessage: string;
}

export type LogCategory = 'ALL' | 'PIPELINE' | 'YTDLP' | 'AI' | 'FFMPEG' | 'THUMBNAIL' | 'DB';

export interface LogMessage {
  id: string;
  timestamp: string;
  level: 'INFO' | 'WARN' | 'ERROR' | 'DEBUG';
  message: string;
  isDebug: boolean;
}

export interface ProgressUpdate {
  stage: string;
  progress: number; // 0.0 to 1.0 (or 0 to 100)
  message: string;
}

export interface Toast {
  id: string;
  type: 'info' | 'success' | 'warning' | 'error';
  title: string;
  message: string;
  timestamp: number;
}
