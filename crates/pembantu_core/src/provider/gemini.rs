use crate::{error::PembantuError, provider::ImageGenerationProvider};
use async_trait::async_trait;

#[derive(Clone)]
pub struct GeminiAPI {
    api_key: String,
    client: reqwest::Client,
    model: String,
}

impl GeminiAPI {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            api_key,
            client: reqwest::Client::new(),
            model
        }
    }
}

#[async_trait]
impl ImageGenerationProvider for GeminiAPI {
    async fn generate_image(&self, prompt: String) -> Result<String, PembantuError> {
        todo!()
    }
}