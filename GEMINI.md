# Diretrizes do Projeto Clippr (Rust 2024 + Tauri v2 + Svelte 5 + SQLite + FFmpeg + IA)

## 1. Regra de Ouro: Foco Estrito e Alta Performance
- **Aderência Total ao Escopo Solicitado**: Mantenha a paridade total de recursos com o projeto original em Java, elevando a velocidade para nível nativo em Rust.
- **Eficiência e Velocidade**: Aproveite a compilação nativa do Rust, concorrência segura com Tokio, SQLite em modo WAL e reatividade zero-overhead do Svelte 5.
- **Uso de Subagents**: Decomponha e execute as tarefas do projeto utilizando subagents autônomos (`invoke_subagent`), garantindo execução modular, paralelismo e isolamento de contexto.

## 2. Padrões Técnicos em Rust
- **Cargo Workspace Modular**:
  - `crates/clipr-domain`: Entidades, records/structs, enums, eventos do barramento.
  - `crates/clipr-persistence`: SQLite local (WAL mode) e repositórios com transações rápidas.
  - `crates/clipr-media`: FFmpeg (NVENC/VAAPI/CPU), yt-dlp, ASS/SRT Subtitles, Whisper, thumbnails.
  - `crates/clipr-ai`: Integrações Gemini 1.5/2.0 e OpenAI (GPT-4o/o3-mini), PromptFactory e cache.
  - `crates/clipr-youtube`: OAuth 2.0 PKCE com servidor loopback local e upload de vídeos/shorts.
  - `src-tauri`: Orquestração de pipeline, Tauri commands, emissão de eventos e gerenciamento de estado.
- **Concorrência Segura**:
  - `tokio` para tarefas assíncronas I/O de rede e processos.
  - `CancellationToken` para cancelamento cooperativo e instantâneo.
  - `rayon` ou `spawn_blocking` para tarefas pesadas de CPU.
- **Tratamento de Erros**:
  - `thiserror` nos crates internos.
  - `serde` para serialização/desserialização JSON em IPC e banco de dados.

## 3. Padrões de Frontend (Tauri v2 + Svelte 5)
- **Svelte 5 Runes**: Use `$state()`, `$derived()`, `$effect()` para reatividade pura.
- **Tailwind CSS**: Design system escuro moderno, responsivo e limpo.
- **Comunicação Reativa**: Backend emite eventos (`clippr://progress`, `clippr://event`, `clippr://log`) e frontend consome em tempo real sem polling.

## 4. Testes e Validação
- Sempre execute `cargo test --workspace` após alterações estruturais.
- Garanta que código novo seja acompanhado por testes automatizados em cada crate.
