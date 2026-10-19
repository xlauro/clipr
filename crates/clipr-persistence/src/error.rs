use thiserror::Error;

#[derive(Error, Debug)]
pub enum PersistenceError {
    #[error("Erro SQLite: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("Erro de serialização JSON: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Erro de I/O: {0}")]
    Io(#[from] std::io::Error),

    #[error("Entidade não encontrada: {0}")]
    NotFound(String),

    #[error("Falha ao adquirir lock do banco: {0}")]
    LockError(String),
}
