use crate::{error::PembantuError, prompt, provider::{gemini::types::{CompletionsRequest, Content, GeminiAPI, GenerateContent, GenerateContentResponse, Part, Role}, ImageGenerationProvider, TextGenerationProvider}};
use async_trait::async_trait;
use serde::Serialize;


#[async_trait]
impl TextGenerationProvider for GeminiAPI {
    async fn generate(&self, message: String) -> Result<String, PembantuError> {
        let req = CompletionsRequest {
            model: self.model.clone(),
            body: GenerateContent {
                contents: vec![
                    Content {
                        role: Role::Model,
                        parts: vec![
                            Part::text(&prompt::get_prompt())
                        ]
                    },
                    Content {
                        role: Role::User,
                        parts: vec![
                            Part::text(&message)
                        ]
                    },
                ]
            }
        };

        let response = self.client.post(format!("https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}", req.model, self.api_key))
            .header("Content-Type", "application/json")
            .json(&req.body)
            .send()
            .await?;
        
        let response_str = response.text().await?;
        let response_json: GenerateContentResponse = serde_json::from_str::<GenerateContentResponse>(&response_str).unwrap();

        Ok(response_json.candidates[0].content.parts[0].text.clone())
    }
}


#[async_trait]
impl ImageGenerationProvider for GeminiAPI {
    async fn generate_image(&self, prompt: String) -> Result<String, PembantuError> {
        todo!()
    }
}