use thiserror::Error;

#[derive(Error, Debug)]
pub enum PembantuError {
    #[error("request error: {0}")]
    Request(#[from] reqwest::Error),
    #[error("LLM error: {0}")]
    Llm(#[from] async_openai::error::OpenAIError),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("database error: {0}")]
    Db(#[from] sqlx::Error),
    #[error("failed to decode base64 data")]
    Base64DecodeError,
    #[error("generation error: {0}")]
    Generate(String),
    #[error("configuration error: {0}")]
    Config(String),
}
