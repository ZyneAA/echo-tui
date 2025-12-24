use thiserror::Error;

#[derive(Error, Debug)]
pub enum EchoError {
    #[error("IO: {0}")]
    Io(#[from] std::io::Error),

    #[error("Audio: {0}")]
    Audio(String),

    #[error("Metadata: {0}")]
    InvalidMetadata(String),

    #[error("Config: {0}")]
    ConfigError(String),

    #[error("Lock poisoned: {0}")]
    LockPoisoned(String),

    #[error("Thread took too long to respond (Timeout)")]
    ThreadTimeout,

    #[error("Resource is busy, try again later")]
    ResourceBusy,
}

pub type EchoResult<T> = Result<T, EchoError>;
