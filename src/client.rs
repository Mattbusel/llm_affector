use std::time::Duration;

use backon::{ExponentialBuilder, Retryable};
use reqwest::header::{HeaderMap, RETRY_AFTER};
use reqwest::Client;

use crate::errors::LlmAffectorError;
use crate::types::{CritiqueReport, OpenAIMessage, OpenAIRequest, OpenAIResponse, Verdict};

/// Default model when `LLM_MODEL` is not set.
pub const DEFAULT_MODEL: &str = "gpt-4o-mini";
/// Default OpenAI-compatible endpoint when `LLM_BASE_URL` is not set.
pub const DEFAULT_BASE_URL: &str = "https://api.openai.com/v1";
/// Default request timeout in seconds.
pub const DEFAULT_TIMEOUT_SECS: u64 = 30;
/// Default number of retries after the first attempt.
pub const DEFAULT_MAX_RETRIES: usize = 2;
/// Longest wait honoured from a `Retry-After` header.
const MAX_RETRY_AFTER: Duration = Duration::from_secs(60);

/// A client for any OpenAI-compatible Chat Completions endpoint: OpenAI,
/// Ollama, OpenRouter, Gemini's OpenAI endpoint, vLLM, LM Studio, a proxy.
///
/// Requests that fail with HTTP 408, 429 or 5xx, or that time out or cannot
/// connect, are retried with exponential backoff and jitter. A `Retry-After`
/// header (seconds or an HTTP date) sets the wait instead, up to 60 seconds.
///
/// ```no_run
/// # async fn demo() -> Result<(), llm_affector::LlmAffectorError> {
/// use llm_affector::{LlmClient, Verdict};
///
/// // A local Ollama model: no API key needed.
/// let judge = LlmClient::builder()
///     .base_url("http://localhost:11434/v1")
///     .model("qwen2.5:7b")
///     .build()?;
/// if let Verdict::Fail(issues) = judge.detect_hallucination("The Moon is 3 km away.").await? {
///     for i in issues {
///         println!("{}: {}", i.claim, i.explanation);
///     }
/// }
/// # Ok(()) }
/// ```
#[derive(Debug, Clone)]
pub struct LlmClient {
    client: Client,
    api_key: Option<String>,
    base_url: String,
    model: String,
    max_retries: usize,
    json_mode: bool,
}

/// Builder for [`LlmClient`]; start with [`LlmClient::builder`].
#[derive(Debug, Clone)]
pub struct LlmClientBuilder {
    api_key: Option<String>,
    base_url: String,
    model: String,
    timeout: Duration,
    max_retries: usize,
    json_mode: bool,
}

impl Default for LlmClientBuilder {
    fn default() -> Self {
        Self {
            api_key: None,
            base_url: DEFAULT_BASE_URL.to_string(),
            model: DEFAULT_MODEL.to_string(),
            timeout: Duration::from_secs(DEFAULT_TIMEOUT_SECS),
            max_retries: DEFAULT_MAX_RETRIES,
            json_mode: false,
        }
    }
}

impl LlmClientBuilder {
    /// API key, sent as `Authorization: Bearer <key>`. Optional for
    /// endpoints other than the default OpenAI one.
    pub fn api_key(mut self, key: impl Into<String>) -> Self {
        let key = key.into();
        self.api_key = Some(key.trim().to_string()).filter(|k| !k.is_empty());
        self
    }

    /// Base URL, the part before `/chat/completions`, e.g.
    /// `http://localhost:11434/v1` for Ollama or
    /// `https://openrouter.ai/api/v1` for OpenRouter.
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into().trim().trim_end_matches('/').to_string();
        self
    }

    /// Model name, e.g. `gpt-4o-mini` or `qwen2.5:7b`.
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into().trim().to_string();
        self
    }

    /// Timeout for each attempt.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Retries after the first attempt (0 turns retrying off).
    pub fn max_retries(mut self, n: usize) -> Self {
        self.max_retries = n;
        self
    }

    /// Ask the server for a JSON reply (`"response_format": {"type":
    /// "json_object"}`), which OpenAI, Ollama, vLLM, Gemini's OpenAI endpoint
    /// and most OpenRouter models support. Off by default because servers
    /// that do not know the field may reject the request.
    pub fn json_mode(mut self, on: bool) -> Self {
        self.json_mode = on;
        self
    }

    /// Build the client.
    ///
    /// Fails with [`LlmAffectorError::ApiKeyNotFound`] when no key was given
    /// and the base URL is the official OpenAI one, and with
    /// [`LlmAffectorError::ConfigError`] when the base URL or model is empty.
    pub fn build(self) -> Result<LlmClient, LlmAffectorError> {
        if self.base_url.is_empty() {
            return Err(LlmAffectorError::ConfigError("base URL is empty".into()));
        }
        if self.model.is_empty() {
            return Err(LlmAffectorError::ConfigError("model is empty".into()));
        }
        if self.api_key.is_none() && self.base_url == DEFAULT_BASE_URL {
            return Err(LlmAffectorError::ApiKeyNotFound);
        }
        let client = Client::builder().timeout(self.timeout).build()?;
        Ok(LlmClient {
            client,
            api_key: self.api_key,
            base_url: self.base_url,
            model: self.model,
            max_retries: self.max_retries,
            json_mode: self.json_mode,
        })
    }
}

