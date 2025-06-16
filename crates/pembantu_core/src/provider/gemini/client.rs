use crate::{error::PembantuError, prompt, provider::{gemini::types::{CompletionsRequest, Content, GeminiAPI, GenerateContent, GenerateContentResponse, GenerationConfig, Modality, Part, Role}, ImageGenerationProvider, TextGenerationProvider}};
use async_trait::async_trait;
use base64::prelude::*;

#[async_trait]
impl TextGenerationProvider for GeminiAPI {
    async fn generate(&self, message: String) -> Result<String, PembantuError> {
        let req = CompletionsRequest {
            model: self.model.clone(),
            body: GenerateContent {
                generation_config: None,
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

        Ok(response_json.candidates[0].content.parts[0].text.clone().unwrap_or("".into()))
    }
}


#[async_trait]
impl ImageGenerationProvider for GeminiAPI {
    async fn generate_image(&self, prompt: String) -> Result<Vec<u8>, PembantuError> {
        let req = CompletionsRequest {
            model: self.model.clone(),
            body: GenerateContent {
                generation_config: Some(GenerationConfig{
                    response_modalities: vec![Modality::Image, Modality::Text],
                }),
                contents: vec![
                    Content {
                        role: Role::User,
                        parts: vec![
                            Part::text(&prompt)
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
        let mut b64 = None;

        for part in response_json.candidates[0].content.parts.iter() {
            if let Some(data) = &part.inline_data {
                b64 = Some(&data.data);
            }
            // if let Some(text) = part.text {
            //     text = Some(text);
            // }
        }
        if let Some(b64) = b64 {
            let decoded = BASE64_STANDARD.decode(b64)
                .map_err(|e| {
                    dbg!(e);
                    PembantuError::Base64DecodeError
                })?;
            return Ok(decoded)
        }

        return Err(PembantuError::GenerateError("Failed to generate image".into()))
    }
}