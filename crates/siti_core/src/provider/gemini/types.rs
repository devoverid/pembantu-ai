use std::time::Duration;

use serde::{Deserialize, Serialize};

/// Native Gemini API client, used for image generation.
#[derive(Clone)]
pub struct GeminiAPI {
    pub api_key: String,
    pub client: reqwest::Client,
    pub model: String,
}

impl GeminiAPI {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            api_key,
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(120))
                .build()
                .expect("failed to build HTTP client"),
            model,
        }
    }
}

#[derive(Serialize)]
pub struct GenerateContent {
    pub contents: Vec<Content>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "generationConfig")]
    pub generation_config: Option<GenerationConfig>,
}

#[derive(Serialize)]
pub struct GenerationConfig {
    pub response_modalities: Vec<Modality>,
}

#[derive(Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Modality {
    Text,
    Image,
}

#[derive(Serialize)]
pub struct Content {
    pub parts: Vec<Part>,
    pub role: Role,
}

#[derive(Serialize, Debug)]
pub struct Part {
    pub text: String,
}

impl Part {
    pub fn text(text: &str) -> Self {
        Self { text: text.into() }
    }
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
}

#[derive(Deserialize, Debug)]
pub struct GenerateContentResponse {
    #[serde(default)]
    pub candidates: Vec<Candidate>,
}

#[derive(Deserialize, Debug)]
pub struct Candidate {
    pub content: Option<ContentResponse>,
    #[serde(rename = "finishReason")]
    pub finish_reason: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct ContentResponse {
    #[serde(default)]
    pub parts: Vec<ContentPart>,
}

#[derive(Deserialize, Debug)]
pub struct ContentPart {
    pub text: Option<String>,
    #[serde(rename = "inlineData")]
    pub inline_data: Option<InlineData>,
}

#[derive(Deserialize, Debug)]
pub struct InlineData {
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    pub data: String,
}
