import { api, listenToClipprEvents } from '../ipc';
import type {
  AppSettings,
  ClipCandidate,
  FramingMode,
  LogCategory,
  LogMessage,
  OrientationMode,
  SubtitlePreset,
  SystemDiagnostics,
  Toast,
  VideoSource,
  WordTimestamp,
} from '../../types';

class AppState {
  activeTab = $state<'queue' | 'clips' | 'settings'>('queue');

  // Queue tab settings
  queueOptions = $state({
    burnSubtitles: true,
    autoProcessAboveThreshold: false,
    defaultOrientation: 'AUTO' as OrientationMode,
  });

  // Video Queue
  videos = $state<VideoSource[]>([]);
  urlInput = $state('');

  // Clips
  clips = $state<ClipCandidate[]>([]);
  selectedClipId = $state<number | null>(null);
  clipFilter = $state<'ALL' | 'VIRAL_90' | 'LONG' | 'SHORT' | 'RENDERED' | 'PENDING'>('ALL');
  activeClipSubTab = $state<'player' | 'framing' | 'thumbnail' | 'subtitles'>('player');

  // Words for the active clip
  currentClipWords = $state<WordTimestamp[]>([]);
  selectedWordIndex = $state<number | null>(null);
  editWordText = $state('');
  hasDynamicEmojis = $state(true);

  // Thumbnail State
  thumbnailHeadline = $state('');
  thumbnailTimestamp = $state(0);
  thumbnailAspectRatio = $state<'16:9' | '9:16'>('9:16');
  thumbnailPreview = $state<string | null>(null);
  rawFramePreview = $state<string | null>(null);

  // Player State
  currentTime = $state(0);
  videoDuration = $state(60);
  isPlaying = $state(false);
  volume = $state(0.8);
  showCropGuide = $state(false);

  // Settings & Environment
  settings = $state<AppSettings>({
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
  });

  // Diagnostics
  diagnostics = $state<SystemDiagnostics>({
    sqliteConnected: true,
    sqliteMessage: 'SQLite WAL conectado e ativo',
    ffmpegOk: true,
    ffmpegEncoder: 'NVENC (h264_nvenc)',
    ffmpegDetails: 'FFmpeg OK com suporte por hardware',
    ytDlpOk: true,
    ytDlpVersion: '2025.01.26',
    geminiOk: false,
    geminiMessage: 'Chave não informada',
    openAiOk: false,
    openAiMessage: 'Chave não informada',
  });

  // Global Status & Progress
  statusLabel = $state('Pronto para processar vídeos');
  progressPercent = $state(0);
  isProcessing = $state(false);

  // Logs & Console Drawer
  isLogDrawerOpen = $state(false);
  isDebugMode = $state(false);
  selectedLogCategory = $state<LogCategory>('ALL');
  logs = $state<LogMessage[]>([]);

  // Notifications
  toasts = $state<Toast[]>([]);

  // Derived state
  selectedClip = $derived<ClipCandidate | null>(
    this.clips.find((c) => c.id === this.selectedClipId) || (this.clips.length > 0 ? this.clips[0] : null)
  );

  filteredClips = $derived.by(() => {
    switch (this.clipFilter) {
      case 'VIRAL_90':
        return this.clips.filter((c) => c.viralityScore >= 90);
      case 'LONG':
        return this.clips.filter((c) => c.endSeconds - c.startSeconds > 90);
      case 'SHORT':
        return this.clips.filter((c) => c.endSeconds - c.startSeconds <= 90);
      case 'RENDERED':
        return this.clips.filter((c) => c.status === 'COMPLETED' || !!c.outputFilePath);
      case 'PENDING':
        return this.clips.filter((c) => c.status !== 'COMPLETED' && !c.outputFilePath);
      default:
        return this.clips;
    }
  });

  filteredLogs = $derived.by(() => {
    const baseLogs = this.isDebugMode ? this.logs : this.logs.filter((l) => !l.isDebug);
    if (this.selectedLogCategory === 'ALL') {
      return baseLogs;
    }
    return baseLogs.filter((l) => {
      const msg = l.message;
      switch (this.selectedLogCategory) {
        case 'PIPELINE':
          return msg.includes('[PIPELINE]');
        case 'YTDLP':
          return msg.includes('[YTDLP]');
        case 'AI':
          return (
            msg.includes('[AI-GEMINI]') ||
            msg.includes('[AI-OPENAI]') ||
            msg.includes('[AI]')
          );
        case 'FFMPEG':
          return msg.includes('[FFMPEG]');
        case 'THUMBNAIL':
          return msg.includes('[THUMBNAIL]');
        case 'DB':
          return msg.includes('[DB]');
        default:
          return true;
      }
    });
  });

