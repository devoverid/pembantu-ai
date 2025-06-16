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
        write!(f, "PembantuError: {}", self);
        Ok(())
    }
}