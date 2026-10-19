<script lang="ts">
  import { appState } from '../../lib/stores/appState.svelte';
  import type { AiProvider } from '../../types';
  import {
    Sparkles,
    Key,
    Folder,
    Youtube,
    Activity,
    CheckCircle2,
    AlertCircle,
    Search,
    Save,
    Link,
    Unlink,
    RefreshCw,
    Shield,
  } from 'lucide-svelte';
</script>

<div class="h-full flex flex-col p-5 space-y-5 overflow-y-auto max-w-4xl mx-auto select-none">
  <!-- Active AI Provider Selection -->
  <div class="bg-zinc-900/80 p-4 rounded-xl border border-zinc-800/80 space-y-2 shadow-sm">
    <label class="text-xs font-bold text-zinc-200 flex items-center space-x-2">
      <Sparkles class="w-4 h-4 text-indigo-400" />
      <span>Provedor de Inteligência Artificial Ativo:</span>
    </label>
    <div class="flex items-center space-x-3">
      <select
        bind:value={appState.settings.aiProvider}
        onchange={() => appState.saveSettings()}
        class="bg-zinc-950 border border-zinc-700/80 rounded-lg px-3 py-1.5 text-xs text-white focus:outline-none focus:ring-1 focus:ring-indigo-500 w-64"
      >
        <option value="GEMINI">Google Gemini (Gemini 2.5 Flash)</option>
        <option value="OPENAI">OpenAI ChatGPT (GPT-4o / mini)</option>
      </select>
      <span class="text-[11px] text-zinc-400">
        Escolha o provedor de IA utilizado para analisar momentos virais e roteirizar cortes.
      </span>
    </div>
  </div>

  <!-- Google Gemini API Key Card -->
  <div class="bg-zinc-900/80 p-4 rounded-xl border border-zinc-800/80 space-y-2.5 shadow-sm">
    <label class="text-xs font-bold text-zinc-200 flex items-center space-x-2">
      <Key class="w-4 h-4 text-amber-400" />
      <span>Chave de API do Google Gemini:</span>
    </label>
    <div class="flex items-center space-x-2">
      <input
        type="password"
        bind:value={appState.settings.geminiApiKey}
        placeholder="Cole sua chave GEMINI_API_KEY aqui"
        class="flex-1 bg-zinc-950 border border-zinc-700/80 rounded-lg px-3 py-1.5 text-xs text-white placeholder-zinc-500 focus:outline-none focus:ring-1 focus:ring-indigo-500 font-mono"
      />
      <button
        onclick={() => appState.saveSettings()}
        class="flex items-center space-x-1.5 bg-indigo-600 hover:bg-indigo-500 text-white px-3.5 py-1.5 rounded-lg text-xs font-semibold cursor-pointer"
      >
        <Save class="w-3.5 h-3.5" />
        <span>Salvar Chave</span>
      </button>
      <button
        onclick={() => appState.detectGeminiKey()}
        class="flex items-center space-x-1.5 bg-zinc-800 hover:bg-zinc-700 text-zinc-200 px-3 py-1.5 rounded-lg text-xs font-medium cursor-pointer"
      >
        <Search class="w-3.5 h-3.5 text-zinc-400" />
        <span>🔍 Buscar no Ambiente</span>
      </button>
    </div>
    <p class="text-[11px] text-zinc-400">
      A chave pode ser configurada aqui ou detectada automaticamente de GEMINI_API_KEY (.zshrc, .bashrc, .env).
    </p>
  </div>

  <!-- OpenAI API Key Card -->
  <div class="bg-zinc-900/80 p-4 rounded-xl border border-zinc-800/80 space-y-2.5 shadow-sm">
    <label class="text-xs font-bold text-zinc-200 flex items-center space-x-2">
      <Key class="w-4 h-4 text-emerald-400" />
      <span>Chave de API da OpenAI (ChatGPT):</span>
    </label>
    <div class="flex items-center space-x-2">
      <input
        type="password"
        bind:value={appState.settings.openaiApiKey}
        placeholder="Cole sua chave OPENAI_API_KEY aqui"
        class="flex-1 bg-zinc-950 border border-zinc-700/80 rounded-lg px-3 py-1.5 text-xs text-white placeholder-zinc-500 focus:outline-none focus:ring-1 focus:ring-indigo-500 font-mono"
      />
      <button
        onclick={() => appState.saveSettings()}
        class="flex items-center space-x-1.5 bg-indigo-600 hover:bg-indigo-500 text-white px-3.5 py-1.5 rounded-lg text-xs font-semibold cursor-pointer"
      >
        <Save class="w-3.5 h-3.5" />
        <span>Salvar Chave</span>
      </button>
      <button
        onclick={() => appState.detectOpenAiKey()}
        class="flex items-center space-x-1.5 bg-zinc-800 hover:bg-zinc-700 text-zinc-200 px-3 py-1.5 rounded-lg text-xs font-medium cursor-pointer"
      >
        <Search class="w-3.5 h-3.5 text-zinc-400" />
        <span>🔍 Buscar no Ambiente</span>
      </button>
    </div>
    <p class="text-[11px] text-zinc-400">
      A chave pode ser configurada aqui ou detectada automaticamente de OPENAI_API_KEY (.zshrc, .bashrc, .env).
    </p>
  </div>

  <!-- Export Directory -->
  <div class="bg-zinc-900/80 p-4 rounded-xl border border-zinc-800/80 space-y-2.5 shadow-sm">
    <label class="text-xs font-bold text-zinc-200 flex items-center space-x-2">
      <Folder class="w-4 h-4 text-sky-400" />
      <span>Pasta de Exportação dos Vídeos:</span>
    </label>
    <div class="flex items-center space-x-2">
      <input
        type="text"
        bind:value={appState.settings.outputDirectory}
        class="flex-1 bg-zinc-950 border border-zinc-700/80 rounded-lg px-3 py-1.5 text-xs text-white font-mono"
      />
      <button
        onclick={() => appState.selectDirectory()}
        class="bg-zinc-800 hover:bg-zinc-700 text-zinc-200 px-3.5 py-1.5 rounded-lg text-xs font-medium cursor-pointer"
      >
        Escolher Pasta...
      </button>
    </div>
  </div>

  <!-- YouTube Integration Card -->
  <div class="bg-zinc-900/80 p-4 rounded-xl border border-zinc-800/80 space-y-3.5 shadow-sm">
    <div>
      <h4 class="text-xs font-bold text-zinc-100 flex items-center space-x-2">
        <Youtube class="w-4 h-4 text-rose-500" />
        <span>Integração com o YouTube (Shorts &amp; Vídeos):</span>
      </h4>
      <p class="text-[11px] text-zinc-400 mt-0.5">
        Conecte seu canal oficial do YouTube para publicar Shorts e momentos virais diretamente do Clippr.
      </p>
    </div>

    <div class="grid grid-cols-2 gap-3 text-xs">
      <div class="space-y-1">
        <label for="youtubeClientId" class="text-[11px] text-zinc-400">Google Client ID:</label>
        <input
          id="youtubeClientId"
          type="text"
          bind:value={appState.settings.youtubeClientId}
          placeholder="Cole seu OAuth 2.0 Client ID aqui"
          class="w-full bg-zinc-950 border border-zinc-700/80 rounded-lg px-3 py-1.5 text-xs text-white font-mono"
        />
      </div>

      <div class="space-y-1">
        <label for="youtubeClientSecret" class="text-[11px] text-zinc-400">Google Client Secret:</label>
        <input
          id="youtubeClientSecret"
          type="password"
          bind:value={appState.settings.youtubeClientSecret}
          placeholder="Cole seu OAuth 2.0 Client Secret aqui"
          class="w-full bg-zinc-950 border border-zinc-700/80 rounded-lg px-3 py-1.5 text-xs text-white font-mono"
        />
      </div>
    </div>

    <!-- Connection Status -->
    <div class="flex items-center justify-between p-3 rounded-lg bg-zinc-950/70 border border-zinc-800 text-xs">
      <div class="flex items-center space-x-3">
        {#if appState.settings.youtubeChannelAvatarUrl}
          <img
            src={appState.settings.youtubeChannelAvatarUrl}
            alt="Avatar"
            class="w-7 h-7 rounded-full border border-zinc-700"
          />
        {:else}
          <div class="w-7 h-7 rounded-full bg-zinc-800 flex items-center justify-center text-zinc-500">
            <Youtube class="w-4 h-4" />
          </div>
        {/if}
        <div>
          <p class="font-semibold text-zinc-200">
            {appState.settings.youtubeChannelTitle || 'Canal Não Conectado'}
          </p>
          <p class="text-[10px] text-zinc-500">
            {appState.settings.youtubeChannelTitle ? 'Pronto para uploads de Shorts' : 'Autentique via OAuth 2.0 PKCE'}
          </p>
        </div>
      </div>

      <div class="flex items-center space-x-2">
        {#if appState.settings.youtubeChannelTitle}
          <button
            onclick={() => appState.disconnectYouTube()}
            class="flex items-center space-x-1 bg-zinc-800 hover:bg-zinc-700 text-zinc-300 px-3 py-1.5 rounded-lg text-xs font-medium cursor-pointer"
          >
            <Unlink class="w-3.5 h-3.5" />
            <span>Desconectar</span>
          </button>
        {:else}
          <button
            onclick={() => appState.connectYouTube()}
            class="flex items-center space-x-1 bg-indigo-600 hover:bg-indigo-500 text-white px-3.5 py-1.5 rounded-lg text-xs font-semibold shadow-sm cursor-pointer"
          >
            <Link class="w-3.5 h-3.5" />
            <span>🔗 Conectar Canal</span>
          </button>
        {/if}

        <button
          onclick={() => appState.saveYouTubeCredentials()}
          class="bg-zinc-800 hover:bg-zinc-700 text-zinc-200 px-3 py-1.5 rounded-lg text-xs font-medium cursor-pointer"
        >
          Salvar Credenciais
        </button>
      </div>
    </div>
  </div>

  <!-- Diagnostics Section -->
  <div class="bg-zinc-900/80 p-4 rounded-xl border border-zinc-800/80 space-y-3 shadow-sm">
    <div class="flex items-center justify-between">
      <h4 class="text-xs font-bold text-zinc-100 flex items-center space-x-2">
        <Activity class="w-4 h-4 text-emerald-400" />
        <span>Diagnóstico de Componentes do Sistema:</span>
      </h4>
      <button
        onclick={() => appState.refreshDiagnostics()}
        class="flex items-center space-x-1 text-xs text-indigo-400 hover:text-indigo-300 cursor-pointer"
      >
        <RefreshCw class="w-3 h-3" />
        <span>Recarregar</span>
      </button>
    </div>

    <div class="grid grid-cols-2 gap-3 text-xs">
      <!-- SQLite -->
      <div class="p-2.5 rounded-lg bg-zinc-950/60 border border-zinc-800 flex items-center justify-between">
        <div>
          <span class="text-zinc-400 font-medium">SQLite WAL:</span>
          <p class="text-[11px] text-zinc-300">{appState.diagnostics.sqliteMessage}</p>
        </div>
        <span class="text-[11px] font-bold text-emerald-400">Ativo</span>
      </div>

      <!-- FFmpeg -->
      <div class="p-2.5 rounded-lg bg-zinc-950/60 border border-zinc-800 flex items-center justify-between">
        <div>
          <span class="text-zinc-400 font-medium">FFmpeg:</span>
          <p class="text-[11px] text-zinc-300">{appState.diagnostics.ffmpegEncoder}</p>
        </div>
        <span class="text-[11px] font-bold text-emerald-400">Detectado</span>
      </div>

      <!-- yt-dlp -->
      <div class="p-2.5 rounded-lg bg-zinc-950/60 border border-zinc-800 flex items-center justify-between">
        <div>
          <span class="text-zinc-400 font-medium">yt-dlp:</span>
          <p class="text-[11px] text-zinc-300">Versão: {appState.diagnostics.ytDlpVersion}</p>
        </div>
        <span class="text-[11px] font-bold text-emerald-400">Instalado</span>
      </div>

      <!-- Gemini AI -->
      <div class="p-2.5 rounded-lg bg-zinc-950/60 border border-zinc-800 flex items-center justify-between">
        <div>
          <span class="text-zinc-400 font-medium">Gemini AI:</span>
          <p class="text-[11px] text-zinc-300">{appState.diagnostics.geminiMessage}</p>
        </div>
        <button
          onclick={() => appState.detectGeminiKey()}
          class="text-[10px] text-indigo-400 hover:underline cursor-pointer"
        >
          Detectar
        </button>
      </div>

      <!-- OpenAI -->
      <div class="p-2.5 rounded-lg bg-zinc-950/60 border border-zinc-800 flex items-center justify-between">
        <div>
          <span class="text-zinc-400 font-medium">OpenAI ChatGPT:</span>
          <p class="text-[11px] text-zinc-300">{appState.diagnostics.openAiMessage}</p>
        </div>
        <button
          onclick={() => appState.detectOpenAiKey()}
          class="text-[10px] text-indigo-400 hover:underline cursor-pointer"
        >
          Detectar
        </button>
      </div>
    </div>
  </div>
</div>
