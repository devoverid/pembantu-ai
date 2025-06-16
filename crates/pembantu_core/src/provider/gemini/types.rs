use serde::{Deserialize, Serialize};


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
            client: reqwest::Client::new(),
            model
        }
    }
}

#[derive(Serialize)]
pub struct CompletionsRequest {
    pub model: String,
    pub body: GenerateContent
}

#[derive(Serialize)]
pub struct GenerateContent {
    pub contents: Vec<Content>,
    #[serde(skip_serializing_if = "Option::is_none", rename="generationConfig")]
    pub generation_config: Option<GenerationConfig>,
}

#[derive(Serialize)]
pub struct GenerationConfig {
    pub response_modalities: Vec<Modality>,
}

#[derive(Serialize)]
#[serde(rename_all="SCREAMING_SNAKE_CASE")]
pub enum Modality {
    ModalityUnspecified,
    Text,
    Image,
    Video
}
#[derive(Serialize)]
pub struct Content {
    pub parts: Vec<Part>,
    pub role: Role
}

#[derive(Serialize)]
pub struct Part {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>
}

impl Part {
    pub fn text(text: &str) -> Self {
        Self {
            text: Some(text.into())
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all="lowercase")]
pub enum Role {
    User,
    Model
}

#[derive(Deserialize, Debug)]
pub struct GenerateContentResponse {
    pub candidates: Vec<Candidate>,
    #[serde(rename = "usageMetadata")]
    pub usage_metadata: UsageMetadata,
    #[serde(rename = "modelVersion")]
    pub model_version: String,
    #[serde(rename = "responseId")]
    pub response_id: String,
}

#[derive(Deserialize, Debug)]
pub struct Candidate {
    pub content: ContentResponse,
    #[serde(rename = "finishReason")]
    pub finish_reason: String,
    #[serde(rename = "avgLogprobs", skip_serializing_if = "Option::is_none")]
    pub avg_logprobs: Option<f64>,
    pub index: i32,
}

#[derive(Deserialize, Debug)]
pub struct ContentResponse {
    pub parts: Vec<ContentPart>,
    pub role: String,
}

#[derive(Deserialize, Debug)]
pub struct ContentPart {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "inlineData")]
    pub inline_data: Option<InlineData>,
}

#[derive(Deserialize, Debug)]
pub struct InlineData {
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    pub data: String,
}

#[derive(Deserialize, Debug)]
pub struct UsageMetadata {
    #[serde(rename = "promptTokenCount")]
    pub prompt_token_count: i32,
    #[serde(rename = "candidatesTokenCount")]
    pub candidates_token_count: i32,
    #[serde(rename = "totalTokenCount")]
    pub total_token_count: i32,
    #[serde(rename = "promptTokensDetails")]
    pub prompt_tokens_details: Vec<TokenDetail>,
    #[serde(rename = "candidatesTokensDetails")]
    pub candidates_tokens_details: Vec<TokenDetail>,
}

#[derive(Deserialize, Debug)]
pub struct TokenDetail {
    pub modality: String,
    #[serde(rename = "tokenCount")]
    pub token_count: i32,
}