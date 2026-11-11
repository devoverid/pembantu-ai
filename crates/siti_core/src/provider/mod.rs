pub mod gemini;
pub mod openrouter;

use crate::{agent::ImageInput, error::SitiError};
use gemini::types::GeminiAPI;
use openrouter::OpenRouterImageAPI;

/// The configured image generation backend.
#[derive(Clone)]
pub enum ImageProvider {
    Gemini(GeminiAPI),
    OpenRouter(OpenRouterImageAPI),
}

impl ImageProvider {
    /// Generates an image from `prompt`; non-empty `images` are sent as references to edit.
    pub async fn generate_image(&self, prompt: &str, images: &[ImageInput]) -> Result<Vec<u8>, SitiError> {
        match self {
            ImageProvider::Gemini(api) => api.generate_image(prompt, images).await,
            ImageProvider::OpenRouter(api) => api.generate_image(prompt, images).await,
        }
    }
}
