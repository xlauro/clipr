---
name: testes-rust
description: >-
  Use esta skill para orientar a criação e execução de testes automatizados unitários, de integração e de regressão em Rust, com cargo test, mocks determinísticos e validação estrita de invariantes.
---

# Mestre de Testes em Rust

Esta skill orienta a suíte de testes do Clippr para garantir estabilidade, confiabilidade e ausência de regressões em todas as fases da migração.

## 1. Testes Unitários de Domínio (`crates/clipr-domain`)
- Testar invariantes de modelos (`ClipCandidate`, `VideoSource`, `AppSettings`):
  - Início menor que fim.
  - Scores no intervalo [0, 100].
  - Normalização e validação de URLs do YouTube (shorts, watch, youtu.be).
  - Formatação de timestamps e duração.

## 2. Testes de Persistência (`crates/clipr-persistence`)
- Usar banco SQLite em memória (`:memory:`) para testes rápidos e isolados:
  ```rust
  #[tokio::test]
  async fn test_save_and_retrieve_video() {
      let db = DatabaseManager::in_memory().await.unwrap();
      // asserções
  }
  ```
- Testar persistência de arrays JSON (hashtags, scores), atualizações de status e recuperação de configurações.

## 3. Testes do Pipeline Multimídia (`crates/clipr-media`)
- Testar geração de arquivos de legenda `.ass` e `.srt` com presets variados.
- Testar parsing de legendas VTT e JSON3 com timestamps em casos extremos (vídeos longos, sobreposição).
- Testar expressões regulares de parsing de progresso do FFmpeg (`time=00:01:23.45`).

## 4. Testes de Clientes de IA (`crates/clipr-ai`)
- Testar parsing dos JSONs gerados pela IA com dados válidos, campos faltantes e caracteres especiais.
- Testar cache de análises (`TranscriptAnalysisCache`).

## 5. Execução Contínua
- Execute `cargo test --workspace` regularmente.
- Verifique se todos os testes passam sem warnings com `cargo clippy --workspace --all-targets`.
