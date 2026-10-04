use thiserror::Error;

/// Everything that can go wrong in a check or a critique.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum LlmAffectorError {
    /// The request could not be sent or the response could not be read
    /// (connection refused, DNS, TLS, timeout).
    #[error("HTTP request failed: {}", error_chain(.0))]
    HttpError(#[from] reqwest::Error),

    /// A JSON value could not be (de)serialised.
    #[error("JSON parsing failed: {0}")]
    JsonError(#[from] serde_json::Error),

    /// No API key, and the endpoint is the official OpenAI one, which needs a key.
    #[error("API key not found: set LLM_API_KEY (local servers such as Ollama need no key, set LLM_BASE_URL instead)")]
    ApiKeyNotFound,

    /// The server answered with a non-2xx status. Retryable statuses (408,
    /// 429, 5xx) are only reported after the retries ran out.
    #[error("API request failed with status: {status}, body: {body}")]
    ApiError {
        /// HTTP status code.
        status: u16,
        /// Response body, as text.
        body: String,
    },

    /// The judge model answered, but not with the JSON shape that was asked for.
    #[error("Invalid response format: {0}")]
    InvalidResponse(String),

    /// A setting (environment variable or builder value) is not usable.
    #[error("Configuration error: {0}")]
    ConfigError(String),
}

/// The error and its causes joined with ": ", so "error sending request"
/// comes with the reason (connection refused, DNS, TLS).
fn error_chain(e: &dyn std::error::Error) -> String {
    let mut out = e.to_string();
    let mut src = e.source();
    while let Some(s) = src {
        let msg = s.to_string();
        if !out.contains(&msg) {
            out.push_str(": ");
            out.push_str(&msg);
        }
        src = s.source();
    }
    out
}
