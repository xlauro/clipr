import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type {
  AppSettings,
  ClipCandidate,
  ProgressUpdate,
  SystemDiagnostics,
  VideoSource,
  WordTimestamp,
} from '../types';

// Mock initial state for browser preview/fallback
let mockSettings: AppSettings = {
  outputDirectory: '/home/user/ClipprExports',
  minViralityScore: 90,
  autoProcessAboveThreshold: false,
  defaultOrientation: 'AUTO',
  autoVerticalMaxDurationSeconds: 60,
  burnSubtitles: true,
  cpuThreads: 0,
  geminiApiKey: '',
  geminiModel: 'gemini-2.5-flash',
  aiProvider: 'GEMINI',
  openaiApiKey: '',
  openaiModel: 'gpt-4o-mini',
  youtubeClientId: '',
  youtubeClientSecret: '',
  youtubeAccessToken: '',
  youtubeRefreshToken: '',
  youtubeTokenExpiresAt: 0,
  youtubeChannelTitle: '',
  youtubeChannelId: '',
  youtubeChannelAvatarUrl: '',
};

let mockDiagnostics: SystemDiagnostics = {
  sqliteConnected: true,
  sqliteMessage: 'SQLite WAL conectado e ativo',
  ffmpegOk: true,
  ffmpegEncoder: 'NVENC (h264_nvenc)',
  ffmpegDetails: 'FFmpeg 7.1 com aceleração por GPU NVIDIA',
  ytDlpOk: true,
  ytDlpVersion: '2025.01.26',
  geminiOk: true,
  geminiMessage: 'Chave configurada e ativa',
  openAiOk: false,
  openAiMessage: 'Chave não informada',
};

let mockVideos: VideoSource[] = [
  {
    id: 1,
    url: 'https://www.youtube.com/watch?v=dQw4w9WgXcQ',
    videoId: 'dQw4w9WgXcQ',
    title: 'Rick Astley - Never Gonna Give You Up (Official Music Video)',
    author: 'Rick Astley',
    durationSeconds: 213,
    status: 'COMPLETED',
    createdAt: new Date().toISOString(),
  },
];

let mockClips: ClipCandidate[] = [
  {
    id: 101,
    videoSourceId: 1,
    title: 'O Segredo da Retenção Absoluta',
    hook: 'Você nunca mais vai cometer esse erro grave ao cortar vídeos...',
    startSeconds: 15.2,
    endSeconds: 58.6,
    viralityScore: 94,
    reasoning: 'Gancho imediato com quebra de expectativa nos primeiros 3 segundos e ritmo acelerado.',
    suggestedCaption: 'Como viralizar qualquer corte em menos de 60 segundos! 🚀 #shorts #viral #growth',
    hashtags: ['#shorts', '#viral', '#edicao', '#growth'],
    orientation: 'VERTICAL_9_16',
    hasSubtitles: true,
    hasEmojis: true,
    status: 'COMPLETED',
    framingMode: 'AUTO_CROP',
    subtitlePreset: 'HORMOZI_YELLOW',
    emotionTag: 'ALTA_ENERGIA',
    thumbnailTimestamp: 22.5,
    thumbnailHeadline: 'PARE DE ERRAR NISSO!',
    scores: {
      hookStrength: 96,
      emotionalArousal: 92,
      storyCohesion: 90,
      retentionPotential: 95,
      standaloneClarity: 94,
      topicNovelty: 91,
    },
  },
  {
    id: 102,
    videoSourceId: 1,
    title: 'A Verdade Chocante que Ninguém Conta',
    hook: 'Eles esconderam isso durante anos na indústria de conteúdo...',
    startSeconds: 75.0,
    endSeconds: 132.4,
    viralityScore: 88,
    reasoning: 'Gera forte curiosidade inicial e engajamento emocional de choque.',
    suggestedCaption: 'A verdade sobre o algoritmo que você precisa saber! 🤯',
    hashtags: ['#podcast', '#reels', '#segredo'],
    orientation: 'VERTICAL_9_16',
    hasSubtitles: true,
    hasEmojis: true,
    status: 'PENDING_APPROVAL',
    framingMode: 'SPLIT_SCREEN',
    subtitlePreset: 'MRBEAST_BLUE_YELLOW',
    emotionTag: 'INSIGHT_CHOCANTE',
    thumbnailTimestamp: 80.0,
    thumbnailHeadline: 'O GRANDE SEGREDO',
    scores: {
      hookStrength: 89,
      emotionalArousal: 88,
      storyCohesion: 87,
      retentionPotential: 86,
      standaloneClarity: 90,
      topicNovelty: 88,
    },
  },
];

