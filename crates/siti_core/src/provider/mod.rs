pub mod gemini;
pub mod openrouter;

use crate::error::SitiError;
use gemini::types::GeminiAPI;
use openrouter::OpenRouterImageAPI;

/// The configured image generation backend.
#[derive(Clone)]
pub enum ImageProvider {
    Gemini(GeminiAPI),
    OpenRouter(OpenRouterImageAPI),
}

impl ImageProvider {
    pub async fn generate_image(&self, prompt: &str) -> Result<Vec<u8>, SitiError> {
        match self {
            ImageProvider::Gemini(api) => api.generate_image(prompt).await,
            ImageProvider::OpenRouter(api) => api.generate_image(prompt).await,
        }
    }
}
