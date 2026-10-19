---
name: tauri-suite
description: >-
  Use esta skill para orientar a integração entre Tauri v2 e o frontend Svelte 5, comandos IPC tipados, streaming reativo de eventos (Tauri Event Emitter), gerenciamento de estado gerenciado (tauri::State) e segurança.
---

# Tauri v2 & Svelte 5 Integration Guide

Esta skill orienta a integração limpa, reativa e de alto desempenho entre o backend Rust e o frontend Svelte 5 no Clippr.

## 1. Padrões de Comandos IPC (`#[tauri::command]`)
- Toda função invocada pelo frontend deve ser decorada com `#[tauri::command]`:
  ```rust
  #[tauri::command]
  pub async fn add_video_to_queue(
      url: String,
      pipeline: tauri::State<'_, Arc<ClipprPipeline>>,
  ) -> Result<VideoSource, String> {
      pipeline.add_video(&url).await.map_err(|e| e.to_string())
  }
  ```
- Retorne sempre `Result<T, String>` ou `Result<T, CustomError>` serializável para capturar erros no frontend com `try/catch`.

## 2. Emissão de Eventos em Tempo Real (Event Streaming)
- O backend deve notificar o frontend sobre progresso, logs e novos cortes sem que o frontend precise fazer polling:
  ```rust
  app_handle.emit("clippr://progress", &ProgressPayload {
      stage: "CUTTING_FFMPEG".into(),
      progress: 0.85,
      details: "Renderizando corte 1 de 3".into(),
  })?;
  ```
- No Svelte 5, use `listen` do `@tauri-apps/api/event`:
  ```typescript
  import { listen } from '@tauri-apps/api/event';
  
  $effect(() => {
    const unlisten = listen('clippr://progress', (event) => {
      progress = event.payload.progress;
    });
    return () => { unlisten.then(fn => fn()); };
  });
  ```

## 3. Estado Gerenciado com `tauri::State`
- Armazene o `DatabaseManager`, o canal de eventos e o `ClipprPipeline` como recursos gerenciados com `app.manage(...)`.
- Use tipos thread-safe (`Arc<tokio::sync::Mutex<...>>` ou `Arc<RwLock<...>>`).

## 4. Boas Práticas Svelte 5 (Runes)
- Use `$state()` para reatividade pura e previsível.
- Use `$derived()` para valores computados (ex: filtros de tabela, scores calculados).
- Use `$effect()` para sincronização de efeitos colaterais e listeners de eventos Tauri.
- Mantenha a separação de responsabilidades em componentes reutilizáveis (`VideoQueue.svelte`, `ClipCard.svelte`, `VideoPlayer.svelte`, `SettingsModal.svelte`, `LogsDrawer.svelte`).
