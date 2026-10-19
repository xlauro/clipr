<script lang="ts">
  import { appState } from '../lib/stores/appState.svelte';
  import { CheckCircle2, AlertTriangle, AlertCircle, Info, X } from 'lucide-svelte';
</script>

<div class="fixed bottom-16 right-5 z-50 flex flex-col space-y-2 pointer-events-none max-w-sm w-full">
  {#each appState.toasts as toast (toast.id)}
    <div
      class="pointer-events-auto flex items-start p-3 rounded-xl border backdrop-blur-md shadow-2xl transition-all duration-300 animate-in slide-in-from-right {toast.type === 'success'
        ? 'bg-zinc-900/95 border-emerald-500/40 text-emerald-200'
        : toast.type === 'warning'
        ? 'bg-zinc-900/95 border-amber-500/40 text-amber-200'
        : toast.type === 'error'
        ? 'bg-zinc-900/95 border-rose-500/40 text-rose-200'
        : 'bg-zinc-900/95 border-indigo-500/40 text-indigo-200'}"
    >
      <div class="mr-2.5 mt-0.5 shrink-0">
        {#if toast.type === 'success'}
          <CheckCircle2 class="w-4 h-4 text-emerald-400" />
        {:else if toast.type === 'warning'}
          <AlertTriangle class="w-4 h-4 text-amber-400" />
        {:else if toast.type === 'error'}
          <AlertCircle class="w-4 h-4 text-rose-400" />
        {:else}
          <Info class="w-4 h-4 text-indigo-400" />
        {/if}
      </div>

      <div class="flex-1 min-w-0 pr-2">
        <h4 class="text-xs font-semibold text-zinc-100">{toast.title}</h4>
        <p class="text-[11px] text-zinc-300 mt-0.5 break-words">{toast.message}</p>
      </div>

      <button
        onclick={() => appState.dismissToast(toast.id)}
        class="text-zinc-400 hover:text-white transition-colors p-1 -mr-1 -mt-1 rounded"
        title="Fechar"
      >
        <X class="w-3.5 h-3.5" />
      </button>
    </div>
  {/each}
</div>
