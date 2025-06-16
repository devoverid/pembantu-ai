
use async_trait::async_trait;
use crate::{error::PembantuError, prompt, provider::{openrouter::types::{self, CompletionsResponse, Message, OpenRouterAPI, Role}, TextGenerationProvider}};

#[async_trait]
impl TextGenerationProvider for OpenRouterAPI {
    async fn generate(&self, message: String) -> Result<String, PembantuError> {
        let body = types::CompletionsRequest {
            model: self.model.clone(),
            messages: vec![
                Message {
                    content: prompt::get_prompt(),
                    role: Role::System,
                },
                Message {
                    content: message,
                    role: Role::User
                }
            ]
        };
        let response = self.client.post("https://openrouter.ai/api/v1/chat/completions")
            .header("Authorization", format!("Bearer {}",self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        let response_str = response.text().await?;
        let response_json: CompletionsResponse = serde_json::from_str::<CompletionsResponse>(&response_str).unwrap();
        
        Ok(response_json.choices[0].message.content.clone().unwrap())
    }
    
}
