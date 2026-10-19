<script lang="ts">
  import { appState } from '../lib/stores/appState.svelte';
  import {
    Scissors,
    Database,
    Cpu,
    Sparkles,
    Download,
    ListVideo,
    Film,
    Settings,
  } from 'lucide-svelte';
</script>

<header class="bg-zinc-900/90 border-b border-zinc-800/80 px-4 py-2.5 flex items-center justify-between shrink-0 select-none">
  <!-- Brand Title -->
  <div class="flex items-center space-x-3">
    <div class="w-8 h-8 rounded-lg overflow-hidden shadow-lg shadow-indigo-500/20 flex items-center justify-center border border-indigo-500/30">
      <img src="/clippr-icon.svg" alt="Clippr" class="w-full h-full object-cover" />
    </div>
    <div>
      <div class="flex items-center space-x-2">
        <h1 class="text-base font-bold tracking-tight text-white flex items-center gap-1.5">
          <span>Clippr</span>
          <span class="text-xs px-1.5 py-0.5 rounded font-mono font-medium bg-indigo-500/10 text-indigo-400 border border-indigo-500/20">
            v0.1 Rust
          </span>
        </h1>
      </div>
      <p class="text-[11px] text-zinc-400 leading-tight">
        AI &amp; FFmpeg Video Clipper
      </p>
    </div>

    <!-- Tab Navigation Pills -->
    <nav class="ml-6 flex items-center space-x-1 bg-zinc-950/60 p-1 rounded-lg border border-zinc-800/70">
      <button
        onclick={() => (appState.activeTab = 'queue')}
        class="flex items-center space-x-2 px-3 py-1.5 rounded-md text-xs font-medium transition-all duration-150 {appState.activeTab === 'queue' ? 'bg-indigo-600 text-white shadow-sm' : 'text-zinc-400 hover:text-zinc-200 hover:bg-zinc-800/50'}"
      >
        <ListVideo class="w-3.5 h-3.5" />
        <span>📺 Fila de Vídeos</span>
        {#if appState.videos.length > 0}
          <span class="ml-1 text-[10px] px-1.5 py-0.2 rounded-full {appState.activeTab === 'queue' ? 'bg-indigo-700 text-white' : 'bg-zinc-800 text-zinc-300'}">
            {appState.videos.length}
          </span>
        {/if}
      </button>

      <button
        onclick={() => (appState.activeTab = 'clips')}
        class="flex items-center space-x-2 px-3 py-1.5 rounded-md text-xs font-medium transition-all duration-150 {appState.activeTab === 'clips' ? 'bg-indigo-600 text-white shadow-sm' : 'text-zinc-400 hover:text-zinc-200 hover:bg-zinc-800/50'}"
      >
        <Film class="w-3.5 h-3.5" />
        <span>🎬 Cortes &amp; Momentos Virais</span>
        {#if appState.clips.length > 0}
          <span class="ml-1 text-[10px] px-1.5 py-0.2 rounded-full {appState.activeTab === 'clips' ? 'bg-indigo-700 text-white' : 'bg-zinc-800 text-zinc-300'}">
            {appState.clips.length}
          </span>
        {/if}
      </button>

      <button
        onclick={() => (appState.activeTab = 'settings')}
        class="flex items-center space-x-2 px-3 py-1.5 rounded-md text-xs font-medium transition-all duration-150 {appState.activeTab === 'settings' ? 'bg-indigo-600 text-white shadow-sm' : 'text-zinc-400 hover:text-zinc-200 hover:bg-zinc-800/50'}"
      >
        <Settings class="w-3.5 h-3.5" />
        <span>⚙ Configurações &amp; Ambiente</span>
      </button>
    </nav>
  </div>

  <!-- Real-time Status Badges -->
  <div class="flex items-center space-x-2.5">
    <!-- SQLite Badge -->
    <div
      class="flex items-center space-x-1.5 text-xs px-2.5 py-1 rounded-full border transition-all duration-200 {appState.diagnostics.sqliteConnected
        ? 'bg-emerald-950/40 text-emerald-300 border-emerald-800/50'
        : 'bg-rose-950/40 text-rose-300 border-rose-800/50'}"
      title={appState.diagnostics.sqliteMessage}
    >
      <span class="w-2 h-2 rounded-full {appState.diagnostics.sqliteConnected ? 'bg-emerald-400 animate-pulse' : 'bg-rose-400'}"></span>
      <Database class="w-3 h-3 text-emerald-400" />
      <span class="font-medium text-[11px]">SQLite WAL: {appState.diagnostics.sqliteConnected ? 'Conectado' : 'Offline'}</span>
    </div>

    <!-- FFmpeg Badge -->
    <div
      class="flex items-center space-x-1.5 text-xs px-2.5 py-1 rounded-full border transition-all duration-200 {appState.diagnostics.ffmpegOk
        ? 'bg-emerald-950/40 text-emerald-300 border-emerald-800/50'
        : 'bg-amber-950/40 text-amber-300 border-amber-800/50'}"
      title={appState.diagnostics.ffmpegDetails}
    >
      <span class="w-2 h-2 rounded-full {appState.diagnostics.ffmpegOk ? 'bg-emerald-400' : 'bg-amber-400'}"></span>
      <Cpu class="w-3 h-3 text-emerald-400" />
      <span class="font-medium text-[11px]">FFmpeg: {appState.diagnostics.ffmpegEncoder || 'OK'}</span>
    </div>

    <!-- AI Gemini / OpenAI Badge -->
    <div
      class="flex items-center space-x-1.5 text-xs px-2.5 py-1 rounded-full border transition-all duration-200 {appState.settings.aiProvider === 'GEMINI'
        ? (appState.diagnostics.geminiOk ? 'bg-emerald-950/40 text-emerald-300 border-emerald-800/50' : 'bg-amber-950/40 text-amber-300 border-amber-800/50')
        : (appState.diagnostics.openAiOk ? 'bg-emerald-950/40 text-emerald-300 border-emerald-800/50' : 'bg-amber-950/40 text-amber-300 border-amber-800/50')}"
      title={appState.settings.aiProvider === 'GEMINI' ? appState.diagnostics.geminiMessage : appState.diagnostics.openAiMessage}
    >
      <span class="w-2 h-2 rounded-full {(appState.settings.aiProvider === 'GEMINI' ? appState.diagnostics.geminiOk : appState.diagnostics.openAiOk) ? 'bg-emerald-400' : 'bg-amber-400'}"></span>
      <Sparkles class="w-3 h-3 text-amber-400" />
      <span class="font-medium text-[11px]">
        {appState.settings.aiProvider === 'GEMINI' ? 'Gemini 2.5' : 'OpenAI'}:
        {(appState.settings.aiProvider === 'GEMINI' ? appState.diagnostics.geminiOk : appState.diagnostics.openAiOk) ? 'Ativo' : 'Sem Chave'}
      </span>
    </div>

    <!-- yt-dlp Badge -->
    <div
      class="flex items-center space-x-1.5 text-xs px-2.5 py-1 rounded-full border transition-all duration-200 {appState.diagnostics.ytDlpOk
        ? 'bg-emerald-950/40 text-emerald-300 border-emerald-800/50'
        : 'bg-rose-950/40 text-rose-300 border-rose-800/50'}"
      title="yt-dlp versão: {appState.diagnostics.ytDlpVersion}"
    >
      <span class="w-2 h-2 rounded-full {appState.diagnostics.ytDlpOk ? 'bg-emerald-400' : 'bg-rose-400'}"></span>
      <Download class="w-3 h-3 text-emerald-400" />
      <span class="font-medium text-[11px]">yt-dlp: {appState.diagnostics.ytDlpOk ? 'OK' : 'Ausente'}</span>
    </div>
  </div>
</header>
