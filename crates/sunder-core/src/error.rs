use thiserror::Error;

#[derive(Error, Debug)]
pub enum SunderError {
    #[error("partial signing failed: {0}")]
    PartialSignFailed(String),

    #[error("signature assembly failed: {0}")]
    AssemblyFailed(String),

    #[error("signature verification failed")]
    VerificationFailed,

    #[error("not enough partial signatures: need {need}, got {got}")]
    InsufficientShares { need: usize, got: usize },

    #[error("key not found: '{0}'")]
    KeyNotFound(String),

    #[error("keystore load failed: {0}")]
    KeystoreLoad(String),

    #[error("serialization error: {0}")]
    Serialization(String),

    #[error("http error: {0}")]
    Http(String),

    #[error("invalid hex: {0}")]
    InvalidHex(String),

    #[error("unauthorized: invalid or missing API token")]
    Unauthorized,

    #[error("configuration error: {0}")]
    Config(String),
}