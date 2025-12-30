use async_trait::async_trait;
use dyn_clone::DynClone;
use crate::error::PembantuError;

pub mod openrouter;
pub mod gemini;

pub enum TextProvider {
    // api_key, model_name
    OpenRouter(String, String),
    Gemini(String, String),
}

pub enum ImageProvider {
    // api_key, model_name
    Gemini(String, String),
}

#[async_trait]
pub(crate) trait TextGenerationProvider: Send + Sync + DynClone {
    async fn generate(&self, message: String, images: Option<Vec<String>>) -> Result<String, PembantuError>;
}

#[async_trait]
pub(crate) trait ImageGenerationProvider: Send + Sync {
    async fn generate_image(&self, prompt: String) -> Result<Vec<u8>, PembantuError>;
}

impl Clone for Box<dyn TextGenerationProvider> {
    fn clone(&self) -> Box<dyn TextGenerationProvider> {
        dyn_clone::clone(self)
    }
}