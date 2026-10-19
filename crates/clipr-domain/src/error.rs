use thiserror::Error;

#[derive(Error, Debug)]
pub enum DomainError {
    #[error("URL de vídeo inválida: {0}")]
    InvalidVideoUrl(String),

    #[error("Intervalo de tempo inválido: início={0}, fim={1}")]
    InvalidTimeBounds(f64, f64),

    #[error("Transição de status inválida de {from:?} para {to:?}")]
    InvalidStatusTransition {
        from: String,
        to: String,
    },

    #[error("Erro de I/O no sistema de arquivos: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Erro de serialização JSON: {0}")]
    JsonError(#[from] serde_json::Error),
}
