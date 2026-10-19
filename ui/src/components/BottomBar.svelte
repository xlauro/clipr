<script lang="ts">
  import { appState } from '../lib/stores/appState.svelte';
  import type { LogCategory } from '../types';
  import {
    Terminal,
    ChevronUp,
    ChevronDown,
    Trash2,
    Bug,
    ClipboardCopy,
  } from 'lucide-svelte';

  const logCategories: { id: LogCategory; label: string }[] = [
    { id: 'ALL', label: 'Todos' },
    { id: 'PIPELINE', label: 'Pipeline' },
    { id: 'YTDLP', label: 'yt-dlp' },
    { id: 'AI', label: 'IA' },
    { id: 'FFMPEG', label: 'FFmpeg' },
    { id: 'THUMBNAIL', label: 'Thumb' },
    { id: 'DB', label: 'DB' },
  ];

  let logContainer: HTMLDivElement | null = $state(null);

  function toggleLogDrawer() {
    appState.isLogDrawerOpen = !appState.isLogDrawerOpen;
  }

  // Auto-scroll smooth to bottom when new logs arrive or category changes
  $effect(() => {
    if (appState.isLogDrawerOpen && logContainer) {
      const _len = appState.filteredLogs.length;
      const _cat = appState.selectedLogCategory;
      requestAnimationFrame(() => {
        if (logContainer) {
          logContainer.scrollTo({
            top: logContainer.scrollHeight,
            behavior: 'smooth',
          });
        }
      });
    }
  });

  interface MessageToken {
    isTag: boolean;
    text: string;
    tagClass?: string;
  }

  const TAG_REGEX = /(\[(?:PIPELINE|YTDLP|AI(?:-[A-Z0-9_-]+)?|FFMPEG|THUMBNAIL|DB)\])/;

  function parseMessageTokens(message: string): MessageToken[] {
    const parts = message.split(TAG_REGEX);
    const tokens: MessageToken[] = [];
    for (let i = 0; i < parts.length; i++) {
      const part = parts[i];
      if (!part) continue;
      const match = part.match(/^\[(PIPELINE|YTDLP|AI(?:-[A-Z0-9_-]+)?|FFMPEG|THUMBNAIL|DB)\]$/);
      if (match) {
        const rawTag = match[1];
        let tagClass = 'bg-zinc-800 text-zinc-300 border-zinc-700';
        if (rawTag === 'PIPELINE') {
          tagClass = 'bg-emerald-950/70 text-emerald-300 border-emerald-800/60';
        } else if (rawTag === 'YTDLP') {
          tagClass = 'bg-cyan-950/70 text-cyan-300 border-cyan-800/60';
        } else if (rawTag.startsWith('AI')) {
          tagClass = 'bg-violet-950/70 text-violet-300 border-violet-800/60';
        } else if (rawTag === 'FFMPEG') {
          tagClass = 'bg-amber-950/70 text-amber-300 border-amber-800/60';
        } else if (rawTag === 'THUMBNAIL') {
          tagClass = 'bg-pink-950/70 text-pink-300 border-pink-800/60';
        } else if (rawTag === 'DB') {
          tagClass = 'bg-blue-950/70 text-blue-300 border-blue-800/60';
        }
        tokens.push({ isTag: true, text: part, tagClass });
      } else {
        let text = part;
        if (tokens.length > 0 && tokens[tokens.length - 1].isTag && text.startsWith(' ')) {
          text = text.substring(1);
        }
        tokens.push({ isTag: false, text });
      }
    }
    return tokens;
  }
</script>

