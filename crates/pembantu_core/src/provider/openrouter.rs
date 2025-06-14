
use async_trait::async_trait;
use dyn_clone::DynClone;
use reqwest;
use crate::{provider::TextGenerationProvider, error::PembantuError, prompt};
use serde::{Serialize, Deserialize};

#[derive(Clone)]
pub struct OpenRouterAPI {
    api_key: String,
    client: reqwest::Client,
    model: String,
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
struct Message {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct CompletionsRequest {
    messages: Vec<Message>,
    model: String
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

#[async_trait]
impl TextGenerationProvider for OpenRouterAPI {
    async fn generate(&self, message: String) -> Result<String, PembantuError> {
        let body = CompletionsRequest {
            model: self.model.clone(),
            messages: vec![
                Message {
                    content: prompt::get_prompt(),
                    role: "system".into()
                },
                Message {
                    content: message,
                    role: "user".into()
                }
            ]
        };
        let response = self.client.post("https://openrouter.ai/api/v1/chat/completions")
            .header("Authorization", format!("Bearer {}",self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;
        
        // let response_text = response.text().await?;
        // dbg!(response_text);

        let response_json = response.json::<CompletionsResponse>().await?;
        Ok(response_json.choices[0].message.content.clone().unwrap())
    }
    
}
