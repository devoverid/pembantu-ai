use secrecy::SecretString;
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub provider_text_generation: Option<String>,
    pub provider_image_generation: Option<String>,
    pub openrouter_api_key: SecretString,
    pub gemini_api_key: SecretString,
    pub model_text_generation: Option<String>,
    pub model_image_generation: Option<String>,
}
