---
name: mestre-rust
description: >-
  Use esta skill para projetar, implementar ou refatorar código Rust de alta performance, utilizando Tokio assíncrono, tratamento de erros idiomático (thiserror/anyhow), serde, concorrência segura, SQLite WAL e padrões de arquitetura modular em Cargo Workspaces.
---

# Mestre do Rust & Alta Performance

Esta skill orienta a escrita e evolução de código Rust idiomático, ultra-rápido e seguro no ecossistema Clippr.

## 1. Concorrência Assíncrona e I/O com Tokio
- Use `tokio` como runtime assíncrono para operações de I/O de rede (APIs de IA Gemini/OpenAI, YouTube OAuth) e I/O de disco:
  ```rust
  tokio::spawn(async move {
      // processamento assíncrono sem travar a thread principal
  });
  ```
- Para subprocessos pesados de longa duração (FFmpeg, yt-dlp):
  - Use `tokio::process::Command` para permitir streaming não bloqueante de stdout/stderr linha a linha (para parsing de progresso `time=hh:mm:ss.xx` e logs).
  - Suporte a cancelamento cooperativo via `tokio_util::sync::CancellationToken`.
- Para processamento CPU-bound (ex: processamento numérico, interpolação de timestamps):
  - Use `tokio::task::spawn_blocking` ou `rayon` para não sufocar o thread pool do Tokio.

## 2. Tratamento de Erros Idiomático
- Use `thiserror` para definir erros tipados e estruturados em bibliotecas de domínio e crates internos:
  ```rust
  #[derive(thiserror::Error, Debug)]
  pub enum PipelineError {
      #[error("Falha na execução do FFmpeg: {0}")]
      Ffmpeg(String),
      #[error("Erro de persistência no banco: {0}")]
      Database(#[from] rusqlite::Error),
      #[error("Chave de API não configurada")]
      MissingApiKey,
  }
  ```
- Use `anyhow::Result` ou tipos de erro mapeáveis para comandos Tauri IPC retornados ao frontend.

## 3. Modelagem de Domínio com Structs, Enums e Serde
- Derive `Debug, Clone, Serialize, Deserialize, PartialEq` onde aplicável.
- Use Enums com `#[serde(rename_all = "snake_case")]` ou similar para estados finitos e variantes bem definidas.
- Mantenha estruturas limpas e imutáveis por padrão, passando referências (`&str`, `&Path`) para evitar alocações desnecessárias.

## 4. Persistência Ultrarrápida com SQLite
- Utilize o modo WAL (`PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;`) para leituras e escritas concorrentes em sub-milissegundos.
- Execute migrações automáticas de schema na inicialização (`CREATE TABLE IF NOT EXISTS`).
- Armazene listas dinâmicas (hashtags, score breakdown) serializadas em JSON quando apropriado.
