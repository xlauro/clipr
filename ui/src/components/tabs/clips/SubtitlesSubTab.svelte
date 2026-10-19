<script lang="ts">
  import { appState } from '../../../lib/stores/appState.svelte';
  import { SubtitlePresetDetails, type SubtitlePreset } from '../../../types';
  import {
    Zap,
    Save,
    Smile,
    MessageSquare,
    Check,
    Edit3,
  } from 'lucide-svelte';

  function formatTime(seconds: number): string {
    const clamped = Math.max(0, seconds || 0);
    const m = Math.floor(clamped / 60);
    const s = Math.floor(clamped % 60);
    const ms = Math.floor((clamped - Math.floor(clamped)) * 100);
    return `${m.toString().padStart(2, '0')}:${s.toString().padStart(2, '0')}.${ms.toString().padStart(2, '0')}`;
  }

  function handleWordKeyDown(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      appState.applyWordEdit();
    }
  }
</script>

<div class="h-full flex flex-col p-4 space-y-3 overflow-hidden select-none">
  <!-- Controls Bar -->
  <div class="bg-zinc-900/80 p-3 rounded-xl border border-zinc-800/80 flex items-center justify-between shadow-sm text-xs">
    <div class="flex items-center space-x-3">
      <span class="font-bold text-zinc-300 flex items-center space-x-1">
        <MessageSquare class="w-3.5 h-3.5 text-indigo-400" />
        <span>💬 Legendas &amp; Transcrição:</span>
      </span>

      <!-- Preset Combo -->
      <div class="flex items-center space-x-1.5">
        <span class="text-zinc-400">Preset:</span>
        <select
          value={appState.selectedClip?.subtitlePreset || 'HORMOZI_YELLOW'}
          onchange={(e) => appState.updateSubtitlePreset(e.currentTarget.value as SubtitlePreset)}
          class="bg-zinc-950 border border-zinc-700/80 rounded-lg px-2 py-1 text-xs text-white focus:outline-none focus:ring-1 focus:ring-indigo-500"
        >
          {#each Object.entries(SubtitlePresetDetails) as [key, val]}
            <option value={key}>{val.label}</option>
          {/each}
        </select>
      </div>

      <!-- Emojis Toggle -->
      <label class="flex items-center space-x-1.5 cursor-pointer text-zinc-200">
        <input
          type="checkbox"
          bind:checked={appState.hasDynamicEmojis}
          class="rounded bg-zinc-950 border-zinc-700 text-indigo-600 focus:ring-indigo-500 cursor-pointer"
        />
        <span class="font-bold text-amber-300 flex items-center space-x-1">
          <Smile class="w-3.5 h-3.5" />
          <span>🔥 Emojis Dinâmicos</span>
        </span>
      </label>
    </div>

    <div class="flex items-center space-x-2">
      <button
        onclick={() => appState.transcribeWhisper()}
        class="flex items-center space-x-1 bg-indigo-600 hover:bg-indigo-500 text-white px-3 py-1.5 rounded-lg font-semibold shadow-sm cursor-pointer"
      >
        <Zap class="w-3.5 h-3.5 fill-current" />
        <span>⚡ Transcrever com Whisper</span>
      </button>

      <button
        onclick={() => appState.saveSubtitleWords()}
        class="flex items-center space-x-1 bg-zinc-800 hover:bg-zinc-700 text-zinc-200 px-3 py-1.5 rounded-lg font-medium cursor-pointer"
      >
        <Save class="w-3.5 h-3.5" />
        <span>💾 Salvar Alterações</span>
      </button>
    </div>
  </div>

  <!-- Words Table / Sync List -->
  <div class="flex-1 bg-zinc-900/50 rounded-xl border border-zinc-800/80 flex flex-col overflow-hidden shadow-inner">
    {#if appState.currentClipWords.length === 0}
      <!-- Empty State -->
      <div class="flex-1 flex flex-col items-center justify-center p-8 text-center">
        <div class="w-12 h-12 rounded-xl bg-zinc-800/70 flex items-center justify-center text-2xl mb-2">
          💬
        </div>
        <h4 class="text-xs font-bold text-zinc-200">Nenhuma palavra transcrita para este corte</h4>
        <p class="text-[11px] text-zinc-400 mt-1 max-w-sm">
          Clique em "⚡ Transcrever com Whisper" para extrair o áudio e alinhar as palavras com precisão de milissegundos.
        </p>
      </div>
    {:else}
      <!-- Header row -->
      <div class="grid grid-cols-12 gap-2 px-3 py-2 bg-zinc-900/90 border-b border-zinc-800 text-[11px] font-semibold text-zinc-400">
        <div class="col-span-2">Início</div>
        <div class="col-span-2">Fim</div>
        <div class="col-span-8">Palavra / Texto Sincronizado</div>
      </div>

      <!-- Word items list -->
      <div class="flex-1 overflow-y-auto divide-y divide-zinc-800/50">
        {#each appState.currentClipWords as word, idx}
          <button
            onclick={() => appState.selectWord(idx)}
            class="grid grid-cols-12 gap-2 px-3 py-1.5 items-center w-full text-left text-xs transition cursor-pointer {appState.selectedWordIndex === idx
              ? 'bg-indigo-950/60 text-white font-semibold'
              : 'hover:bg-zinc-800/40 text-zinc-300'}"
          >
            <div class="col-span-2 font-mono text-[11px] text-zinc-400">
              {formatTime(word.startSeconds)}
            </div>
            <div class="col-span-2 font-mono text-[11px] text-zinc-400">
              {formatTime(word.endSeconds)}
            </div>
            <div class="col-span-8 flex items-center space-x-1.5">
              <span>{word.word}</span>
            </div>
          </button>
        {/each}
      </div>
    {/if}
  </div>

  <!-- Quick Edit Line -->
  <div class="bg-zinc-900/80 p-2.5 rounded-xl border border-zinc-800/80 flex items-center space-x-2 text-xs">
    <span class="text-zinc-400 font-medium shrink-0 flex items-center space-x-1">
      <Edit3 class="w-3.5 h-3.5 text-zinc-500" />
      <span>Editar Palavra:</span>
    </span>
    <input
      type="text"
      bind:value={appState.editWordText}
      onkeydown={handleWordKeyDown}
      disabled={appState.selectedWordIndex === null}
      placeholder={appState.selectedWordIndex !== null ? 'Digite a correção da palavra' : 'Selecione uma palavra na tabela acima'}
      class="flex-1 bg-zinc-950 border border-zinc-700/80 rounded-lg px-3 py-1 text-xs text-white placeholder-zinc-500 focus:outline-none focus:ring-1 focus:ring-indigo-500 disabled:opacity-40"
    />
    <button
      onclick={() => appState.applyWordEdit()}
      disabled={appState.selectedWordIndex === null}
      class="flex items-center space-x-1 bg-indigo-600 hover:bg-indigo-500 disabled:opacity-40 text-white px-3 py-1 rounded-lg text-xs font-semibold cursor-pointer"
    >
      <Check class="w-3.5 h-3.5" />
      <span>Aplicar</span>
    </button>
  </div>
</div>
