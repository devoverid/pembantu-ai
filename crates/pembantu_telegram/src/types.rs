use secrecy::SecretString;
use serde::Deserialize;

/// Environment configuration (read with `envy`, so `FOO_BAR` maps to `foo_bar`).
#[derive(Deserialize)]
pub struct Config {
    pub teloxide_token: SecretString,
    /// `openrouter` or `gemini`.
    pub provider_text_generation: String,
    pub model_text_generation: String,
    /// `gemini`, or unset to disable image generation.
    pub provider_image_generation: Option<String>,
    pub model_image_generation: Option<String>,
    pub openrouter_api_key: Option<SecretString>,
    pub gemini_api_key: Option<SecretString>,
    #[serde(default = "default_timezone")]
    pub default_timezone: String,
}

fn default_timezone() -> String {
    "Asia/Jakarta".into()
}
