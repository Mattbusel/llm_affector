//! Bring your own model client.
//!
//! The checks only need "send this prompt, give me the reply text". [`Judge`]
//! is that one method; [`crate::LlmClient`] implements it, and so can a
//! wrapper around async-openai, genai, rig, a local llama.cpp binding or a
//! test double. Then call [`detect_hallucination_with`],
//! [`check_against_sources_with`] or [`critique_code_with`].
//!
//! ```
//! use llm_affector::judge::{detect_hallucination_with, Judge};
//! use llm_affector::{LlmAffectorError, Verdict};
//!
//! /// A judge that always answers the same thing (a test double).
//! struct Canned(&'static str);
//!
//! impl Judge for Canned {
//!     async fn complete(&self, _prompt: &str) -> Result<String, LlmAffectorError> {
//!         Ok(self.0.to_string())
//!     }
//! }
//!
//! # #[tokio::main(flavor = "current_thread")]
//! # async fn main() -> Result<(), LlmAffectorError> {
//! let judge = Canned(r#"{"verdict": "FAIL", "issues": [{"claim": "1999", "explanation": "It opened in 1889."}]}"#);
//! let v = detect_hallucination_with(&judge, "The Eiffel Tower opened in 1999.").await?;
//! assert!(matches!(v, Verdict::Fail(ref i) if i[0].claim == "1999"));
//! # Ok(())
//! # }
//! ```

use std::future::Future;

use crate::errors::LlmAffectorError;
use crate::types::{CritiqueReport, Verdict};

/// Anything that can answer a prompt with text.
pub trait Judge {
    /// Send `prompt` as a user message and return the reply text.
    fn complete(&self, prompt: &str) -> impl Future<Output = Result<String, LlmAffectorError>> + Send;
}

impl Judge for crate::LlmClient {
    fn complete(&self, prompt: &str) -> impl Future<Output = Result<String, LlmAffectorError>> + Send {
        self.send_prompt(prompt)
    }
}

/// Fact-check `text` with any [`Judge`], from the judge's own knowledge.
pub async fn detect_hallucination_with(
    judge: &impl Judge,
    text: &str,
) -> Result<Verdict, LlmAffectorError> {
    let reply = judge
        .complete(&crate::hallucination::build_hallucination_prompt(text))
        .await?;
    crate::hallucination::parse_verdict(&reply)
}

/// Check `text` against `sources` (retrieved documents, the context given
/// to the first model) with any [`Judge`]: every claim not supported by the
/// sources is an issue, even if it happens to be true.
///
/// # Errors
///
/// [`LlmAffectorError::ConfigError`] when `sources` is empty.
pub async fn check_against_sources_with(
    judge: &impl Judge,
    text: &str,
    sources: &[&str],
) -> Result<Verdict, LlmAffectorError> {
    if sources.is_empty() {
        return Err(LlmAffectorError::ConfigError(
            "at least one source is needed for a grounded check".into(),
        ));
    }
    let reply = judge
        .complete(&crate::hallucination::build_grounded_prompt(text, sources))
        .await?;
    crate::hallucination::parse_verdict(&reply)
}

/// Review Rust `code` with any [`Judge`].
pub async fn critique_code_with(
    judge: &impl Judge,
    code: &str,
) -> Result<CritiqueReport, LlmAffectorError> {
    let reply = judge
        .complete(&crate::critique::build_critique_prompt(code))
        .await?;
    crate::critique::parse_critique(&reply)
}
