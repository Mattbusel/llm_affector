use crate::errors::LlmAffectorError;
use crate::types::{OpenAIRequest, OpenAIResponse, OpenAIMessage};
use reqwest::Client;
use std::env;

/// HTTP client wrapper for LLM API interactions
pub struct LlmClient {
    client: Client,
    api_key: String,
    base_url: String,
}

impl LlmClient {
    /// Create a new LLM client with API key from environment
    pub fn new() -> Result<Self, LlmAffectorError> {
        // Load .env file if it exists
        dotenv::dotenv().ok();
        
        let api_key = env::var("LLM_API_KEY")
            .map_err(|_| LlmAffectorError::ApiKeyNotFound)?;
        
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        Ok(Self {
            client,
            api_key,
            base_url: "https://api.openai.com/v1".to_string(),
        })
    }

    /// Send a prompt to the LLM and get the response
    pub async fn send_prompt(&self, prompt: &str) -> Result<String, LlmAffectorError> {
        let request = OpenAIRequest {
            model: "gpt-4".to_string(),
            messages: vec![OpenAIMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            temperature: 0.1,
            max_tokens: 2048,
        };

        let response = self
            .client
            .post(&format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
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