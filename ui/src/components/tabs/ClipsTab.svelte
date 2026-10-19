<script lang="ts">
  import { appState } from '../../lib/stores/appState.svelte';
  import { EmotionTagDetails, ProcessingStatusLabels } from '../../types';
  import PlayerSubTab from './clips/PlayerSubTab.svelte';
  import FramingSubTab from './clips/FramingSubTab.svelte';
  import ThumbnailSubTab from './clips/ThumbnailSubTab.svelte';
  import SubtitlesSubTab from './clips/SubtitlesSubTab.svelte';
  import {
    Play,
    Zap,
    Share2,
    Film,
    Filter,
    Clock,
    Sparkles,
    Smartphone,
    Layers,
    Image,
    MessageSquare,
    CheckCircle2,
    AlertCircle,
    Loader2,
  } from 'lucide-svelte';

  function formatTime(seconds: number): string {
    const total = Math.max(0, Math.floor(seconds || 0));
    const m = Math.floor(total / 60);
    const s = total % 60;
    return `${m.toString().padStart(2, '0')}:${s.toString().padStart(2, '0')}`;
  }

  function formatDuration(start: number, end: number): string {
    const diff = Math.max(0, Math.round(end - start));
    const m = Math.floor(diff / 60);
    const s = diff % 60;
    return m > 0 ? `${m}m ${s}s` : `${s}s`;
  }

  function getScoreBadgeClass(score: number): string {
    if (score >= 90) {
      return 'bg-gradient-to-r from-amber-500/25 to-orange-500/25 text-amber-300 border-amber-500/60 shadow-sm shadow-amber-500/20';
    } else if (score >= 80) {
      return 'bg-amber-500/15 text-amber-300 border-amber-500/40';
    } else {
      return 'bg-zinc-800 text-zinc-400 border-zinc-700';
    }
  }
</script>