<footer class="bg-zinc-900/95 border-t border-zinc-800/80 flex flex-col shrink-0 select-none shadow-2xl z-20">
  <!-- Status & Progress Summary Row -->
  <div class="px-4 py-2 flex items-center justify-between text-xs space-x-4">
    <!-- Live Status Label -->
    <div class="flex items-center space-x-2 min-w-0 flex-1">
      <span class="w-2 h-2 rounded-full {appState.isProcessing ? 'bg-amber-400 animate-ping' : 'bg-emerald-400'}"></span>
      <span class="font-semibold text-zinc-200 truncate" title={appState.statusLabel}>
        {appState.statusLabel}
      </span>
    </div>

    <!-- Dynamic Progress Bar -->
    <div class="flex items-center space-x-2.5 w-72">
      <span class="text-[11px] text-zinc-400 shrink-0 font-medium">Progresso:</span>
      <div class="flex-1 h-2 bg-zinc-800 rounded-full overflow-hidden border border-zinc-700/60 relative">
        <div
          class="h-full bg-gradient-to-r from-indigo-500 via-indigo-400 to-violet-500 transition-all duration-300 rounded-full"
          style="width: {appState.progressPercent}%"
        ></div>
      </div>
      <span class="text-[11px] font-mono font-bold text-zinc-300 w-9 text-right shrink-0">
        {appState.progressPercent}%
      </span>
    </div>

    <!-- Toggle Console Button -->
    <button
      onclick={toggleLogDrawer}
      class="flex items-center space-x-1.5 px-3 py-1 rounded-lg border text-xs font-medium transition cursor-pointer {appState.isLogDrawerOpen
        ? 'bg-zinc-800 text-white border-zinc-600'
        : 'bg-zinc-950/70 text-zinc-400 border-zinc-800 hover:text-zinc-200 hover:border-zinc-700'}"
    >
      <Terminal class="w-3.5 h-3.5" />
      <span>📋 Console de Logs</span>
      {#if appState.isLogDrawerOpen}
        <ChevronDown class="w-3.5 h-3.5" />
      {:else}
        <ChevronUp class="w-3.5 h-3.5" />
      {/if}
    </button>
  </div>

  <!-- Collapsible Log Drawer -->
  {#if appState.isLogDrawerOpen}
    <div class="border-t border-zinc-800/80 bg-zinc-950 p-3 flex flex-col h-52 transition-all duration-200">
      <!-- Drawer Header Bar -->
      <div class="flex flex-wrap items-center justify-between gap-2 pb-2 mb-2 border-b border-zinc-900 text-xs text-zinc-400">
        <!-- Left: Console Label & Category Pills -->
        <div class="flex items-center space-x-3 overflow-x-auto min-w-0">
          <div class="flex items-center space-x-1.5 shrink-0">
            <Terminal class="w-3.5 h-3.5 text-indigo-400" />
            <span class="font-bold text-zinc-300">Console:</span>
          </div>

          <!-- Category Pills -->
          <div class="flex items-center space-x-1 shrink-0">
            {#each logCategories as cat}
              {@const count = appState.logCategoryCounts[cat.id]}
              <button
                onclick={() => (appState.selectedLogCategory = cat.id)}
                class="px-2 py-0.5 rounded-md text-[11px] font-medium transition cursor-pointer flex items-center space-x-1 {appState.selectedLogCategory === cat.id
                  ? 'bg-indigo-600 text-white font-semibold shadow-xs'
                  : 'bg-zinc-900 text-zinc-400 hover:text-zinc-200 hover:bg-zinc-800 border border-zinc-800/80'}"
              >
                <span>{cat.label}</span>
                <span class="text-[10px] opacity-80 font-mono">({count})</span>
              </button>
            {/each}
          </div>
        </div>

        <!-- Right: Actions & Controls -->
        <div class="flex items-center space-x-3 shrink-0">
          <!-- Debug Mode Checkbox -->
          <label class="flex items-center space-x-1.5 cursor-pointer hover:text-zinc-200">
            <input
              type="checkbox"
              bind:checked={appState.isDebugMode}
              class="rounded bg-zinc-900 border-zinc-700 text-indigo-600 focus:ring-indigo-500 cursor-pointer"
            />
            <span class="text-[11px] font-medium flex items-center space-x-1">
              <Bug class="w-3 h-3 text-amber-400" />
              <span>Debug</span>
            </span>
          </label>

          <!-- Copy Logs Button -->
          <button
            onclick={() => appState.copyLogsToClipboard()}
            title="Copiar logs visíveis para a área de transferência"
            class="flex items-center space-x-1 text-zinc-300 hover:text-white text-[11px] font-medium px-2 py-0.5 rounded border border-zinc-800 hover:border-zinc-700 hover:bg-zinc-900 transition cursor-pointer"
          >
            <ClipboardCopy class="w-3 h-3 text-indigo-400" />
            <span>Copiar Logs</span>
          </button>

          <!-- Clear Console -->
          <button
            onclick={() => appState.clearLogs()}
            title="Limpar mensagens do console"
            class="flex items-center space-x-1 text-zinc-400 hover:text-zinc-200 text-[11px] font-medium px-2 py-0.5 rounded border border-zinc-800 hover:border-zinc-700 hover:bg-zinc-900 transition cursor-pointer"
          >
            <Trash2 class="w-3 h-3" />
            <span>Limpar</span>
          </button>
        </div>
      </div>

      <!-- Log Entries List -->
      <div
        bind:this={logContainer}
        class="flex-1 overflow-y-auto space-y-1 font-mono text-[11px] select-text scroll-smooth"
      >
        {#if appState.filteredLogs.length === 0}
          <div class="text-zinc-600 italic p-3 text-center select-none">
            Nenhuma mensagem no console para a categoria selecionada.
          </div>
        {:else}
          {#each appState.filteredLogs as log (log.id)}
            <div class="flex items-start space-x-2 text-zinc-300 leading-tight py-0.5">
              <span class="text-zinc-500 shrink-0 select-none">[{log.timestamp}]</span>
              <span
                class="px-1.5 py-0.2 rounded text-[10px] font-bold shrink-0 select-none {log.level === 'ERROR'
                  ? 'bg-rose-950/80 text-rose-300 border border-rose-800/60'
                  : log.level === 'WARN'
                  ? 'bg-amber-950/80 text-amber-300 border border-amber-800/60'
                  : log.level === 'DEBUG'
                  ? 'bg-zinc-800 text-zinc-400 border border-zinc-700'
                  : 'bg-indigo-950/80 text-indigo-300 border border-indigo-800/60'}"
              >
                {log.level}
              </span>
              <span class="flex-1 break-all {log.level === 'ERROR' ? 'text-rose-300' : log.level === 'WARN' ? 'text-amber-200' : log.isDebug ? 'text-zinc-400' : 'text-zinc-200'}">
                {#each parseMessageTokens(log.message) as token}
                  {#if token.isTag}
                    <span class="inline-block px-1.5 py-0.2 rounded text-[10px] font-mono font-bold tracking-tight border {token.tagClass} mr-1 select-none">
                      {token.text}
                    </span>
                  {:else}
                    <span>{token.text}</span>
                  {/if}
                {/each}
              </span>
            </div>
          {/each}
        {/if}
      </div>
    </div>
  {/if}
</footer>
