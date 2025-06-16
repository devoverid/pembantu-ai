use thiserror::Error;
use reqwest;

#[derive(Error, Debug)]
pub enum PembantuError {
    RequestError(#[from] reqwest::Error),
    ProviderNotImplemented,
    Base64DecodeError,
    GenerateError(String)
}

impl std::fmt::Display for PembantuError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PembantuError::RequestError(e) => write!(f, "Request error: {}", e),
            PembantuError::ProviderNotImplemented => write!(f, "Provider not implemented"),
            PembantuError::Base64DecodeError => write!(f, "Failed to decode base64 data"),
            PembantuError::GenerateError(msg) => write!(f, "Generation error: {}", msg),
        }.unwrap();
        Ok(())
    }
}