  logCategoryCounts = $derived.by(() => {
    const baseLogs = this.isDebugMode ? this.logs : this.logs.filter((l) => !l.isDebug);
    const counts: Record<LogCategory, number> = {
      ALL: baseLogs.length,
      PIPELINE: 0,
      YTDLP: 0,
      AI: 0,
      FFMPEG: 0,
      THUMBNAIL: 0,
      DB: 0,
    };
    for (const log of baseLogs) {
      const msg = log.message;
      if (msg.includes('[PIPELINE]')) counts.PIPELINE++;
      if (msg.includes('[YTDLP]')) counts.YTDLP++;
      if (
        msg.includes('[AI-GEMINI]') ||
        msg.includes('[AI-OPENAI]') ||
        msg.includes('[AI]')
      ) {
        counts.AI++;
      }
      if (msg.includes('[FFMPEG]')) counts.FFMPEG++;
      if (msg.includes('[THUMBNAIL]')) counts.THUMBNAIL++;
      if (msg.includes('[DB]')) counts.DB++;
    }
    return counts;
  });

  constructor() {
    this.init();
  }

  async init() {
    this.addLog('INFO', 'Clippr inicializado com sucesso (Svelte 5 Runes + Tauri v2).');
    await this.refreshDiagnostics();
    await this.loadSettings();
    await this.loadVideos();
    await this.loadClips();
    this.setupEventListeners();
  }

  async setupEventListeners() {
    try {
      await listenToClipprEvents({
        onProgress: (p) => {
          this.statusLabel = p.message || p.stage;
          this.progressPercent = p.progress <= 1 ? Math.round(p.progress * 100) : Math.round(p.progress);
          this.isProcessing = this.progressPercent > 0 && this.progressPercent < 100;
        },
        onLog: (l) => {
          this.addLog(l.level, l.message, l.isDebug);
        },
        onVideoUpdated: (v) => {
          const idx = this.videos.findIndex((x) => x.id === v.id);
          if (idx >= 0) {
            this.videos[idx] = v;
          } else {
            this.videos.unshift(v);
          }
        },
        onClipsEvaluated: (clips) => {
          this.clips = clips;
          if (clips.length > 0 && !this.selectedClipId) {
            this.selectClip(clips[0].id);
          }
          this.addToast('success', 'Momentos Virais', `${clips.length} momentos avaliados com sucesso!`);
        },
        onClipUpdated: (clip) => {
          const idx = this.clips.findIndex((c) => c.id === clip.id);
          if (idx >= 0) {
            this.clips[idx] = clip;
          }
        },
        onDiagnosticsUpdated: (diag) => {
          this.diagnostics = diag;
        },
      });
    } catch (e) {
      console.warn('Events registration failed (browser mode):', e);
    }
  }

  addToast(type: Toast['type'], title: string, message: string) {
    const id = Math.random().toString(36).substring(2, 9);
    const toast: Toast = { id, type, title, message, timestamp: Date.now() };
    this.toasts = [...this.toasts, toast];
    setTimeout(() => {
      this.dismissToast(id);
    }, 4500);
  }

  dismissToast(id: string) {
    this.toasts = this.toasts.filter((t) => t.id !== id);
  }

  addLog(level: LogMessage['level'], message: string, isDebug = false) {
    const log: LogMessage = {
      id: Math.random().toString(36).substring(2, 9),
      timestamp: new Date().toLocaleTimeString('pt-BR', { hour12: false, fractionalSecondDigits: 3 }),
      level,
      message,
      isDebug,
    };
    this.logs = [...this.logs, log];
  }

  clearLogs() {
    this.logs = [];
  }

