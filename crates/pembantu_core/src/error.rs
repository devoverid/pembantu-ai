use thiserror::Error;

#[derive(Error, Debug)]
pub enum PembantuError {
    #[error("Request error: {0}")]
    RequestError(#[from] reqwest::Error),
    #[error("Provider not implemented")]
    ProviderNotImplemented,
    #[error("Failed to decode base64 data")]
    Base64DecodeError,
    #[error("Generation error: {0}")]
    GenerateError(String),
}
