use serde::{Deserialize, Serialize};

/// Represents an issue found during hallucination detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    pub claim: String,
    pub explanation: String,
}

/// Result of hallucination detection
#[derive(Debug, Clone)]
pub enum Verdict {
    Pass,
    Fail(Vec<Issue>),
}

/// Report from code critique analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CritiqueReport {
    pub risks: Vec<String>,
    pub improvements: Vec<String>,
    pub missing_tests: Vec<String>,
}

/// Internal struct for parsing hallucination detection response
#[derive(Deserialize)]
pub(crate) struct HallucinationResponse {
    pub verdict: String,
    pub issues: Option<Vec<Issue>>,
}

/// OpenAI API request structure
#[derive(Serialize)]
pub(crate) struct OpenAIRequest {
    pub model: String,
    pub messages: Vec<OpenAIMessage>,
    pub temperature: f32,
    pub max_tokens: u32,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct OpenAIMessage {
    pub role: String,
    pub content: String,
}

/// OpenAI API response structure
#[derive(Deserialize)]
pub(crate) struct OpenAIResponse {
    pub choices: Vec<OpenAIChoice>,
}

#[derive(Deserialize)]
pub(crate) struct OpenAIChoice {
    pub message: OpenAIMessage,
}