const mockWordsMap: Record<number, WordTimestamp[]> = {
  101: [
    { word: 'Você', startSeconds: 15.2, endSeconds: 15.5 },
    { word: 'nunca', startSeconds: 15.5, endSeconds: 15.9 },
    { word: 'mais', startSeconds: 15.9, endSeconds: 16.2 },
    { word: 'vai', startSeconds: 16.2, endSeconds: 16.5 },
    { word: 'cometer', startSeconds: 16.5, endSeconds: 17.0 },
    { word: 'esse', startSeconds: 17.0, endSeconds: 17.3 },
    { word: 'erro', startSeconds: 17.3, endSeconds: 17.7 },
    { word: 'grave', startSeconds: 17.7, endSeconds: 18.2 },
    { word: 'ao', startSeconds: 18.2, endSeconds: 18.4 },
    { word: 'cortar', startSeconds: 18.4, endSeconds: 18.9 },
    { word: 'vídeos.', startSeconds: 18.9, endSeconds: 19.5 },
  ],
};

async function safeInvoke<T>(cmd: string, args?: Record<string, unknown>, fallback?: T): Promise<T> {
  if (isTauri()) {
    try {
      return await invoke<T>(cmd, args);
    } catch (err) {
      console.warn(`[Tauri IPC] Failed '${cmd}', returning fallback:`, err);
      if (fallback !== undefined) return fallback;
      throw err;
    }
  }
  return fallback as T;
}

