use thiserror::Error;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("operation cancelled")]
    Cancelled,
    #[error("unsupported platform: {0}")]
    UnsupportedPlatform(&'static str),
    #[error("invalid launch request: {0}")]
    InvalidLaunchRequest(String),
    #[error("state migration failed: {0}")]
    StateMigration(String),
    #[error("serialization failed: {0}")]
    Serialization(String),
    #[error("network error: {0}")]
    Network(String),
    #[error("invalid channel status: {0}")]
    InvalidChannelStatus(u16),
    #[error("invalid manifest: {0}")]
    InvalidManifest(String),
    #[error("checksum mismatch for {target}: expected {expected}, got {actual}")]
    ChecksumMismatch {
        target: String,
        expected: String,
        actual: String,
    },
    #[error("process error: {0}")]
    Process(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("zip error: {0}")]
    Zip(String),
}

pub type Result<T> = std::result::Result<T, DomainError>;
