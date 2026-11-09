use crate::{error::PembantuError, prompt, provider::{gemini::types::{CompletionsRequest, Content, GeminiAPI, GenerateContent, GenerateContentResponse, GenerationConfig, Modality, Part, Role}, ImageGenerationProvider, TextGenerationProvider}};
use async_trait::async_trait;
use base64::prelude::*;

#[async_trait]
impl TextGenerationProvider for GeminiAPI {
    async fn generate(&self, message: String, images: Option<Vec<String>>) -> Result<String, PembantuError> {
        let mut user_parts = vec![Part::text(&message)];
        if let Some(images) = images {
            for image_url in images {
                let response = self.client.get(&image_url).send().await?;
                let image_data = response.bytes().await?;
                let base64_image = BASE64_STANDARD.encode(image_data);
                user_parts.push(Part::image("image/jpeg", &base64_image));
            }
        }
        log::debug!("req {:?}", user_parts.len());

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
                        parts: user_parts
                    },
                ]
            }
        };


        let response = self.client.post(format!("https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}", req.model, self.api_key))
            .header("Content-Type", "application/json")
            .json(&req.body)
            .send()
            .await?;
        let status = response.status();
        let response_str = response.text().await?;

        if status != 200 {
            log::error!("Error generating response: {}", response_str);
            return Ok("Error generating response".into());
        }

        let response_json: GenerateContentResponse = serde_json::from_str(&response_str)
            .map_err(|e| PembantuError::GenerateError(format!("invalid Gemini response: {e}")))?;

        response_json.candidates.first()
            .and_then(|c| c.content.as_ref())
            .and_then(|c| c.parts.iter().find_map(|p| p.text.clone()))
            .ok_or_else(|| PembantuError::GenerateError("no text returned".into()))
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
        let status = response.status();
        let body = response.text().await?;
        if !status.is_success() {
            return Err(PembantuError::GenerateError(format!("Gemini image API {status}: {body}")));
        }
        let response_json: GenerateContentResponse = serde_json::from_str(&body)
            .map_err(|e| PembantuError::GenerateError(format!("invalid Gemini image response: {e}")))?;

        let candidate = response_json.candidates.first();
        let parts = candidate
            .and_then(|c| c.content.as_ref())
            .map(|c| c.parts.as_slice())
            .unwrap_or_default();

        if let Some(data) = parts.iter().find_map(|p| p.inline_data.as_ref()) {
            return BASE64_STANDARD.decode(&data.data).map_err(|_| PembantuError::Base64DecodeError);
        }

        let finish_reason = candidate.and_then(|c| c.finish_reason.as_deref()).unwrap_or("none");
        let text: String = parts.iter().filter_map(|p| p.text.as_deref()).collect::<Vec<_>>().join(" ");
        Err(PembantuError::GenerateError(format!("no image returned (finishReason={finish_reason}, text={text})")))
    }
}