export const api = {
  isTauriRuntime(): boolean {
    return isTauri();
  },

  async getDiagnostics(): Promise<SystemDiagnostics> {
    return safeInvoke<SystemDiagnostics>('get_diagnostics', undefined, mockDiagnostics);
  },

  async getSettings(): Promise<AppSettings> {
    return safeInvoke<AppSettings>('get_settings', undefined, mockSettings);
  },

  async saveSettings(settings: AppSettings): Promise<boolean> {
    mockSettings = { ...settings };
    return safeInvoke<boolean>('save_settings', { settings }, true);
  },

  async detectGeminiApiKey(): Promise<string> {
    return safeInvoke<string>('detect_gemini_key', undefined, 'AIzaSy' + Math.random().toString(36).substring(2, 15));
  },

  async detectOpenAiApiKey(): Promise<string> {
    return safeInvoke<string>('detect_openai_key', undefined, 'sk-' + Math.random().toString(36).substring(2, 15));
  },

  async getVideos(): Promise<VideoSource[]> {
    return safeInvoke<VideoSource[]>('get_videos', undefined, mockVideos);
  },

  async addVideoUrl(url: string): Promise<VideoSource> {
    const newVideo: VideoSource = {
      id: Date.now(),
      url,
      videoId: url.split('v=')[1]?.substring(0, 11) || 'vid_' + Date.now(),
      title: 'Vídeo importado: ' + url,
      author: 'YouTube Creator',
      durationSeconds: 180,
      status: 'PENDING',
      createdAt: new Date().toISOString(),
    };
    mockVideos = [newVideo, ...mockVideos];
    return safeInvoke<VideoSource>('add_video_url', { url }, newVideo);
  },

  async clearQueue(): Promise<boolean> {
    mockVideos = [];
    return safeInvoke<boolean>('clear_queue', undefined, true);
  },

  async startPipeline(options: {
    burnSubtitles: boolean;
    autoProcessAboveThreshold: boolean;
    orientation: string;
  }): Promise<boolean> {
    return safeInvoke<boolean>('start_pipeline', { options }, true);
  },

  async getClips(): Promise<ClipCandidate[]> {
    return safeInvoke<ClipCandidate[]>('get_clips', undefined, mockClips);
  },

  async updateClipBounds(clipId: number, startSeconds: number, endSeconds: number): Promise<boolean> {
    const clip = mockClips.find((c) => c.id === clipId);
    if (clip) {
      clip.startSeconds = startSeconds;
      clip.endSeconds = endSeconds;
    }
    return safeInvoke<boolean>('update_clip_bounds', { clipId, startSeconds, endSeconds }, true);
  },

  async updateClipFramingSubtitle(
    clipId: number,
    framingMode: string,
    subtitlePreset: string
  ): Promise<boolean> {
    const clip = mockClips.find((c) => c.id === clipId);
    if (clip) {
      clip.framingMode = framingMode as any;
      clip.subtitlePreset = subtitlePreset as any;
    }
    return safeInvoke<boolean>(
      'update_clip_framing_subtitle',
      { clipId, framingMode, subtitlePreset },
      true
    );
  },

  async getClipWords(clipId: number): Promise<WordTimestamp[]> {
    const words = mockWordsMap[clipId] || [];
    return safeInvoke<WordTimestamp[]>('get_clip_words', { clipId }, words);
  },

  async saveClipWords(clipId: number, words: WordTimestamp[]): Promise<boolean> {
    mockWordsMap[clipId] = [...words];
    return safeInvoke<boolean>('save_clip_words', { clipId, words }, true);
  },

  async renderClip(clipId: number): Promise<boolean> {
    const clip = mockClips.find((c) => c.id === clipId);
    if (clip) {
      clip.status = 'CUTTING_FFMPEG';
      setTimeout(() => {
        clip.status = 'COMPLETED';
        clip.outputFilePath = `/home/user/ClipprExports/clip_${clipId}.mp4`;
      }, 1500);
    }
    return safeInvoke<boolean>('render_clip', { clipId }, true);
  },

  async renderAllClips(): Promise<boolean> {
    for (const clip of mockClips) {
      clip.status = 'CUTTING_FFMPEG';
    }
    return safeInvoke<boolean>('render_all_clips', undefined, true);
  },

  async generateThumbnail(
    clipId: number,
    timestamp: number,
    headline: string,
    aspectRatio: '16:9' | '9:16'
  ): Promise<{ thumbnailPath: string; rawFramePath: string }> {
    const clip = mockClips.find((c) => c.id === clipId);
    const result = {
      thumbnailPath: `/home/user/ClipprExports/thumb_${clipId}_${aspectRatio.replace(':', '_')}.jpg`,
      rawFramePath: `/home/user/ClipprExports/raw_${clipId}.jpg`,
    };
    if (clip) {
      clip.thumbnailTimestamp = timestamp;
      clip.thumbnailHeadline = headline;
      clip.thumbnailPath = result.thumbnailPath;
      clip.rawFramePath = result.rawFramePath;
    }
    return safeInvoke<{ thumbnailPath: string; rawFramePath: string }>(
      'generate_thumbnail',
      { clipId, timestamp, headline, aspectRatio },
      result
    );
  },

  async openPath(path: string): Promise<boolean> {
    return safeInvoke<boolean>('open_path', { path }, true);
  },

  async transcribeClipWhisper(clipId: number): Promise<WordTimestamp[]> {
    const generated: WordTimestamp[] = [
      { word: 'Isso', startSeconds: 0.0, endSeconds: 0.4 },
      { word: 'aqui', startSeconds: 0.4, endSeconds: 0.7 },
      { word: 'muda', startSeconds: 0.7, endSeconds: 1.0 },
      { word: 'completamente', startSeconds: 1.0, endSeconds: 1.8 },
      { word: 'o', startSeconds: 1.8, endSeconds: 1.9 },
      { word: 'jogo', startSeconds: 1.9, endSeconds: 2.4 },
    ];
    mockWordsMap[clipId] = generated;
    return safeInvoke<WordTimestamp[]>('transcribe_clip_whisper', { clipId }, generated);
  },

  async youtubeConnect(): Promise<{ channelTitle: string; avatarUrl: string }> {
    const mockChannel = {
      channelTitle: 'Clippr Oficial (YouTube Shorts)',
      avatarUrl: 'https://images.unsplash.com/photo-1618005182384-a83a8bd57fbe?w=100&h=100&fit=crop',
    };
    mockSettings.youtubeChannelTitle = mockChannel.channelTitle;
    mockSettings.youtubeChannelAvatarUrl = mockChannel.avatarUrl;
    mockSettings.youtubeAccessToken = 'mock_access_token';
    mockSettings.youtubeRefreshToken = 'mock_refresh_token';
    return safeInvoke('youtube_connect', undefined, mockChannel);
  },

  async youtubeDisconnect(): Promise<boolean> {
    mockSettings.youtubeChannelTitle = '';
    mockSettings.youtubeChannelAvatarUrl = '';
    mockSettings.youtubeAccessToken = '';
    mockSettings.youtubeRefreshToken = '';
    return safeInvoke<boolean>('youtube_disconnect', undefined, true);
  },

  async youtubeSaveCredentials(clientId: string, clientSecret: string): Promise<boolean> {
    mockSettings.youtubeClientId = clientId;
    mockSettings.youtubeClientSecret = clientSecret;
    return safeInvoke<boolean>('youtube_save_credentials', { clientId, clientSecret }, true);
  },

  async youtubePublishClip(clipId: number): Promise<boolean> {
    return safeInvoke<boolean>('youtube_publish_clip', { clipId }, true);
  },

  async selectDirectory(): Promise<string | null> {
    return safeInvoke<string | null>('select_directory', undefined, '/home/user/ClipprExports');
  },
};

