use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub teloxide_token: secrecy::SecretString,
    pub openrouter_api_key: secrecy::SecretString,
    pub gemini_api_key: secrecy::SecretString,
    pub bot_username: String,
    pub provider_text_generation: Option<String>,
    pub provider_image_generation: Option<String>,
    pub model_image_generation: Option<String>,
    pub model_text_generation: Option<String>,
}