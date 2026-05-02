//! Environment-backed configuration for the AnyweAR server.

use std::env;

/// Runtime configuration loaded from environment variables.
#[derive(Debug, Clone)]
pub struct Config {
    /// Socket address the HTTP server binds to.
    pub bind_addr: String,
    /// Optional Gemini API key; when absent the server uses the mock extractor.
    pub gemini_api_key: Option<String>,
    /// Gemini model name used for general event extraction.
    pub gemini_model: String,
}

impl Config {
    /// Builds configuration from process environment with local-development defaults.
    ///
    /// # Returns
    ///
    /// A [`Config`] populated from `BIND_ADDR`, `GEMINI_API_KEY`, and `GEMINI_MODEL`.
    pub fn from_env() -> Self {
        Self {
            bind_addr: env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:3000".to_string()),
            gemini_api_key: env::var("GEMINI_API_KEY")
                .ok()
                .filter(|value| !value.is_empty()),
            gemini_model: env::var("GEMINI_MODEL")
                .unwrap_or_else(|_| "gemini-3-flash-preview".to_string()),
        }
    }
}
