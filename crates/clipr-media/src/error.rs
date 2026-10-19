use thiserror::Error;

#[derive(Error, Debug)]
pub enum MediaError {
    #[error("Erro de I/O: {0}")]
    Io(#[from] std::io::Error),

    #[error("Erro na execução do FFmpeg (código de saída {exit_code}): {details}")]
    FfmpegExecution {
        exit_code: i32,
        details: String,
    },

    #[error("FFmpeg não encontrado no sistema")]
    FfmpegNotFound,

    #[error("Erro na execução do yt-dlp (código de saída {exit_code}): {details}")]
    YtDlpExecution {
        exit_code: i32,
        details: String,
    },

    #[error("yt-dlp não encontrado no sistema")]
    YtDlpNotFound,

    #[error("Erro de requisição de rede: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Erro de parsing JSON: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Argumento inválido: {0}")]
    InvalidArgument(String),

    #[error("Timeout na operação multimídia: {0}")]
    Timeout(String),

    #[error("Serviço Whisper indisponível: {0}")]
    WhisperUnavailable(String),

    #[error("Erro no processamento de mídia: {0}")]
    General(String),
}
