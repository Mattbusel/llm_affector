use serde::{Deserialize, Serialize};

/// One claim the judge model considers unsupported or false.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Issue {
    /// The problematic claim, quoted or paraphrased from the input.
    pub claim: String,
    /// Why the judge thinks it is wrong or unsupported.
    pub explanation: String,
}

/// Result of a hallucination check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// The judge found no unsupported claims. This is not proof the text is
    /// true: the judge has no sources, only its own knowledge.
    Pass,
    /// The judge found problems, listed here (possibly empty if the model
    /// said FAIL without listing any).
    Fail(Vec<Issue>),
}

/// Result of a code critique.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CritiqueReport {
    /// Potential bugs, panics or security problems.
    pub risks: Vec<String>,
    /// Code quality and style suggestions.
    pub improvements: Vec<String>,
    /// Test scenarios that should be added.
    pub missing_tests: Vec<String>,
}

/// The JSON shape the hallucination prompt asks for.
#[derive(Deserialize)]
pub(crate) struct HallucinationResponse {
    pub verdict: String,
    #[serde(default)]
    pub issues: Option<Vec<Issue>>,
}

/// OpenAI Chat Completions request.
#[derive(Serialize)]
pub(crate) struct OpenAIRequest<'a> {
    pub model: &'a str,
    pub messages: Vec<OpenAIMessage>,
    pub temperature: f32,
    pub max_tokens: u32,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct OpenAIMessage {
    pub role: String,
    #[serde(default)]
    pub content: Option<String>,
}

/// OpenAI Chat Completions response (only the parts we read).
#[derive(Deserialize)]
pub(crate) struct OpenAIResponse {
    pub choices: Vec<OpenAIChoice>,
}

#[derive(Deserialize)]
pub(crate) struct OpenAIChoice {
    pub message: OpenAIMessage,
}
