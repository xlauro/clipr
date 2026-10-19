<script lang="ts">
  import { appState } from '../../../lib/stores/appState.svelte';
  import {
    Play,
    Pause,
    Volume2,
    VolumeX,
    Smartphone,
    MapPin,
    Film,
  } from 'lucide-svelte';

  function formatTime(seconds: number): string {
    const clamped = Math.max(0, seconds || 0);
    const m = Math.floor(clamped / 60);
    const s = Math.floor(clamped % 60);
    const ms = Math.floor((clamped - Math.floor(clamped)) * 100);
    return `${m.toString().padStart(2, '0')}:${s.toString().padStart(2, '0')}.${ms.toString().padStart(2, '0')}`;
  }

  function togglePlayPause() {
    appState.isPlaying = !appState.isPlaying;
  }
</script>

<div class="h-full flex flex-col p-3.5 space-y-3 overflow-y-auto">
  <!-- Video Player Preview Container -->
  <div class="relative bg-black rounded-xl border border-zinc-800/90 overflow-hidden flex items-center justify-center min-h-[220px] max-h-[280px] shadow-lg">
    <!-- Center simulated screen / video -->
    <div class="w-full h-full flex flex-col items-center justify-center p-4 text-center select-none bg-gradient-to-b from-zinc-900 to-zinc-950">
      <Film class="w-10 h-10 text-indigo-400 mb-2 opacity-80" />
      <p class="text-xs font-semibold text-zinc-200">
        {appState.selectedClip?.title || 'Selecione um corte para reproduzir'}
      </p>
      <p class="text-[11px] text-zinc-400 mt-1 max-w-md">
        Intervalo ativo: {formatTime(appState.selectedClip?.startSeconds || 0)} até {formatTime(appState.selectedClip?.endSeconds || 0)}
      </p>

      <!-- Karaokê subtitle preview simulation -->
      {#if appState.selectedClip?.hasSubtitles}
        <div class="mt-4 px-3 py-1.5 rounded-lg bg-black/75 border border-zinc-700/60 shadow-xl max-w-sm">
          <span class="text-xs font-black tracking-wide" style="color: #FFFF00; text-shadow: 2px 2px 0 #000, -2px -2px 0 #000, 2px -2px 0 #000, -2px 2px 0 #000;">
            {appState.selectedClip.hook || 'LEGENDA DINÂMICA HORMOZI'}
          </span>
        </div>
      {/if}
    </div>

    <!-- 9:16 Crop Guide Safe Zone Overlay -->
    {#if appState.showCropGuide}
      <div class="absolute inset-0 pointer-events-none flex items-center justify-center">
        <!-- Left shade -->
        <div class="flex-1 h-full bg-black/65"></div>
        <!-- 9:16 active viewport box -->
        <div class="h-full aspect-[9/16] relative border-2 border-dashed border-sky-400/90 bg-sky-400/5 flex flex-col justify-between p-2">
          <!-- Top Safe Zone (TikTok/Reels user header info) -->
          <div class="text-[9px] font-bold text-sky-300 bg-zinc-950/80 px-2 py-0.5 rounded self-center shadow">
            📱 Safe Zone (9:16)
          </div>
          <!-- Bottom Safe Zone (Caption & Audio disc) -->
          <div class="text-[9px] font-bold text-amber-300 bg-zinc-950/80 px-2 py-0.5 rounded self-center shadow">
            ⚠️ Margem de Legendas
          </div>
        </div>
        <!-- Right shade -->
        <div class="flex-1 h-full bg-black/65"></div>
      </div>
    {/if}
  </div>

  <!-- Playback Controls Card -->
  <div class="bg-zinc-900/80 p-3 rounded-xl border border-zinc-800/80 space-y-2 select-none shadow-sm">
    <!-- Slider -->
    <div class="flex items-center space-x-2">
      <input
        type="range"
        min={appState.selectedClip?.startSeconds || 0}
        max={appState.selectedClip?.endSeconds || 100}
        step="0.05"
        bind:value={appState.currentTime}
        class="w-full h-1.5 bg-zinc-800 rounded-lg appearance-none cursor-pointer accent-indigo-500"
      />
    </div>

    <!-- Buttons row -->
    <div class="flex items-center justify-between text-xs">
      <div class="flex items-center space-x-2">
        <button
          onclick={togglePlayPause}
          class="flex items-center justify-center w-8 h-8 rounded-lg bg-indigo-600 hover:bg-indigo-500 text-white font-semibold transition cursor-pointer"
        >
          {#if appState.isPlaying}
            <Pause class="w-4 h-4 fill-current" />
          {:else}
            <Play class="w-4 h-4 fill-current" />
          {/if}
        </button>

        <span class="font-mono text-zinc-300 text-xs">
          {formatTime(appState.currentTime)} / {formatTime(appState.selectedClip?.endSeconds || 0)}
        </span>
      </div>

      <div class="flex items-center space-x-3">
        <!-- 9:16 Guide Toggle -->
        <button
          onclick={() => (appState.showCropGuide = !appState.showCropGuide)}
          class="flex items-center space-x-1.5 px-2.5 py-1 rounded-lg text-xs font-medium border transition cursor-pointer {appState.showCropGuide
            ? 'bg-sky-500/20 text-sky-300 border-sky-500/50 shadow-sm'
            : 'bg-zinc-800/60 text-zinc-400 border-zinc-700/60 hover:text-white'}"
        >
          <Smartphone class="w-3.5 h-3.5" />
          <span>📱 Guia 9:16</span>
        </button>

        <!-- Volume -->
        <div class="flex items-center space-x-1.5 text-zinc-400">
          {#if appState.volume === 0}
            <VolumeX class="w-4 h-4" />
          {:else}
            <Volume2 class="w-4 h-4" />
          {/if}
          <input
            type="range"
            min="0"
            max="1"
            step="0.05"
            bind:value={appState.volume}
            class="w-16 h-1.5 bg-zinc-800 rounded appearance-none cursor-pointer accent-indigo-500"
          />
        </div>
      </div>
    </div>
  </div>

  <!-- Fine-tuning Timestamps Panel -->
  <div class="bg-zinc-900/80 p-3 rounded-xl border border-zinc-800/80 space-y-2.5 select-none shadow-sm">
    <div class="flex items-center justify-between">
      <h4 class="text-xs font-bold text-zinc-200">Ajuste Fino dos Marcadores de Tempo</h4>
      <span class="text-[10px] text-zinc-400 font-mono">Precisão milissegundos</span>
    </div>

    <!-- Start Marker Row -->
    <div class="flex items-center space-x-2 text-xs">
      <span class="text-zinc-400 w-10 shrink-0 font-medium">Início:</span>
      <span class="font-mono font-bold text-indigo-400 w-20 shrink-0 text-xs">
        {formatTime(appState.selectedClip?.startSeconds || 0)}
      </span>
      <div class="flex items-center space-x-1">
        <button
          onclick={() => appState.adjustClipBounds(-1, 0)}
          class="px-2 py-1 bg-zinc-800 hover:bg-zinc-700 rounded text-zinc-200 text-[11px] font-mono cursor-pointer"
        >-1s</button>
        <button
          onclick={() => appState.adjustClipBounds(-0.1, 0)}
          class="px-2 py-1 bg-zinc-800 hover:bg-zinc-700 rounded text-zinc-200 text-[11px] font-mono cursor-pointer"
        >-100ms</button>
        <button
          onclick={() => appState.adjustClipBounds(0.1, 0)}
          class="px-2 py-1 bg-zinc-800 hover:bg-zinc-700 rounded text-zinc-200 text-[11px] font-mono cursor-pointer"
        >+100ms</button>
        <button
          onclick={() => appState.adjustClipBounds(1, 0)}
          class="px-2 py-1 bg-zinc-800 hover:bg-zinc-700 rounded text-zinc-200 text-[11px] font-mono cursor-pointer"
        >+1s</button>
      </div>
      <div class="flex-1"></div>
      <button
        onclick={() => appState.setStartToCurrent()}
        class="flex items-center space-x-1 px-3 py-1 bg-indigo-600/90 hover:bg-indigo-500 text-white rounded-lg text-xs font-semibold shadow-sm cursor-pointer"
      >
        <MapPin class="w-3.5 h-3.5" />
        <span>📍 Definir Início Aqui</span>
      </button>
    </div>

    <!-- End Marker Row -->
    <div class="flex items-center space-x-2 text-xs">
      <span class="text-zinc-400 w-10 shrink-0 font-medium">Fim:</span>
      <span class="font-mono font-bold text-indigo-400 w-20 shrink-0 text-xs">
        {formatTime(appState.selectedClip?.endSeconds || 0)}
      </span>
      <div class="flex items-center space-x-1">
        <button
          onclick={() => appState.adjustClipBounds(0, -1)}
          class="px-2 py-1 bg-zinc-800 hover:bg-zinc-700 rounded text-zinc-200 text-[11px] font-mono cursor-pointer"
        >-1s</button>
        <button
          onclick={() => appState.adjustClipBounds(0, -0.1)}
          class="px-2 py-1 bg-zinc-800 hover:bg-zinc-700 rounded text-zinc-200 text-[11px] font-mono cursor-pointer"
        >-100ms</button>
        <button
          onclick={() => appState.adjustClipBounds(0, 0.1)}
          class="px-2 py-1 bg-zinc-800 hover:bg-zinc-700 rounded text-zinc-200 text-[11px] font-mono cursor-pointer"
        >+100ms</button>
        <button
          onclick={() => appState.adjustClipBounds(0, 1)}
          class="px-2 py-1 bg-zinc-800 hover:bg-zinc-700 rounded text-zinc-200 text-[11px] font-mono cursor-pointer"
        >+1s</button>
      </div>
      <div class="flex-1"></div>
      <button
        onclick={() => appState.setEndToCurrent()}
        class="flex items-center space-x-1 px-3 py-1 bg-indigo-600/90 hover:bg-indigo-500 text-white rounded-lg text-xs font-semibold shadow-sm cursor-pointer"
      >
        <MapPin class="w-3.5 h-3.5" />
        <span>📍 Definir Fim Aqui</span>
      </button>
    </div>
  </div>
</div>
