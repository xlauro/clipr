<script lang="ts">
  import { appState } from '../../../lib/stores/appState.svelte';
  import {
    Image as ImageIcon,
    FolderOpen,
    Sparkles,
    SlidersHorizontal,
    Clock,
    Camera,
  } from 'lucide-svelte';

  function adjustThumb(delta: number) {
    const clip = appState.selectedClip;
    if (!clip) return;
    const current = appState.thumbnailTimestamp;
    const clamped = Math.max(clip.startSeconds, Math.min(clip.endSeconds, current + delta));
    appState.thumbnailTimestamp = Math.round(clamped * 10) / 10;
  }

  function setFromPlayer() {
    appState.thumbnailTimestamp = Math.round(appState.currentTime * 10) / 10;
    appState.addToast('info', 'Frame da Capa', `Timestamp ajustado para ${appState.thumbnailTimestamp}s.`);
  }
</script>

<div class="h-full flex flex-col p-4 space-y-4 overflow-y-auto">
  <div class="bg-zinc-900/80 p-4 rounded-xl border border-zinc-800/80 space-y-4 shadow-sm select-none">
    <div class="flex items-center justify-between">
      <h4 class="text-xs font-bold text-zinc-200 flex items-center space-x-1.5">
        <ImageIcon class="w-4 h-4 text-indigo-400" />
        <span>🖼 Capa / Thumbnail do Corte (Multi-proporção)</span>
      </h4>
      <span class="text-[10px] text-zinc-400">Geração por FFmpeg + Headline Magnética</span>
    </div>

    <div class="flex items-start space-x-4">
      <!-- Thumbnail Preview Box -->
      <div
        class="bg-black/90 border border-zinc-800 rounded-xl overflow-hidden flex flex-col items-center justify-center relative shadow-inner shrink-0 {appState.thumbnailAspectRatio === '9:16'
          ? 'w-36 h-60'
          : 'w-64 h-36'}"
      >
        {#if appState.thumbnailPreview}
          <div class="w-full h-full flex flex-col justify-between p-2 relative bg-zinc-900 text-center">
            <span class="text-[9px] bg-indigo-600/80 text-white font-bold px-1.5 py-0.5 rounded self-start">
              {appState.thumbnailAspectRatio}
            </span>
            <div class="my-auto px-1">
              <span class="text-xs font-black uppercase text-amber-300 tracking-tight leading-none drop-shadow-md">
                {appState.thumbnailHeadline || 'HEADLINE'}
              </span>
            </div>
            <span class="text-[9px] text-zinc-400 self-end">
              {appState.thumbnailTimestamp.toFixed(1)}s
            </span>
          </div>
        {:else}
          <div class="text-center p-3 text-zinc-500">
            <Camera class="w-8 h-8 mx-auto mb-1 opacity-40 text-zinc-400" />
            <p class="text-[11px] font-medium text-zinc-400">Nenhuma capa gerada</p>
            <p class="text-[10px] text-zinc-500 mt-0.5">Clique em "Gerar Capa"</p>
          </div>
        {/if}
      </div>

      <!-- Controls Column -->
      <div class="flex-1 space-y-3">
        <!-- Aspect Ratio Selection -->
        <div class="flex items-center space-x-2 text-xs">
          <span class="text-zinc-400 font-medium">Proporção:</span>
          <select
            bind:value={appState.thumbnailAspectRatio}
            class="bg-zinc-950 border border-zinc-700/80 rounded-lg px-2.5 py-1 text-xs text-white focus:outline-none focus:ring-1 focus:ring-indigo-500"
          >
            <option value="9:16">Vertical 9:16 (Shorts / Reels / TikTok)</option>
            <option value="16:9">Horizontal 16:9 (YouTube Widescreen)</option>
          </select>
        </div>

        <!-- Headline Input -->
        <div class="space-y-1">
          <label for="txtHeadline" class="text-[11px] font-medium text-zinc-400">
            Texto / Headline da Capa (Alto CTR):
          </label>
          <input
            id="txtHeadline"
            type="text"
            bind:value={appState.thumbnailHeadline}
            placeholder="Texto magnético de alto CTR da capa"
            class="w-full bg-zinc-950 border border-zinc-700/80 rounded-lg px-3 py-1.5 text-xs text-white placeholder-zinc-500 focus:outline-none focus:ring-1 focus:ring-indigo-500 font-semibold"
          />
        </div>

        <!-- Frame Timestamp Controls -->
        <div class="flex items-center space-x-2 text-xs">
          <span class="text-zinc-400 font-medium flex items-center space-x-1">
            <Clock class="w-3.5 h-3.5 text-zinc-500" />
            <span>Frame:</span>
          </span>
          <span class="font-mono font-bold text-amber-400 w-14">
            {appState.thumbnailTimestamp.toFixed(1)}s
          </span>

          <div class="flex items-center space-x-1">
            <button
              onclick={() => adjustThumb(-1)}
              class="px-2 py-1 bg-zinc-800 hover:bg-zinc-700 text-zinc-200 rounded text-xs cursor-pointer"
            >-1s</button>
            <button
              onclick={() => adjustThumb(1)}
              class="px-2 py-1 bg-zinc-800 hover:bg-zinc-700 text-zinc-200 rounded text-xs cursor-pointer"
            >+1s</button>
          </div>

          <button
            onclick={setFromPlayer}
            class="px-2.5 py-1 bg-zinc-800 hover:bg-zinc-700 text-zinc-200 rounded text-xs font-medium cursor-pointer"
          >
            📍 Usar Tempo Atual do Player
          </button>
        </div>

        <!-- Action Buttons -->
        <div class="flex items-center space-x-2 pt-1">
          <button
            onclick={() => appState.generateThumbnail()}
            class="flex items-center space-x-1.5 bg-indigo-600 hover:bg-indigo-500 text-white px-3.5 py-1.5 rounded-lg text-xs font-semibold shadow-md shadow-indigo-600/20 cursor-pointer"
          >
            <Sparkles class="w-3.5 h-3.5" />
            <span>🎨 Gerar / Regenerar Capa</span>
          </button>

          <button
            onclick={() => appState.openPath(appState.thumbnailPreview)}
            disabled={!appState.thumbnailPreview}
            class="flex items-center space-x-1 bg-zinc-800 hover:bg-zinc-700 disabled:opacity-40 text-zinc-200 px-3 py-1.5 rounded-lg text-xs font-medium cursor-pointer"
          >
            <FolderOpen class="w-3.5 h-3.5" />
            <span>📂 Abrir Capa</span>
          </button>

          <button
            onclick={() => appState.openPath(appState.rawFramePreview)}
            disabled={!appState.rawFramePreview}
            class="flex items-center space-x-1 bg-zinc-800 hover:bg-zinc-700 disabled:opacity-40 text-zinc-200 px-3 py-1.5 rounded-lg text-xs font-medium cursor-pointer"
          >
            <Camera class="w-3.5 h-3.5" />
            <span>🖼 Frame Limpo</span>
          </button>
        </div>
      </div>
    </div>
  </div>
</div>
