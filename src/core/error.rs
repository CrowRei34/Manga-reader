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

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("SQL error: {0}")]
    Sql(#[from] rusqlite::Error),
    #[error("join error: {0}")]
    Join(String),
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
