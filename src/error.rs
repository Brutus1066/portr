//! Error types for portr

use thiserror::Error;

/// All possible errors in portr
#[derive(Error, Debug)]
pub enum PortrError {
    #[error("invalid port: {0}")]
    InvalidPort(String),

    #[error("invalid port range: {0}")]
    InvalidPortRange(String),

    #[error("failed to get network connections: {0}")]
    NetworkError(String),

    #[error("failed to kill process {0}: {1}")]
    KillError(u32, String),

    #[error("permission denied: {0}")]
    PermissionDenied(String),

    #[error("process not found: PID {0}")]
    ProcessNotFound(u32),

    #[error("export error: {0}")]
    ExportError(String),

    #[error("IO error: {0}")]
    IoError(String),

    #[error("JSON serialization error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Docker error: {0}")]
    DockerError(String),

    #[error("Docker not available: {0}")]
    DockerNotAvailable(String),

    #[error("System error: {0}")]
    SystemError(String),
}

impl From<std::io::Error> for PortrError {
    fn from(err: std::io::Error) -> Self {
        PortrError::IoError(err.to_string())
    }
}
