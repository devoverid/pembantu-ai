use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub teloxide_token: String,
    pub openrouter_api_key: String,
    pub gemini_api_key: String,
    pub bot_username: String,
    pub provider_text_generation: Option<String>,
    pub provider_image_generation: Option<String>,
    pub model_image_generation: Option<String>,
    pub model_text_generation: Option<String>,
}