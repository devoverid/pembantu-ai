use async_trait::async_trait;
use reqwest;
use crate::{bot::{Bot}, error::PembantuError};
use serde::{Serialize, Deserialize};

#[derive(Clone)]
pub struct OpenRouterAPI {
    api_key: String,
    client: reqwest::Client
}

impl OpenRouterAPI {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            client: reqwest::Client::new()
        }
    }
}
#[derive(Serialize)] 
struct Message {
    role: String,
    content: String,
    prompt: String
}

#[derive(Serialize)]
struct CompletionsRequest {
    messages: Vec<Message>,
}


#[derive(Deserialize, Debug)]
pub struct CompletionsResponse {
    pub id: String,
    pub choices: Vec<NonStreamingChoice>,
    pub created: i32,
    pub model: String,
    pub object: String
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



#[async_trait]
impl Bot for OpenRouterAPI {
    async fn generate(&self, message: String) -> Result<String, PembantuError> {
        let body = CompletionsRequest {
            messages: vec![
                Message {
                    content: message,
                    prompt: "You are a helpful AI assistant that can help people answering their questions. 
                            If you are given an instruction or question in Bahasa Indonesia, reply it only in Bahasa Indonesia. 
                            If you are given an instruction or question in another language, reply it in that corresponding language.".into(),
                    role: "user".into()
                }
            ]
        };
        let response = self.client.post("https://openrouter.ai/api/v1/chat/completions")
            .header("Authorization", format!("Bearer {}",self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?
            .json::<CompletionsResponse>()
            // .text()
            .await
            .map_err(|f| {
                println!("error {:?}", f);
                f
            })?;
        Ok(response.choices[0].message.content.clone().unwrap())
    }
    
}
