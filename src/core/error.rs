use crate::core::daemon::rpc::RpcException;

#[derive(Debug, thiserror::Error)]
pub enum DaemonError {
    #[error("DaemonException: {0}")]
    Spawn(String),
    #[error("DaemonException: {0}")]
    Socket(String),
    #[error("RPC error: {0}")]
    Rpc(#[from] RpcException),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Clone manual: `std::io::Error` y `serde_json::Error` no impl `Clone`,
/// pero el UI necesita `Message: Clone`. Reconstruimos `io::Error` con el
/// mismo `kind` + string, y degradamos `Json` a `Spawn` (mismo mensaje,
/// sólo importa para el `to_string()` que usa el reducer).
impl Clone for DaemonError {
    fn clone(&self) -> Self {
        match self {
            DaemonError::Spawn(s) => DaemonError::Spawn(s.clone()),
            DaemonError::Socket(s) => DaemonError::Socket(s.clone()),
            DaemonError::Rpc(e) => DaemonError::Rpc(e.clone()),
            DaemonError::Io(e) => {
                DaemonError::Io(std::io::Error::new(e.kind(), e.to_string()))
            }
            DaemonError::Json(e) => {
                // `serde_json::Error` no es `Clone` ni tiene constructor
                // público; degradamos a `Spawn` conservando el mensaje.
                DaemonError::Spawn(e.to_string())
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("SQL error: {0}")]
    Sql(#[from] rusqlite::Error),
    #[error("join error: {0}")]
    Join(String),
}

/// Clone manual: `rusqlite::Error` no impl `Clone`, pero el reducer del UI
/// necesita `Message: Clone` para anidar `Result<_, DbError>`. Degradeamos
/// `Sql` a `Join` (string) — al reducer sólo le interesa `to_string()`.
impl Clone for DbError {
    fn clone(&self) -> Self {
        match self {
            DbError::Sql(e) => DbError::Join(e.to_string()),
            DbError::Join(s) => DbError::Join(s.clone()),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum NetError {
    #[error("HTTP {status}: {url}")]
    Http { status: reqwest::StatusCode, url: String },
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),
}

/// Error del DownloadManager. Referenciado por la firma pública de
/// `DownloadManager::{poll_once,run_job}`; aún sin rutas que lo construyan
/// en el binario (la cola no está enrutada desde el UI).
#[derive(Debug, thiserror::Error)]
#[allow(dead_code)]
pub enum DownloadError {
    #[error("DB: {0}")]
    Db(#[from] DbError),
    #[error("daemon: {0}")]
    Daemon(#[from] DaemonError),
    #[error("net: {0}")]
    Net(#[from] NetError),
}
