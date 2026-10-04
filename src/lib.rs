#![doc = include_str!("../README.md")]
#![deny(missing_docs)]

mod client;
mod critique;
mod errors;
mod hallucination;
mod json;
pub mod judge;
mod types;

pub use client::{
    LlmClient, LlmClientBuilder, DEFAULT_BASE_URL, DEFAULT_MAX_RETRIES, DEFAULT_MODEL,
    DEFAULT_TIMEOUT_SECS,
};
pub use critique::critique_code;
pub use errors::LlmAffectorError;
pub use hallucination::{check_against_sources, detect_hallucination};
pub use types::{CritiqueReport, Issue, Verdict};

/// `Result` with [`LlmAffectorError`].
pub type Result<T> = std::result::Result<T, LlmAffectorError>;
