<script lang="ts">
  import { appState } from '../../lib/stores/appState.svelte';
  import { api } from '../../lib/ipc';
  import { ProcessingStatusLabels, type ProcessingStatus } from '../../types';
  import {
    Plus,
    Play,
    Trash2,
    Tv,
    Sparkles,
    Captions,
    CheckCircle2,
    Clock,
    AlertCircle,
    Loader2,
  } from 'lucide-svelte';

  function formatDuration(seconds: number): string {
    if (!seconds || seconds <= 0) return '-';
    const total = Math.round(seconds);
    const m = Math.floor(total / 60);
    const s = total % 60;
    return `${m}m ${s.toString().padStart(2, '0')}s`;
  }

  function getStatusBadgeClass(status: ProcessingStatus): string {
    switch (status) {
      case 'PENDING':
      case 'PENDING_APPROVAL':
        return 'bg-zinc-800 text-zinc-300 border-zinc-700';
      case 'FETCHING_TRANSCRIPT':
      case 'DOWNLOADING_VIDEO':
        return 'bg-sky-950/60 text-sky-300 border-sky-700/60 animate-pulse';
      case 'ANALYZING_AI':
        return 'bg-purple-950/60 text-purple-300 border-purple-700/60 animate-pulse';
      case 'CUTTING_FFMPEG':
        return 'bg-amber-950/60 text-amber-300 border-amber-700/60 animate-pulse';
      case 'COMPLETED':
        return 'bg-emerald-950/60 text-emerald-300 border-emerald-700/60';
      case 'DISCARDED_LOW_SCORE':
        return 'bg-zinc-900 text-zinc-400 border-zinc-800';
      case 'FAILED':
        return 'bg-rose-950/60 text-rose-300 border-rose-700/60';
      default:
        return 'bg-zinc-800 text-zinc-300 border-zinc-700';
    }
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      appState.addVideo();
    }
  }
</script>