<div class="h-full flex overflow-hidden">
  <!-- Left Panel: Clip List & Actions (52% width) -->
  <div class="w-[52%] border-r border-zinc-800/80 flex flex-col bg-zinc-950/40 overflow-hidden">
    <!-- Top Filter & Batch Action Bar -->
    <div class="p-3 border-b border-zinc-800/80 space-y-2.5 bg-zinc-900/60 shrink-0 select-none">
      <div class="flex items-center justify-between">
        <div class="flex items-center space-x-2">
          <Film class="w-4 h-4 text-indigo-400" />
          <h3 class="text-xs font-bold text-zinc-100">Momentos Virais Detectados:</h3>
        </div>

        <!-- Filter dropdown -->
        <div class="flex items-center space-x-1.5 text-xs">
          <Filter class="w-3.5 h-3.5 text-zinc-400" />
          <select
            bind:value={appState.clipFilter}
            class="bg-zinc-950 border border-zinc-700/80 rounded-lg px-2 py-1 text-xs text-white focus:outline-none focus:ring-1 focus:ring-indigo-500"
          >
            <option value="ALL">Todos os Cortes</option>
            <option value="VIRAL_90">🔥 Alta Virilidade (&gt;= 90)</option>
            <option value="SHORT">⚡ Curtos (&lt;= 90s)</option>
            <option value="LONG">🎬 Longos (&gt; 90s)</option>
            <option value="RENDERED">✅ Renderizados</option>
            <option value="PENDING">⏳ Pendentes</option>
          </select>
        </div>
      </div>

      <!-- Batch Action Buttons -->
      <div class="flex items-center space-x-2 pt-0.5">
        <button
          onclick={() => appState.renderSelectedClip()}
          disabled={!appState.selectedClip}
          class="flex items-center space-x-1.5 bg-indigo-600 hover:bg-indigo-500 disabled:opacity-40 text-white px-3 py-1.5 rounded-lg text-xs font-semibold shadow-sm cursor-pointer"
        >
          <Play class="w-3.5 h-3.5 fill-current" />
          <span>▶ Renderizar Selecionado</span>
        </button>

        <button
          onclick={() => appState.renderAllClips()}
          disabled={appState.clips.length === 0}
          class="flex items-center space-x-1.5 bg-emerald-600 hover:bg-emerald-500 disabled:opacity-40 text-white px-3 py-1.5 rounded-lg text-xs font-bold shadow-sm cursor-pointer"
        >
          <Zap class="w-3.5 h-3.5 fill-current" />
          <span>⚡ Renderizar Todos (100% CPU)</span>
        </button>

        <div class="flex-1"></div>

        <button
          onclick={() => appState.publishSelectedClip()}
          disabled={!appState.selectedClip}
          class="flex items-center space-x-1.5 bg-rose-600/90 hover:bg-rose-500 disabled:opacity-40 text-white px-3 py-1.5 rounded-lg text-xs font-semibold shadow-sm cursor-pointer"
        >
          <Share2 class="w-3.5 h-3.5" />
          <span>📤 Publicar no YouTube</span>
        </button>
      </div>
    </div>

    <!-- Clips Cards List -->
    <div class="flex-1 overflow-y-auto p-3 space-y-2.5">
      {#if appState.filteredClips.length === 0}
        <div class="h-full flex flex-col items-center justify-center p-8 text-center select-none">
          <div class="w-14 h-14 rounded-2xl bg-zinc-800/60 border border-zinc-700/50 flex items-center justify-center text-3xl mb-3 shadow-lg">
            🎬
          </div>
          <h4 class="text-xs font-semibold text-zinc-200">Nenhum Momento Viral Encontrado</h4>
          <p class="text-[11px] text-zinc-400 mt-1 max-w-xs">
            Nenhum momento viral corresponde ao filtro selecionado ou ainda não foi processado.
          </p>
        </div>
      {:else}
        {#each appState.filteredClips as clip (clip.id)}
          {@const emotion = EmotionTagDetails[clip.emotionTag || 'ALTA_ENERGIA']}
          <div
            role="button"
            tabindex="0"
            onclick={() => appState.selectClip(clip.id)}
            onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') appState.selectClip(clip.id); }}
            class="p-3 rounded-xl border text-left transition-all cursor-pointer relative shadow-sm select-none {appState.selectedClipId === clip.id
              ? 'bg-zinc-800/90 border-indigo-500 ring-1 ring-indigo-500/50'
              : 'bg-zinc-900/60 border-zinc-800/80 hover:bg-zinc-800/50 hover:border-zinc-700'}"
          >
            <!-- Card Header: Score, Emotion, Type, Status -->
            <div class="flex items-center justify-between mb-2">
              <div class="flex items-center space-x-2">
                <!-- Virality Score Badge -->
                <span
                  class="px-2 py-0.5 rounded-md text-xs font-black border flex items-center space-x-1 {getScoreBadgeClass(clip.viralityScore)}"
                >
                  <Sparkles class="w-3 h-3" />
                  <span>{clip.viralityScore} ⭐</span>
                </span>

                <!-- Emotion Tag Badge -->
                <span
                  class="px-2 py-0.5 rounded-md text-[11px] font-semibold border flex items-center space-x-1 bg-zinc-800/70 text-zinc-200 border-zinc-700"
                >
                  <span>{emotion.emoji}</span>
                  <span>{emotion.label}</span>
                </span>

                <!-- Short / Long badge -->
                <span class="text-[10px] font-medium text-zinc-400 px-1.5 py-0.5 rounded bg-zinc-800">
                  {clip.endSeconds - clip.startSeconds > 90 ? '🎬 Longo' : '⚡ Curto'}
                </span>
              </div>

              <!-- Status Badge -->
              <div class="flex items-center space-x-1">
                {#if clip.status === 'COMPLETED' || clip.outputFilePath}
                  <span class="flex items-center space-x-1 text-[11px] font-semibold text-emerald-400 bg-emerald-950/40 border border-emerald-800/50 px-2 py-0.5 rounded-full">
                    <CheckCircle2 class="w-3 h-3" />
                    <span>Pronto</span>
                  </span>
                {:else if clip.status === 'CUTTING_FFMPEG'}
                  <span class="flex items-center space-x-1 text-[11px] font-semibold text-amber-400 bg-amber-950/40 border border-amber-800/50 px-2 py-0.5 rounded-full animate-pulse">
                    <Loader2 class="w-3 h-3 animate-spin" />
                    <span>Renderizando</span>
                  </span>
                {:else}
                  <span class="text-[11px] font-medium text-zinc-400 bg-zinc-800/80 border border-zinc-700/60 px-2 py-0.5 rounded-full">
                    Pendente
                  </span>
                {/if}
              </div>
            </div>

            <!-- Clip Title & Hook -->
            <h4 class="text-xs font-bold text-zinc-100 line-clamp-1">{clip.title}</h4>
            <p class="text-[11px] text-zinc-400 mt-0.5 italic line-clamp-1">
              "{clip.hook || 'Gancho magnético...'}"
            </p>

            <!-- Card Footer: Time interval, orientation, subtitles -->
            <div class="flex items-center justify-between text-[11px] text-zinc-400 mt-2.5 pt-2 border-t border-zinc-800/60">
              <div class="flex items-center space-x-1 text-zinc-300 font-mono">
                <Clock class="w-3 h-3 text-zinc-500" />
                <span>{formatTime(clip.startSeconds)} - {formatTime(clip.endSeconds)} ({formatDuration(clip.startSeconds, clip.endSeconds)})</span>
              </div>

              <div class="flex items-center space-x-1.5">
                <span class="px-1.5 py-0.5 rounded bg-zinc-800/80 text-[10px] text-zinc-300 border border-zinc-700/50">
                  {clip.framingMode === 'HORIZONTAL_16_9' ? '16:9' : '9:16'}
                </span>
                {#if clip.hasSubtitles}
                  <span class="px-1.5 py-0.5 rounded bg-amber-500/10 text-[10px] text-amber-300 border border-amber-500/20">
                    Legendas
                  </span>
                {/if}
              </div>
            </div>
          </div>
        {/each}
      {/if}
    </div>
  </div>

  <!-- Right Panel: Editor Sub-tabs (48% width) -->
  <div class="flex-1 flex flex-col bg-zinc-900/30 overflow-hidden">
    <!-- Sub-tab Navigation Header -->
    <div class="flex items-center space-x-1 p-2 bg-zinc-900/80 border-b border-zinc-800/80 shrink-0 select-none">
      <button
        onclick={() => (appState.activeClipSubTab = 'player')}
        class="flex items-center space-x-1.5 px-3 py-1.5 rounded-lg text-xs font-semibold transition cursor-pointer {appState.activeClipSubTab === 'player'
          ? 'bg-indigo-600 text-white shadow-sm'
          : 'text-zinc-400 hover:text-white hover:bg-zinc-800/50'}"
      >
        <Film class="w-3.5 h-3.5" />
        <span>🎥 Player &amp; Corte</span>
      </button>

      <button
        onclick={() => (appState.activeClipSubTab = 'framing')}
        class="flex items-center space-x-1.5 px-3 py-1.5 rounded-lg text-xs font-semibold transition cursor-pointer {appState.activeClipSubTab === 'framing'
          ? 'bg-indigo-600 text-white shadow-sm'
          : 'text-zinc-400 hover:text-white hover:bg-zinc-800/50'}"
      >
        <Layers class="w-3.5 h-3.5" />
        <span>🎨 Enquadramento &amp; Legendas</span>
      </button>

      <button
        onclick={() => (appState.activeClipSubTab = 'thumbnail')}
        class="flex items-center space-x-1.5 px-3 py-1.5 rounded-lg text-xs font-semibold transition cursor-pointer {appState.activeClipSubTab === 'thumbnail'
          ? 'bg-indigo-600 text-white shadow-sm'
          : 'text-zinc-400 hover:text-white hover:bg-zinc-800/50'}"
      >
        <Image class="w-3.5 h-3.5" />
        <span>🖼 Capa / Thumbnail</span>
      </button>

      <button
        onclick={() => (appState.activeClipSubTab = 'subtitles')}
        class="flex items-center space-x-1.5 px-3 py-1.5 rounded-lg text-xs font-semibold transition cursor-pointer {appState.activeClipSubTab === 'subtitles'
          ? 'bg-indigo-600 text-white shadow-sm'
          : 'text-zinc-400 hover:text-white hover:bg-zinc-800/50'}"
      >
        <MessageSquare class="w-3.5 h-3.5" />
        <span>💬 Legendas &amp; Transcrição</span>
      </button>
    </div>

    <!-- Active Sub-tab View -->
    <div class="flex-1 overflow-hidden">
      {#if appState.activeClipSubTab === 'player'}
        <PlayerSubTab />
      {:else if appState.activeClipSubTab === 'framing'}
        <FramingSubTab />
      {:else if appState.activeClipSubTab === 'thumbnail'}
        <ThumbnailSubTab />
      {:else if appState.activeClipSubTab === 'subtitles'}
        <SubtitlesSubTab />
      {/if}
    </div>
  </div>
</div>
