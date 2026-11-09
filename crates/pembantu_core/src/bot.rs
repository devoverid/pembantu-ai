use serde_json::json;

use crate::{error::PembantuError, llm::LlmClient, prompt, provider::gemini::types::GeminiAPI};

pub struct Bot {
    llm: LlmClient,
    image: Option<GeminiAPI>,
}

impl Bot {
    pub fn new(llm: LlmClient, image: Option<GeminiAPI>) -> Self {
        Self { llm, image }
    }

    pub async fn generate_text(&self, prompt: String) -> Result<String, PembantuError> {
        let messages = [
            json!({"role": "system", "content": prompt::get_prompt()}),
            json!({"role": "user", "content": prompt}),
        ];
        Ok(self.llm.complete(&messages, &[]).await?.content)
    }

    pub async fn generate_image(&self, prompt: String) -> Result<Vec<u8>, PembantuError> {
        match &self.image {
            Some(image) => image.generate_image(&prompt).await,
            None => Err(PembantuError::Config("image generation is not configured".into())),
        }
    }
}
