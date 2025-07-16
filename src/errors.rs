use thiserror::Error;

/// Error types for the llm_affector library
#[derive(Error, Debug)]
pub enum LlmAffectorError {
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),
    
    #[error("JSON parsing failed: {0}")]
    JsonError(#[from] serde_json::Error),
    
    #[error("API key not found in environment variable LLM_API_KEY")]
    ApiKeyNotFound,
    
    #[error("API request failed with status: {status}, body: {body}")]
    ApiError { status: u16, body: String },
    
    #[error("Invalid response format: {0}")]
    InvalidResponse(String),
    
    #[error("Configuration error: {0}")]
    ConfigError(String),
}