<script lang="ts">
  import { appState } from '../../../lib/stores/appState.svelte';
  import {
    FramingModeDetails,
    SubtitlePresetDetails,
    type FramingMode,
    type SubtitlePreset,
  } from '../../../types';
  import {
    Layers,
    Type,
    Check,
    Info,
    Smartphone,
    Columns2,
    Monitor,
    Maximize2,
  } from 'lucide-svelte';

  const framingModes: { id: FramingMode; label: string; desc: string; icon: any }[] = [
    {
      id: 'AUTO_CROP',
      label: 'Auto-Crop 9:16 (Tela Cheia)',
      desc: 'Recorte vertical inteligente centralizado na ação principal sem faixas pretas.',
      icon: Maximize2,
    },
    {
      id: 'SPLIT_SCREEN',
      label: 'Split-Screen 9:16 (Podcasts)',
      desc: 'Divisão em dois planos verticais empilhando host (topo) e convidado (base).',
      icon: Columns2,
    },
    {
      id: 'BLURRED_BG',
      label: 'Fundo Desfocado (9:16)',
      desc: 'Vídeo original 16:9 no centro com preenchimento em blur superior/inferior.',
      icon: Layers,
    },
    {
      id: 'HORIZONTAL_16_9',
      label: 'Horizontal 16:9 (Padrão)',
      desc: 'Mantém a proporção nativa sem corte para o YouTube tradicional.',
      icon: Monitor,
    },
  ];

  const subtitleStyles: { id: SubtitlePreset; label: string; primary: string; secondary: string }[] = [
    { id: 'HORMOZI_YELLOW', label: 'Hormozi Amarelo', primary: '#FFFF00', secondary: '#00FF00' },
    { id: 'MRBEAST_BLUE_YELLOW', label: 'MrBeast (Azul/Amarelo)', primary: '#00D7FF', secondary: '#FFFF00' },
    { id: 'CYBER_GREEN', label: 'Cyber Green', primary: '#00FF66', secondary: '#00FFFF' },
    { id: 'NEON_MAGENTA', label: 'Neon Magenta', primary: '#FF00FF', secondary: '#FFFF00' },
    { id: 'MINIMALIST_ELEGANT', label: 'Minimalista Elegante', primary: '#FFFFFF', secondary: '#00E5FF' },
    { id: 'CLEAN_WHITE', label: 'Clean White', primary: '#FFFFFF', secondary: '#00D7FF' },
  ];
</script>

