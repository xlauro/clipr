---
name: pipeline-multimidia
description: >-
  Use esta skill para orientar a execução de operações multimídia de alto desempenho: comandos FFmpeg com aceleração por hardware (NVENC/VAAPI/QSV/libx264), geração e estilização de legendas ASS (presets Hormozi, MrBeast, Safe Zone), extração de áudio, Whisper e download via yt-dlp.
---

# Pipeline Multimídia de Alta Performance

Esta skill orienta o processamento audiovisual no Clippr, garantindo renderizações ultrarrápidas, legendagem de alto impacto e compatibilidade entre plataformas.

## 1. Execução do FFmpeg e Aceleração por Hardware
- **Detecção Inteligente de Encoders**:
  - Testar disponibilidade de hardware: `h264_nvenc` (NVIDIA), `h264_vaapi` (Linux Intel/AMD), `h264_qsv` (Intel), `h264_videotoolbox` (macOS).
  - Fallback gracioso para CPU multithreaded com `libx264 -preset veryfast` ou `fast`.
- **Parsing de Progresso**:
  - Monitorar linhas de stderr com regex `time=(\d+):(\d{1,2}):(\d{1,2}(?:\.\d+)?)` para emitir progresso percentual exato ao usuário.
- **Filtros de Enquadramento 9:16**:
  - *Auto-Crop*: `crop=ih*9/16:ih:(iw-ih*9/16)/2:0,scale=1080:1920`
  - *Fundo Desfocado (Blurred Background)*: `[0:v]scale=1080:1920:force_original_aspect_ratio=increase,boxblur=20:5[bg]; [0:v]scale=1080:-1[fg]; [bg][fg]overlay=(W-w)/2:(H-h)/2`
  - *Split-Screen (Podcasts)*: Recorte e empilhamento vertical de dois planos.

## 2. Geração Avançada de Legendas ASS (SubStation Alpha)
- Suporte a presets de estilo de alta retenção:
  - **Hormozi Amarelo**: Fonte Montserrat ExtraBold em caixa alta, texto principal em amarelo (`&H0000FFFF`), borda preta grossa de 4px, sombra de 2px.
  - **MrBeast**: Texto branco com destaque de palavras-chave em verde neon (`&H0000FF00`) e rosa.
  - **Cyber Green / Neon Magenta / Clean White**.
- **Safe Zone Vertical**: Margem inferior segura de 280-350 pixels para não sobrepor botões nativos do TikTok, Reels e YouTube Shorts (curtir, comentar, descrição).
- **Karaokê e Animação Pop/Zoom**:
  - Marcação de tempo palavra por palavra (`\k<centissegundos>`).
  - Animação de escala inicial `\t(0, 100, \fscx100\fscy100)` para efeito pop.

## 3. Gestão do yt-dlp
- Localizar o binário prioritariamente em `bin/yt-dlp` local ao aplicativo.
- Se não encontrado localmente, buscar no `PATH` do sistema operacional.
- Se ausente em ambos, realizar download automático da release oficial mais recente do GitHub para a pasta de dados do usuário.
- Baixar legendas em formato VTT ou JSON3 antes do vídeo completo para avaliação rápida pela IA antes de gastar banda e disco com o download completo.