  async copyLogsToClipboard() {
    if (this.filteredLogs.length === 0) {
      this.addToast('info', 'Console', 'Nenhum log para copiar.');
      return;
    }
    const formatted = this.filteredLogs
      .map((l) => `[${l.timestamp}] [${l.level}] ${l.message}`)
      .join('\n');
    try {
      await navigator.clipboard.writeText(formatted);
      this.addToast('success', 'Console', 'Logs copiados para a área de transferência!');
    } catch (e) {
      console.error('Falha ao copiar logs para a área de transferência:', e);
      this.addToast('error', 'Console', 'Falha ao copiar logs para a área de transferência.');
    }
  }

  async refreshDiagnostics() {
    try {
      this.diagnostics = await api.getDiagnostics();
    } catch (e) {
      console.warn('Failed to load diagnostics', e);
    }
  }

  async loadSettings() {
    try {
      const s = await api.getSettings();
      this.settings = s;
      this.queueOptions.burnSubtitles = s.burnSubtitles;
      this.queueOptions.autoProcessAboveThreshold = s.autoProcessAboveThreshold;
      this.queueOptions.defaultOrientation = s.defaultOrientation;
    } catch (e) {
      console.warn('Failed to load settings', e);
    }
  }

  async saveSettings() {
    try {
      await api.saveSettings(this.settings);
      this.addToast('success', 'Configurações', 'Preferências salvas com sucesso!');
      await this.refreshDiagnostics();
    } catch (e) {
      this.addToast('error', 'Erro', 'Falha ao salvar configurações.');
    }
  }

  async loadVideos() {
    try {
      this.videos = await api.getVideos();
    } catch (e) {
      console.warn('Failed to load videos', e);
    }
  }

  async addVideo() {
    const url = this.urlInput.trim();
    if (!url) return;
    try {
      this.addLog('INFO', `Adicionando à fila: ${url}`);
      const video = await api.addVideoUrl(url);
      this.videos = [video, ...this.videos];
      this.urlInput = '';
      this.addToast('success', 'Vídeo Adicionado', 'O vídeo foi enfileirado com sucesso.');
    } catch (e) {
      this.addToast('error', 'Erro ao Adicionar', 'Não foi possível enfileirar o vídeo.');
    }
  }

  async clearQueue() {
    await api.clearQueue();
    this.videos = [];
    this.addToast('info', 'Fila Limpa', 'Todos os vídeos foram removidos da fila.');
    this.addLog('INFO', 'Fila de vídeos limpa pelo usuário.');
  }

  async startPipeline() {
    if (this.videos.length === 0) {
      this.addToast('warning', 'Fila Vazia', 'Adicione pelo menos um link do YouTube antes de iniciar.');
      return;
    }
    this.isProcessing = true;
    this.statusLabel = 'Iniciando pipeline de processamento...';
    this.progressPercent = 5;
    this.addLog('INFO', 'Iniciando pipeline para vídeos na fila...');

    try {
      await api.startPipeline({
        burnSubtitles: this.queueOptions.burnSubtitles,
        autoProcessAboveThreshold: this.queueOptions.autoProcessAboveThreshold,
        orientation: this.queueOptions.defaultOrientation,
      });
      this.addToast('info', 'Processamento Iniciado', 'Download e análise IA em andamento.');
    } catch (e) {
      this.isProcessing = false;
      this.addToast('error', 'Erro', 'Falha ao iniciar processamento.');
    }
  }

  async loadClips() {
    try {
      const clips = await api.getClips();
      this.clips = clips;
      if (clips.length > 0) {
        this.selectClip(clips[0].id);
      }
    } catch (e) {
      console.warn('Failed to load clips', e);
    }
  }

  async selectClip(clipId: number) {
    this.selectedClipId = clipId;
    const clip = this.clips.find((c) => c.id === clipId);
    if (!clip) return;

    this.thumbnailHeadline = clip.thumbnailHeadline || clip.hook || 'GANCHO MAGNÉTICO';
    this.thumbnailTimestamp = clip.thumbnailTimestamp ?? clip.startSeconds;
    this.thumbnailPreview = clip.thumbnailPath || null;
    this.rawFramePreview = clip.rawFramePath || null;
    this.currentTime = clip.startSeconds;
    this.videoDuration = Math.max(1, clip.endSeconds);

    // Carregar palavras da transcrição
    try {
      this.currentClipWords = await api.getClipWords(clipId);
    } catch (e) {
      this.currentClipWords = [];
    }
  }

