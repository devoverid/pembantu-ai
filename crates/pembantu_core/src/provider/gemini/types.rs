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
}

#[derive(Serialize)]
pub struct Content {
    pub part: Vec<Part>,
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

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GenerateContentResponse {
    pub candidates: Candidate,
}

#[derive(Deserialize)]
pub struct Candidate {
    pub content: ContentResponse,
    #[serde(rename = "tokenCount")]
    pub token_count: String,
}

#[derive(Deserialize)]
pub struct ContentResponse {
    pub parts: Vec<ContentPart>
}

#[derive(Deserialize)]
pub struct ContentPart {
    pub thought: bool,
    pub data: String // assume the content is text
}