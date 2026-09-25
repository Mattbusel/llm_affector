use crate::errors::LlmAffectorError;
use crate::types::{OpenAIMessage, OpenAIRequest, OpenAIResponse};
use reqwest::Client;
use std::env;

/// HTTP client wrapper for LLM API interactions
pub struct LlmClient {
    client: Client,
    api_key: String,
    base_url: String,
    model: String,
}

/// Default model when `LLM_MODEL` is not set.
pub const DEFAULT_MODEL: &str = "gpt-4o-mini";
/// Default OpenAI-compatible endpoint when `LLM_BASE_URL` is not set.
pub const DEFAULT_BASE_URL: &str = "https://api.openai.com/v1";

fn env_nonempty(name: &str) -> Option<String> {
    env::var(name)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

impl LlmClient {
    /// Create a new LLM client configured from the environment (or a `.env` file).
    ///
    /// * `LLM_API_KEY` (required): API key sent as a Bearer token.
    /// * `LLM_BASE_URL` (optional): OpenAI-compatible endpoint, default `https://api.openai.com/v1`.
    /// * `LLM_MODEL` (optional): model name, default `gpt-4o-mini`.
    /// * `LLM_TIMEOUT_SECONDS` (optional): request timeout, default 30.
    pub fn new() -> Result<Self, LlmAffectorError> {
        // Load .env file if it exists
        dotenv::dotenv().ok();

        let api_key = env_nonempty("LLM_API_KEY").ok_or(LlmAffectorError::ApiKeyNotFound)?;
        let base_url = env_nonempty("LLM_BASE_URL")
            .unwrap_or_else(|| DEFAULT_BASE_URL.to_string())
            .trim_end_matches('/')
            .to_string();
        let model = env_nonempty("LLM_MODEL").unwrap_or_else(|| DEFAULT_MODEL.to_string());
        let timeout = env_nonempty("LLM_TIMEOUT_SECONDS")
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(30);

        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(timeout))
            .build()?;

        Ok(Self {
            client,
            api_key,
            base_url,
            model,
        })
    }

    /// The model this client sends requests to.
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Send a prompt to the LLM and get the response
    pub async fn send_prompt(&self, prompt: &str) -> Result<String, LlmAffectorError> {
        let request = OpenAIRequest {
            model: self.model.clone(),
            messages: vec![OpenAIMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            temperature: 0.1,
            max_tokens: 2048,
        };

        let response = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(LlmAffectorError::ApiError { status, body });
        }

        let api_response: OpenAIResponse = response.json().await?;

        api_response
            .choices
            .first()
            .map(|choice| choice.message.content.clone())
            .ok_or_else(|| LlmAffectorError::InvalidResponse("No choices in response".to_string()))
    }
}