fn env_nonempty(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

/// One failed attempt, with the server's requested wait if it sent one.
struct Attempt {
    error: LlmAffectorError,
    retry_after: Option<Duration>,
}

impl Attempt {
    fn retryable(&self) -> bool {
        match &self.error {
            LlmAffectorError::ApiError { status, .. } => {
                matches!(status, 408 | 429 | 500 | 502 | 503 | 504 | 529)
            }
            LlmAffectorError::HttpError(e) => e.is_timeout() || e.is_connect(),
            _ => false,
        }
    }
}

/// Parse `Retry-After` as delay-seconds or an HTTP date.
fn retry_after(headers: &HeaderMap) -> Option<Duration> {
    let v = headers.get(RETRY_AFTER)?.to_str().ok()?.trim();
    let d = match v.parse::<u64>() {
        Ok(secs) => Duration::from_secs(secs),
        Err(_) => {
            let when = httpdate::parse_http_date(v).ok()?;
            when.duration_since(std::time::SystemTime::now())
                .unwrap_or(Duration::ZERO)
        }
    };
    Some(d.min(MAX_RETRY_AFTER))
}

impl LlmClient {
    /// Create a client configured from the environment (and a `.env` file in
    /// the current directory, if there is one).
    ///
    /// * `LLM_API_KEY`: API key. Required for the default OpenAI endpoint,
    ///   optional when `LLM_BASE_URL` points somewhere else (Ollama, vLLM).
    /// * `LLM_BASE_URL`: OpenAI-compatible endpoint, default `https://api.openai.com/v1`.
    /// * `LLM_MODEL`: model name, default `gpt-4o-mini`.
    /// * `LLM_TIMEOUT_SECONDS`: timeout per attempt, default 30.
    /// * `LLM_MAX_RETRIES`: retries after the first attempt, default 2.
    /// * `LLM_JSON_MODE`: `1`/`true` to request a JSON reply (see
    ///   [`LlmClientBuilder::json_mode`]).
    pub fn new() -> Result<Self, LlmAffectorError> {
        dotenvy::dotenv().ok();

        let mut b = LlmClientBuilder::default();
        if let Some(k) = env_nonempty("LLM_API_KEY") {
            b = b.api_key(k);
        }
        if let Some(u) = env_nonempty("LLM_BASE_URL") {
            b = b.base_url(u);
        }
        if let Some(m) = env_nonempty("LLM_MODEL") {
            b = b.model(m);
        }
        if let Some(t) = env_nonempty("LLM_TIMEOUT_SECONDS") {
            let secs = t.parse::<u64>().map_err(|_| {
                LlmAffectorError::ConfigError(format!("LLM_TIMEOUT_SECONDS is not a number: {t}"))
            })?;
            b = b.timeout(Duration::from_secs(secs));
        }
        if let Some(j) = env_nonempty("LLM_JSON_MODE") {
            b = b.json_mode(matches!(j.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"));
        }
        if let Some(r) = env_nonempty("LLM_MAX_RETRIES") {
            let n = r.parse::<usize>().map_err(|_| {
                LlmAffectorError::ConfigError(format!("LLM_MAX_RETRIES is not a number: {r}"))
            })?;
            b = b.max_retries(n);
        }
        b.build()
    }

    /// Start configuring a client in code instead of from the environment.
    pub fn builder() -> LlmClientBuilder {
        LlmClientBuilder::default()
    }

    /// The model this client sends requests to.
    pub fn model(&self) -> &str {
        &self.model
    }

    /// The base URL this client sends requests to.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Send one user message and return the reply text, retrying transient
    /// failures (see the type-level docs).
    pub async fn send_prompt(&self, prompt: &str) -> Result<String, LlmAffectorError> {
        let backoff = ExponentialBuilder::default()
            .with_min_delay(Duration::from_millis(500))
            .with_max_delay(Duration::from_secs(20))
            .with_max_times(self.max_retries)
            .with_jitter();
        (|| self.attempt(prompt))
            .retry(backoff)
            .when(Attempt::retryable)
            .adjust(|a: &Attempt, dur| match (dur, a.retry_after) {
                // Out of retries: stop, whatever the server asked for.
                (None, _) => None,
                (Some(_), Some(wait)) => Some(wait),
                (d, None) => d,
            })
            .await
            .map_err(|a| a.error)
    }

    async fn attempt(&self, prompt: &str) -> Result<String, Attempt> {
        let plain = |error: LlmAffectorError| Attempt {
            error,
            retry_after: None,
        };
        let request = OpenAIRequest {
            model: &self.model,
            messages: vec![OpenAIMessage {
                role: "user".to_string(),
                content: Some(prompt.to_string()),
            }],
            temperature: 0.1,
            max_tokens: 2048,
        };
        let mut body = serde_json::to_value(&request).map_err(|e| plain(e.into()))?;
        if self.json_mode {
            body["response_format"] = serde_json::json!({"type": "json_object"});
        }
        let mut req = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .json(&body);
        if let Some(key) = &self.api_key {
            req = req.bearer_auth(key);
        }
        let response = req.send().await.map_err(|e| plain(e.into()))?;

        let status = response.status();
        if !status.is_success() {
            let wait = retry_after(response.headers());
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(Attempt {
                error: LlmAffectorError::ApiError {
                    status: status.as_u16(),
                    body,
                },
                retry_after: wait,
            });
        }

        let api_response: OpenAIResponse = response.json().await.map_err(|e| plain(e.into()))?;
        api_response
            .choices
            .into_iter()
            .next()
            .and_then(|c| c.message.content)
            .ok_or_else(|| {
                plain(LlmAffectorError::InvalidResponse(
                    "No message content in response".to_string(),
                ))
            })
    }

    /// Fact-check `text` with this client. See [`crate::detect_hallucination`].
    pub async fn detect_hallucination(&self, text: &str) -> Result<Verdict, LlmAffectorError> {
        let reply = self
            .send_prompt(&crate::hallucination::build_hallucination_prompt(text))
            .await?;
        crate::hallucination::parse_verdict(&reply)
    }

    /// Check `text` against `sources` (retrieved documents, the context the
    /// first model was given). See
    /// [`crate::judge::check_against_sources_with`].
    pub async fn check_against_sources(
        &self,
        text: &str,
        sources: &[&str],
    ) -> Result<Verdict, LlmAffectorError> {
        crate::judge::check_against_sources_with(self, text, sources).await
    }

    /// Review Rust `code` with this client. See [`crate::critique_code`].
    pub async fn critique_code(&self, code: &str) -> Result<CritiqueReport, LlmAffectorError> {
        let reply = self
            .send_prompt(&crate::critique::build_critique_prompt(code))
            .await?;
        crate::critique::parse_critique(&reply)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_required_only_for_the_official_endpoint() {
        assert!(matches!(
            LlmClient::builder().build(),
            Err(LlmAffectorError::ApiKeyNotFound)
        ));
        assert!(LlmClient::builder().api_key("sk").build().is_ok());
        let local = LlmClient::builder()
            .base_url("http://localhost:11434/v1/")
            .build()
            .unwrap();
        assert_eq!(local.base_url(), "http://localhost:11434/v1");
        assert!(LlmClient::builder().api_key("  ").build().is_err());
        assert!(LlmClient::builder().api_key("k").model(" ").build().is_err());
    }

    #[test]
    fn retry_after_seconds_and_http_date() {
        let mut h = HeaderMap::new();
        h.insert(RETRY_AFTER, "3".parse().unwrap());
        assert_eq!(retry_after(&h), Some(Duration::from_secs(3)));
        h.insert(RETRY_AFTER, "86400".parse().unwrap());
        assert_eq!(retry_after(&h), Some(MAX_RETRY_AFTER));
        let past = httpdate::fmt_http_date(std::time::SystemTime::UNIX_EPOCH);
        h.insert(RETRY_AFTER, past.parse().unwrap());
        assert_eq!(retry_after(&h), Some(Duration::ZERO));
        h.insert(RETRY_AFTER, "soon".parse().unwrap());
        assert_eq!(retry_after(&h), None);
    }
}