<div class="h-full flex flex-col p-4 space-y-4 overflow-hidden">
  {#if !api.isTauriRuntime()}
    <div class="bg-amber-950/40 border border-amber-600/30 rounded-xl px-3.5 py-2.5 flex items-center justify-between text-xs text-amber-200 shadow-sm shrink-0">
      <div class="flex items-center space-x-2">
        <span class="text-base">⚡</span>
        <span><strong>Modo Navegador (Preview)</strong>: Para processamento nativo completo com FFmpeg, Whisper e GPU acelerada, execute no terminal: <code class="bg-amber-900/40 px-1.5 py-0.5 rounded font-mono text-amber-100">npm run tauri:dev</code></span>
      </div>
    </div>
  {/if}

  <!-- Top: Add YouTube Link -->
  <div class="flex items-center space-x-3 bg-zinc-900/70 p-3 rounded-xl border border-zinc-800/80 shadow-md">
    <div class="relative flex-1">
      <input
        type="text"
        bind:value={appState.urlInput}
        onkeydown={handleKeyDown}
        placeholder="Cole o link do YouTube aqui (ex: https://www.youtube.com/watch?v=...)"
        class="w-full bg-zinc-950 border border-zinc-700/80 rounded-lg px-3.5 py-2 text-xs text-white placeholder-zinc-500 focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:border-transparent transition-all"
      />
    </div>
    <button
      onclick={() => appState.addVideo()}
      class="flex items-center space-x-1.5 bg-indigo-600 hover:bg-indigo-500 active:bg-indigo-700 text-white px-4 py-2 rounded-lg text-xs font-semibold shadow-md shadow-indigo-600/20 transition-all shrink-0 cursor-pointer"
    >
      <Plus class="w-4 h-4" />
      <span>+ Adicionar à Fila</span>
    </button>
  </div>

  <!-- Video Queue Table / List -->
  <div class="flex-1 bg-zinc-900/50 rounded-xl border border-zinc-800/80 flex flex-col overflow-hidden shadow-inner">
    {#if appState.videos.length === 0}
      <!-- Empty State -->
      <div class="flex-1 flex flex-col items-center justify-center p-8 text-center select-none">
        <div class="w-16 h-16 rounded-2xl bg-zinc-800/60 border border-zinc-700/50 flex items-center justify-center text-3xl mb-3 shadow-lg">
          📺
        </div>
        <h3 class="text-sm font-semibold text-zinc-200">Fila de Vídeos Vazia</h3>
        <p class="text-xs text-zinc-400 max-w-sm mt-1">
          Nenhum vídeo na fila. Cole o link do YouTube acima e clique em Adicionar para iniciar a busca por momentos virais.
        </p>
      </div>
    {:else}
      <!-- Header row -->
      <div class="grid grid-cols-12 gap-3 px-4 py-2.5 bg-zinc-900/90 border-b border-zinc-800 text-[11px] font-semibold text-zinc-400 select-none">
        <div class="col-span-6">Título / URL</div>
        <div class="col-span-2">Canal</div>
        <div class="col-span-2">Duração</div>
        <div class="col-span-2 text-center">Status</div>
      </div>

      <!-- Scrollable video rows -->
      <div class="flex-1 overflow-y-auto divide-y divide-zinc-800/60">
        {#each appState.videos as video (video.id)}
          <div class="grid grid-cols-12 gap-3 px-4 py-3 items-center hover:bg-zinc-800/40 transition-colors text-xs">
            <div class="col-span-6 flex items-center space-x-2.5 min-w-0">
              <div class="w-7 h-7 rounded-lg bg-indigo-950/60 border border-indigo-800/50 flex items-center justify-center text-indigo-400 shrink-0">
                <Tv class="w-3.5 h-3.5" />
              </div>
              <div class="min-w-0">
                <p class="font-medium text-zinc-100 truncate" title={video.title}>
                  {video.title || video.url}
                </p>
                <p class="text-[10px] text-zinc-500 font-mono truncate">{video.url}</p>
              </div>
            </div>

            <div class="col-span-2 text-zinc-300 truncate" title={video.author}>
              {video.author || '-'}
            </div>

            <div class="col-span-2 text-zinc-400 flex items-center space-x-1">
              <Clock class="w-3 h-3 text-zinc-500" />
              <span>{formatDuration(video.durationSeconds)}</span>
            </div>

            <div class="col-span-2 flex justify-center">
              <span
                class="px-2.5 py-1 rounded-full text-[11px] font-semibold border inline-flex items-center space-x-1 {getStatusBadgeClass(video.status)}"
              >
                {#if video.status === 'COMPLETED'}
                  <CheckCircle2 class="w-3 h-3" />
                {:else if video.status === 'FETCHING_TRANSCRIPT' || video.status === 'DOWNLOADING_VIDEO' || video.status === 'ANALYZING_AI' || video.status === 'CUTTING_FFMPEG'}
                  <Loader2 class="w-3 h-3 animate-spin" />
                {:else if video.status === 'FAILED'}
                  <AlertCircle class="w-3 h-3" />
                {/if}
                <span>{ProcessingStatusLabels[video.status] || video.status}</span>
              </span>
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </div>

  <!-- Bottom: Quick Actions Bar -->
  <div class="bg-zinc-900/80 p-3.5 rounded-xl border border-zinc-800/80 flex items-center justify-between shadow-md text-xs select-none">
    <div class="flex items-center space-x-5">
      <!-- Dynamic Subtitles Checkbox -->
      <label class="flex items-center space-x-2 cursor-pointer text-zinc-200 hover:text-white">
        <input
          type="checkbox"
          bind:checked={appState.queueOptions.burnSubtitles}
          class="rounded bg-zinc-950 border-zinc-700 text-indigo-600 focus:ring-indigo-500 focus:ring-offset-zinc-900 cursor-pointer"
        />
        <span class="flex items-center space-x-1">
          <Captions class="w-3.5 h-3.5 text-indigo-400" />
          <span class="font-medium">Legendas Dinâmicas (.ass)</span>
        </span>
      </label>

      <!-- Auto-process Above Threshold Checkbox -->
      <label class="flex items-center space-x-2 cursor-pointer text-zinc-200 hover:text-white">
        <input
          type="checkbox"
          bind:checked={appState.queueOptions.autoProcessAboveThreshold}
          class="rounded bg-zinc-950 border-zinc-700 text-indigo-600 focus:ring-indigo-500 focus:ring-offset-zinc-900 cursor-pointer"
        />
        <span class="flex items-center space-x-1">
          <Sparkles class="w-3.5 h-3.5 text-amber-400" />
          <span class="font-medium">Processar Automaticamente (Score &gt;= 90)</span>
        </span>
      </label>

      <div class="h-4 w-px bg-zinc-800"></div>

      <!-- Orientation Mode Combo -->
      <div class="flex items-center space-x-2">
        <span class="text-zinc-400">Orientação:</span>
        <select
          bind:value={appState.queueOptions.defaultOrientation}
          class="bg-zinc-950 border border-zinc-700/80 rounded-lg px-2.5 py-1 text-xs text-white focus:outline-none focus:ring-1 focus:ring-indigo-500"
        >
          <option value="AUTO">Automático (Detectar Duração)</option>
          <option value="VERTICAL_9_16">Vertical 9:16 (Shorts/Reels)</option>
          <option value="HORIZONTAL_16_9">Horizontal 16:9 (YouTube Tradicional)</option>
        </select>
      </div>
    </div>

    <div class="flex items-center space-x-3">
      <button
        onclick={() => appState.clearQueue()}
        disabled={appState.videos.length === 0}
        class="flex items-center space-x-1 text-zinc-400 hover:text-zinc-200 disabled:opacity-40 disabled:hover:text-zinc-400 px-3 py-1.5 rounded-lg border border-zinc-800 hover:border-zinc-700 transition cursor-pointer"
      >
        <Trash2 class="w-3.5 h-3.5" />
        <span>Limpar Fila</span>
      </button>

      <button
        onclick={() => appState.startPipeline()}
        disabled={appState.isProcessing || appState.videos.length === 0}
        class="flex items-center space-x-2 bg-emerald-600 hover:bg-emerald-500 active:bg-emerald-700 disabled:opacity-40 text-white px-5 py-2 rounded-lg font-bold shadow-lg shadow-emerald-600/20 transition-all cursor-pointer"
      >
        <Play class="w-4 h-4 fill-current" />
        <span>▶ Iniciar Processamento</span>
      </button>
    </div>
  </div>
</div>