  async adjustClipBounds(deltaStart: number, deltaEnd: number) {
    const clip = this.selectedClip;
    if (!clip) return;

    const newStart = Math.max(0, Math.round((clip.startSeconds + deltaStart) * 100) / 100);
    const newEnd = Math.max(newStart + 1, Math.round((clip.endSeconds + deltaEnd) * 100) / 100);

    clip.startSeconds = newStart;
    clip.endSeconds = newEnd;
    this.currentTime = newStart;

    await api.updateClipBounds(clip.id, newStart, newEnd);
    this.addLog('DEBUG', `Clip #${clip.id} bounds atualizados: ${newStart.toFixed(2)}s -> ${newEnd.toFixed(2)}s`, true);
  }

  async setStartToCurrent() {
    const clip = this.selectedClip;
    if (!clip) return;
    const newStart = Math.min(this.currentTime, clip.endSeconds - 1);
    clip.startSeconds = newStart;
    await api.updateClipBounds(clip.id, newStart, clip.endSeconds);
    this.addToast('info', 'Marcador de Início', `Início ajustado para ${newStart.toFixed(2)}s`);
  }

  async setEndToCurrent() {
    const clip = this.selectedClip;
    if (!clip) return;
    const newEnd = Math.max(this.currentTime, clip.startSeconds + 1);
    clip.endSeconds = newEnd;
    await api.updateClipBounds(clip.id, clip.startSeconds, newEnd);
    this.addToast('info', 'Marcador de Fim', `Fim ajustado para ${newEnd.toFixed(2)}s`);
  }

  async updateFraming(mode: FramingMode) {
    const clip = this.selectedClip;
    if (!clip) return;
    clip.framingMode = mode;
    await api.updateClipFramingSubtitle(clip.id, mode, clip.subtitlePreset);
    this.addToast('info', 'Enquadramento', `Modo alterado para ${mode}`);
  }

  async updateSubtitlePreset(preset: SubtitlePreset) {
    const clip = this.selectedClip;
    if (!clip) return;
    clip.subtitlePreset = preset;
    await api.updateClipFramingSubtitle(clip.id, clip.framingMode, preset);
    this.addToast('info', 'Estilo de Legenda', `Preset alterado para ${preset}`);
  }

  async renderSelectedClip() {
    const clip = this.selectedClip;
    if (!clip) return;
    this.statusLabel = `Renderizando corte "${clip.title}"...`;
    this.addLog('INFO', `Renderizando corte ID #${clip.id} (${clip.framingMode}, ${clip.subtitlePreset})...`);
    await api.renderClip(clip.id);
    this.addToast('info', 'Renderização Iniciada', `Renderizando corte com aceleração ${this.diagnostics.ffmpegEncoder}.`);
  }

  async renderAllClips() {
    this.statusLabel = 'Renderizando todos os cortes virais em lote...';
    this.addLog('INFO', 'Renderização em lote disparada (100% CPU / NVENC).');
    await api.renderAllClips();
    this.addToast('info', 'Renderização em Lote', 'Renderização de todos os cortes iniciada.');
  }

  async publishSelectedClip() {
    const clip = this.selectedClip;
    if (!clip) return;
    if (!this.settings.youtubeChannelTitle) {
      this.addToast('warning', 'YouTube Não Conectado', 'Conecte seu canal oficial na aba de Configurações primeiro.');
      this.activeTab = 'settings';
      return;
    }
    this.addLog('INFO', `Publicando corte #${clip.id} no YouTube Shorts...`);
    await api.youtubePublishClip(clip.id);
    this.addToast('success', 'Publicação YouTube', `Corte "${clip.title}" enviado para a fila do YouTube!`);
  }

  async generateThumbnail() {
    const clip = this.selectedClip;
    if (!clip) return;
    this.statusLabel = 'Gerando thumbnail magnética com FFmpeg...';
    this.addLog('INFO', `Gerando capa para corte #${clip.id} em ${this.thumbnailTimestamp.toFixed(2)}s`);
    try {
      const res = await api.generateThumbnail(
        clip.id,
        this.thumbnailTimestamp,
        this.thumbnailHeadline,
        this.thumbnailAspectRatio
      );
      this.thumbnailPreview = res.thumbnailPath;
      this.rawFramePreview = res.rawFramePath;
      clip.thumbnailPath = res.thumbnailPath;
      clip.rawFramePath = res.rawFramePath;
      this.statusLabel = 'Capa gerada com sucesso!';
      this.addToast('success', 'Capa Gerada', 'Thumbnail de alto CTR criada e salva.');
    } catch (e) {
      this.addToast('error', 'Erro', 'Falha ao gerar capa.');
    }
  }

