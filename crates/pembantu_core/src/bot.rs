use crate::{error::PembantuError, provider::{gemini::GeminiAPI, openrouter::OpenRouterAPI, ImageGenerationProvider, ImageProvider, TextGenerationProvider, TextProvider}};

pub struct Bot {
    text_provider: Option<Box<dyn TextGenerationProvider>>,
    image_provider: Option<Box<dyn ImageGenerationProvider>>
}

impl Bot {
    pub fn new(text_provider: Option<TextProvider>, image_provider: Option<ImageProvider>) -> Result<Self, PembantuError> {
        let text_provider = match text_provider {
            Some(text_provider) => match text_provider {
                TextProvider::OpenRouter(api_key, model_name) => Some(Box::new(OpenRouterAPI::new(api_key, model_name)) as Box<dyn TextGenerationProvider>),
            },
            None => None
        };
        let image_provider = match image_provider {
            Some(image_provider) => match image_provider {
                ImageProvider::Gemini(api_key, model_name) => Some(Box::new(GeminiAPI::new(api_key, model_name)) as Box<dyn ImageGenerationProvider>),
            },
            None => None
        };
        Ok(Self { 
            text_provider,   
            image_provider 
        })
    }

    pub async fn generate_text(&self, prompt: String) -> Result<String, PembantuError> {
        match &self.text_provider {
            Some(provider) => provider.generate(prompt).await,
            None => Err(PembantuError::ProviderNotImplemented)
        }
    }

    pub async fn generate_image(&self, prompt: String) -> Result<String, PembantuError> {
        match &self.image_provider {
            Some(provider) => provider.generate_image(prompt).await,
            None => Err(PembantuError::ProviderNotImplemented)
        }
    }
}