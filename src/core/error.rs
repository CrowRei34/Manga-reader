#[derive(Debug, thiserror::Error)]
pub enum DaemonError {
    #[error("{0}")]
    Generic(String),
}
