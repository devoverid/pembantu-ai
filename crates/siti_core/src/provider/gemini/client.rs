use base64::prelude::*;

use crate::{
    agent::ImageInput,
    error::SitiError,
    provider::gemini::types::{Content, GeminiAPI, GenerateContent, GenerateContentResponse, GenerationConfig, Modality, Part, Role},
};

impl GeminiAPI {
    pub async fn generate_image(&self, prompt: &str, images: &[ImageInput]) -> Result<Vec<u8>, SitiError> {
        let mut parts: Vec<Part> = images.iter().map(|image| Part::inline_data(&image.mime, &image.bytes)).collect();
        parts.push(Part::text(prompt));
        let body = GenerateContent {
            generation_config: Some(GenerationConfig {
                response_modalities: vec![Modality::Image, Modality::Text],
            }),
            contents: vec![Content {
                role: Role::User,
                parts,
            }],
        };
        let response = self
            .client
            .post(format!(
                "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent",
                self.model
            ))
            .header("x-goog-api-key", &self.api_key)
            .json(&body)
            .send()
            .await?;
        let status = response.status();
        let body = response.text().await?;
        if !status.is_success() {
            return Err(SitiError::Generate(format!("Gemini image API {status}: {body}")));
        }
        let response_json: GenerateContentResponse = serde_json::from_str(&body)
            .map_err(|e| SitiError::Generate(format!("invalid Gemini image response: {e}")))?;

        let candidate = response_json.candidates.first();
        let parts = candidate
            .and_then(|c| c.content.as_ref())
            .map(|c| c.parts.as_slice())
            .unwrap_or_default();

        if let Some(data) = parts.iter().find_map(|p| p.inline_data.as_ref()) {
            return BASE64_STANDARD.decode(&data.data).map_err(|_| SitiError::Base64DecodeError);
        }

        let finish_reason = candidate.and_then(|c| c.finish_reason.as_deref()).unwrap_or("none");
        let text = parts.iter().filter_map(|p| p.text.as_deref()).collect::<Vec<_>>().join(" ");
        Err(SitiError::Generate(format!(
            "no image returned (finishReason={finish_reason}, text={text})"
        )))
    }
}