<div class="h-full flex flex-col p-4 space-y-4 overflow-y-auto">
  <!-- Active Badges Summary Card -->
  <div class="bg-zinc-900/80 p-3 rounded-xl border border-zinc-800/80 flex items-center justify-between shadow-sm select-none">
    <div class="flex items-center space-x-2">
      <Layers class="w-4 h-4 text-indigo-400" />
      <span class="text-xs font-bold text-zinc-200">Enquadramento &amp; Legendas Ativas:</span>
    </div>
    <div class="flex items-center space-x-2">
      <span class="text-[11px] font-semibold px-2.5 py-0.5 rounded-full bg-indigo-500/20 text-indigo-300 border border-indigo-500/30">
        {FramingModeDetails[appState.selectedClip?.framingMode || 'AUTO_CROP'].label}
      </span>
      <span class="text-[11px] font-semibold px-2.5 py-0.5 rounded-full bg-amber-500/20 text-amber-300 border border-amber-500/30">
        {SubtitlePresetDetails[appState.selectedClip?.subtitlePreset || 'HORMOZI_YELLOW'].label}
      </span>
    </div>
  </div>

  <!-- Framing Mode Selection -->
  <div class="space-y-2 select-none">
    <label class="text-xs font-bold text-zinc-300 flex items-center space-x-1.5">
      <Smartphone class="w-3.5 h-3.5 text-indigo-400" />
      <span>Modo de Enquadramento:</span>
    </label>

    <div class="grid grid-cols-2 gap-2.5">
      {#each framingModes as mode}
        {@const IconComponent = mode.icon}
        <button
          onclick={() => appState.updateFraming(mode.id)}
          class="flex items-start space-x-3 p-3 rounded-xl border text-left transition-all cursor-pointer {appState.selectedClip?.framingMode === mode.id
            ? 'bg-indigo-950/40 border-indigo-500 shadow-md shadow-indigo-500/10'
            : 'bg-zinc-900/60 border-zinc-800/80 hover:bg-zinc-800/60 hover:border-zinc-700'}"
        >
          <div class="p-2 rounded-lg {appState.selectedClip?.framingMode === mode.id ? 'bg-indigo-600 text-white' : 'bg-zinc-800 text-zinc-400'} shrink-0">
            <IconComponent class="w-4 h-4" />
          </div>
          <div class="min-w-0 flex-1">
            <div class="flex items-center justify-between">
              <h5 class="text-xs font-semibold text-zinc-100">{mode.label}</h5>
              {#if appState.selectedClip?.framingMode === mode.id}
                <Check class="w-3.5 h-3.5 text-indigo-400" />
              {/if}
            </div>
            <p class="text-[11px] text-zinc-400 mt-0.5 line-clamp-2">{mode.desc}</p>
          </div>
        </button>
      {/each}
    </div>
  </div>

  <!-- Subtitle Preset Selection -->
  <div class="space-y-2 select-none">
    <label class="text-xs font-bold text-zinc-300 flex items-center space-x-1.5">
      <Type class="w-3.5 h-3.5 text-amber-400" />
      <span>Estilo de Legenda Dinâmica:</span>
    </label>

    <div class="grid grid-cols-3 gap-2.5">
      {#each subtitleStyles as preset}
        <button
          onclick={() => appState.updateSubtitlePreset(preset.id)}
          class="flex flex-col p-3 rounded-xl border text-left transition-all cursor-pointer {appState.selectedClip?.subtitlePreset === preset.id
            ? 'bg-amber-950/30 border-amber-500 shadow-md shadow-amber-500/10'
            : 'bg-zinc-900/60 border-zinc-800/80 hover:bg-zinc-800/60 hover:border-zinc-700'}"
        >
          <div class="flex items-center justify-between w-full mb-2">
            <span class="text-xs font-semibold text-zinc-200">{preset.label}</span>
            <div class="flex space-x-1">
              <span class="w-3 h-3 rounded-full border border-black/40" style="background-color: {preset.primary}"></span>
              <span class="w-3 h-3 rounded-full border border-black/40" style="background-color: {preset.secondary}"></span>
            </div>
          </div>
          <!-- Subtitle Preview Pill -->
          <div class="bg-black/80 px-2 py-1 rounded text-center border border-zinc-800">
            <span
              class="text-[11px] font-black uppercase tracking-wider"
              style="color: {preset.primary}; text-shadow: 1px 1px 0 #000, -1px -1px 0 #000;"
            >
              VIRAL RETENTION
            </span>
          </div>
        </button>
      {/each}
    </div>
  </div>

  <!-- Educational framing guide box -->
  <div class="bg-zinc-900/50 p-3 rounded-xl border border-zinc-800/80 space-y-1.5 text-[11px] text-zinc-400">
    <div class="flex items-center space-x-1.5 font-semibold text-zinc-300">
      <Info class="w-3.5 h-3.5 text-indigo-400" />
      <span>Guia dos Modos de Enquadramento:</span>
    </div>
    <ul class="space-y-1 pl-4 list-disc text-zinc-400">
      <li><strong class="text-zinc-300">Auto-Crop (9:16):</strong> Ideal para vídeos solos e tutoriais. Centraliza no orador em resolução nativa vertical 1080x1920.</li>
      <li><strong class="text-zinc-300">Split-Screen (Podcasts):</strong> Cria layout empilhado topo/base para cortes de entrevistas e conversas entre duas pessoas.</li>
      <li><strong class="text-zinc-300">Fundo Desfocado:</strong> Mantém a tela 16:9 inteira centralizada com efeito blur nas margens superior e inferior.</li>
      <li><strong class="text-zinc-300">Horizontal 16:9:</strong> Exporta para o padrão tradicional do YouTube sem alterações geométricas.</li>
    </ul>
  </div>
</div>
