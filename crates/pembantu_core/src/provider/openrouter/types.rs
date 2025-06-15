use serde::{Deserialize, Serialize};


#[derive(Clone)]
pub struct OpenRouterAPI {
    pub api_key: String,
    pub client: reqwest::Client,
    pub model: String,
}

impl OpenRouterAPI {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            api_key,
            client: reqwest::Client::new(),
            model
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all="lowercase")]
pub enum Role {
    System,
    User
}

#[derive(Serialize)] 
pub struct Message {
    pub role: Role,
    pub content: String,
}

#[derive(Serialize)]
pub struct CompletionsRequest {
    pub messages: Vec<Message>,
    pub model: String
}


#[derive(Deserialize, Debug)]
pub struct CompletionsResponse {
    pub id: String,
    pub choices: Vec<NonStreamingChoice>,
    pub created: i32,
    pub model: String,
}


#[derive(Deserialize, Debug, Clone)]
pub struct ResponseMessage {
    pub content: Option<String>,
    pub role: String,
}

#[derive(Deserialize, Debug)]
pub struct NonStreamingChoice {
    pub finish_reason: Option<String>, // Depends on the model. pub Ex: 'stop' | 'length' | 'content_filter' | 'tool_calls' | 'function_call'
    pub message: ResponseMessage
}