// Event Listeners
export async function listenToClipprEvents(callbacks: {
  onProgress?: (progress: ProgressUpdate) => void;
  onLog?: (log: { level: 'INFO' | 'WARN' | 'ERROR' | 'DEBUG'; message: string; isDebug: boolean; timestamp: string }) => void;
  onVideoUpdated?: (video: VideoSource) => void;
  onClipsEvaluated?: (clips: ClipCandidate[]) => void;
  onClipUpdated?: (clip: ClipCandidate) => void;
  onDiagnosticsUpdated?: (diag: SystemDiagnostics) => void;
}): Promise<UnlistenFn[]> {
  if (!isTauri()) {
    return [];
  }

  const unlistens: UnlistenFn[] = [];

  if (callbacks.onProgress) {
    unlistens.push(await listen<ProgressUpdate>('clippr://progress', (event) => callbacks.onProgress?.(event.payload)));
  }

  if (callbacks.onLog) {
    unlistens.push(
      await listen<{ level: 'INFO' | 'WARN' | 'ERROR' | 'DEBUG'; message: string; isDebug: boolean; timestamp: string }>(
        'clippr://log',
        (event) => callbacks.onLog?.(event.payload)
      )
    );
  }

  if (callbacks.onVideoUpdated) {
    unlistens.push(await listen<VideoSource>('clippr://video_updated', (event) => callbacks.onVideoUpdated?.(event.payload)));
  }

  if (callbacks.onClipsEvaluated) {
    unlistens.push(await listen<ClipCandidate[]>('clippr://clips_evaluated', (event) => callbacks.onClipsEvaluated?.(event.payload)));
  }

  if (callbacks.onClipUpdated) {
    unlistens.push(await listen<ClipCandidate>('clippr://clip_updated', (event) => callbacks.onClipUpdated?.(event.payload)));
  }

  if (callbacks.onDiagnosticsUpdated) {
    unlistens.push(await listen<SystemDiagnostics>('clippr://diagnostics_updated', (event) => callbacks.onDiagnosticsUpdated?.(event.payload)));
  }

  return unlistens;
}
