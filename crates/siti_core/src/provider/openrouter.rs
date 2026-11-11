use std::time::Duration;

use base64::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{agent::ImageInput, error::SitiError};

/// OpenRouter's dedicated Image API (`POST /api/v1/images`), used for image generation.
#[derive(Clone)]
pub struct OpenRouterImageAPI {
    api_key: String,
    client: reqwest::Client,
    model: String,
}

#[derive(Serialize)]
struct ImageRequest<'a> {
    model: &'a str,
    prompt: &'a str,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    input_references: Vec<InputReference>,
}

#[derive(Serialize)]
struct InputReference {
    r#type: &'static str,
    image_url: ImageUrl,
}

#[derive(Serialize)]
struct ImageUrl {
    url: String,
}

#[derive(Deserialize)]
struct ImageResponse {
    #[serde(default)]
    data: Vec<ImageData>,
}

#[derive(Deserialize)]
struct ImageData {
    b64_json: String,
}

impl OpenRouterImageAPI {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            api_key,
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(180))
                .build()
                .expect("failed to build HTTP client"),
            model,
        }
    }

    pub async fn generate_image(&self, prompt: &str, images: &[ImageInput]) -> Result<Vec<u8>, SitiError> {
        let input_references = images
            .iter()
            .map(|image| InputReference {
                r#type: "image_url",
                image_url: ImageUrl { url: format!("data:{};base64,{}", image.mime, BASE64_STANDARD.encode(&image.bytes)) },
            })
            .collect();
        let response = self
            .client
            .post("https://openrouter.ai/api/v1/images")
            .bearer_auth(&self.api_key)
            .json(&ImageRequest { model: &self.model, prompt, input_references })
            .send()
            .await?;
        let status = response.status();
        let body = response.text().await?;
        if !status.is_success() {
            return Err(SitiError::Generate(format!("OpenRouter image API {status}: {body}")));
        }
        let response: ImageResponse = serde_json::from_str(&body)
            .map_err(|e| SitiError::Generate(format!("invalid OpenRouter image response: {e}")))?;
        let image = response
            .data
            .into_iter()
            .next()
            .ok_or_else(|| SitiError::Generate("OpenRouter returned no image".into()))?;
        BASE64_STANDARD.decode(image.b64_json).map_err(|_| SitiError::Base64DecodeError)
    }
}
