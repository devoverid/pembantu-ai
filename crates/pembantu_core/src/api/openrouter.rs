use async_trait::async_trait;
use reqwest;
use crate::bot::{Bot};
use serde::{Serialize, Deserialize};
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
    content: String
}

#[derive(Serialize)]
struct CompletionsRequest {
    messages: Vec<Message>,
}


#[derive(Deserialize, Debug)]
struct CompletionsResponse {
    pub id: String,
    pub choices: NonStreamingChoice,
    pub created: i32,
    pub model: String,
    pub object: String
}


#[derive(Deserialize, Debug)]
struct ResponseMessage {
    pub content: Option<String>,
    pub role: String,
    pub tool_calls: Option<Vec<ToolCall>>,
    pub function_call: Option<Vec<FunctionCall>>
}


#[derive(Deserialize, Debug)]
struct NonStreamingChoice {
    pub finish_reason: Option<String>, // Depends on the model. pub Ex: 'stop' | 'length' | 'content_filter' | 'tool_calls' | 'function_call'
    pub message: ResponseMessage
}


#[derive(Deserialize, Debug)]
struct FunctionCall {
    name: String,
    arguments: String,
}
  
#[derive(Deserialize, Debug)]
struct ToolCall {
    id: String,
    function: FunctionCall
}

#[async_trait]
impl Bot for OpenRouterAPI {
    async fn generate(&self, message: String) -> String {
        let body = CompletionsRequest {
            messages: vec![
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
            .await
            .unwrap()
            .json::<CompletionsResponse>()
            .await
            .map_err(|f| {
                println!("error {:?}", f);
                f
            });
        println!("{:?}",response);
        // response.choices.message.content.unwrap_or("".into())
        "".into()
    }
}