  async openPath(path: string | null | undefined) {
    if (!path) return;
    await api.openPath(path);
  }

  async transcribeWhisper() {
    const clip = this.selectedClip;
    if (!clip) return;
    this.statusLabel = 'Transcrevendo áudio com Whisper local...';
    this.addLog('INFO', `Iniciando extração de áudio e transcrição Whisper para o corte #${clip.id}`);
    try {
      this.currentClipWords = await api.transcribeClipWhisper(clip.id);
      this.statusLabel = 'Transcrição Whisper concluída!';
      this.addToast('success', 'Whisper Transcrição', `${this.currentClipWords.length} palavras sincronizadas!`);
    } catch (e) {
      this.addToast('error', 'Erro', 'Falha ao executar Whisper.');
    }
  }

  selectWord(index: number) {
    this.selectedWordIndex = index;
    const w = this.currentClipWords[index];
    if (w) {
      this.editWordText = w.word;
      this.currentTime = w.startSeconds;
    }
  }

  applyWordEdit() {
    if (this.selectedWordIndex === null || !this.currentClipWords[this.selectedWordIndex]) return;
    this.currentClipWords[this.selectedWordIndex].word = this.editWordText;
    this.addToast('info', 'Palavra Atualizada', `"${this.editWordText}" sincronizada.`);
  }

  async saveSubtitleWords() {
    const clip = this.selectedClip;
    if (!clip) return;
    await api.saveClipWords(clip.id, this.currentClipWords);
    this.addToast('success', 'Legendas Salvas', 'Palavras e timestamps salvos no banco local.');
  }

  async detectGeminiKey() {
    try {
      const key = await api.detectGeminiApiKey();
      this.settings.geminiApiKey = key;
      this.addToast('success', 'Gemini AI', 'Chave encontrada no ambiente com sucesso!');
      await this.saveSettings();
    } catch (e) {
      this.addToast('error', 'Gemini AI', 'Nenhuma chave encontrada em GEMINI_API_KEY ou dotfiles.');
    }
  }

  async detectOpenAiKey() {
    try {
      const key = await api.detectOpenAiApiKey();
      this.settings.openaiApiKey = key;
      this.addToast('success', 'OpenAI ChatGPT', 'Chave encontrada no ambiente com sucesso!');
      await this.saveSettings();
    } catch (e) {
      this.addToast('error', 'OpenAI', 'Nenhuma chave encontrada em OPENAI_API_KEY ou dotfiles.');
    }
  }

  async selectDirectory() {
    const dir = await api.selectDirectory();
    if (dir) {
      this.settings.outputDirectory = dir;
      await this.saveSettings();
    }
  }

  async connectYouTube() {
    try {
      this.statusLabel = 'Iniciando autenticação OAuth 2.0 PKCE do YouTube...';
      const ch = await api.youtubeConnect();
      this.settings.youtubeChannelTitle = ch.channelTitle;
      this.settings.youtubeChannelAvatarUrl = ch.avatarUrl;
      this.statusLabel = `Conectado ao canal: ${ch.channelTitle}`;
      this.addToast('success', 'YouTube Conectado', `Canal "${ch.channelTitle}" autenticado com sucesso!`);
    } catch (e) {
      this.addToast('error', 'YouTube OAuth', 'Falha ao autenticar com o YouTube.');
    }
  }

  async disconnectYouTube() {
    await api.youtubeDisconnect();
    this.settings.youtubeChannelTitle = '';
    this.settings.youtubeChannelAvatarUrl = '';
    this.addToast('info', 'YouTube', 'Canal desconectado com sucesso.');
  }

  async saveYouTubeCredentials() {
    await api.youtubeSaveCredentials(this.settings.youtubeClientId, this.settings.youtubeClientSecret);
    this.addToast('success', 'Credenciais YouTube', 'Client ID e Client Secret salvos com segurança.');
  }
}

export const appState = new AppState();
