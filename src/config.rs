use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub bind_addr: String,
    pub gemini_api_key: Option<String>,
    pub gemini_model: String,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            bind_addr: env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:3000".to_string()),
            gemini_api_key: env::var("GEMINI_API_KEY").ok().filter(|value| !value.is_empty()),
            gemini_model: env::var("GEMINI_MODEL")
                .unwrap_or_else(|_| "gemini-3-flash-preview".to_string()),
        }
    }
}
