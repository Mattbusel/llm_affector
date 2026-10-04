# Changelog

## 0.3.0 (2026-10-04)

- **rustls instead of OpenSSL.** reqwest 0.11 with default features pulled in native-tls (OpenSSL on Linux); now reqwest 0.12 with rustls only.
- **Works without an API key on local servers.** `LLM_API_KEY` is only required for the default OpenAI endpoint, so `LLM_BASE_URL=http://localhost:11434/v1` (Ollama), vLLM or LM Studio work with no key. README lists base URLs for OpenAI, Ollama, OpenRouter, Gemini, vLLM and LM Studio.
- **Retries with [backon](https://crates.io/crates/backon).** HTTP 408, 429 and 5xx, timeouts and connection failures are retried with exponential backoff and jitter; `Retry-After` (seconds or HTTP date, parsed with httpdate) sets the wait, capped at 60 s. `LLM_MAX_RETRIES` (default 2). 401/403/404 fail at once.
- **Sturdier JSON parsing.** 0.2 only understood a bare object or a lower-case ```` ```json ```` fence; a reply like "Here is my review: ```` ```JSON ```` {...}" or "Sure. {...}" failed. The parser now takes the first balanced JSON object of the right shape anywhere in the reply, ignoring braces inside strings.
- A reply with `"content": null` (a refusal) is now `InvalidResponse`, not a JSON decode error.
- **Grounded check:** `check_against_sources(text, sources)` / `LlmClient::check_against_sources` / `llm_affector check --source FILE` (repeatable): claims the documents do not support are issues, for retrieval-augmented answers.
- **Bring your own client:** the one-method `judge::Judge` trait (implemented for `LlmClient`) and `detect_hallucination_with`, `check_against_sources_with`, `critique_code_with`, so async-openai, genai, rig or a local model can be the judge. `examples/custom_judge.rs` runs with no key.
- **JSON mode:** `.json_mode(true)` / `LLM_JSON_MODE=1` sends `response_format: {"type": "json_object"}`. Off by default.
- **`json-repair` feature** (off): broken judge JSON (trailing commas, single quotes) is repaired with llm_json before giving up.
- Transport errors include their cause ("tcp connect error: ... refused") instead of only "error sending request".
- `LlmClient::builder()` to configure base URL, key, model, timeout and retries in code, and `LlmClient::detect_hallucination` / `LlmClient::critique_code` to reuse one client. The free functions still read the environment.
- `dotenv` (unmaintained, RUSTSEC-2021-0141) replaced by `dotenvy`.
- An unparseable `LLM_TIMEOUT_SECONDS` is now a `ConfigError` instead of silently using 30.
- `LlmAffectorError` is `#[non_exhaustive]`; `Verdict`, `Issue` and `CritiqueReport` derive `PartialEq`.
- First real tests: unit tests, end-to-end tests against a local mock server (retry timing, Retry-After, no retry on 401, no auth header without a key, sources and JSON mode in the request), CLI tests that run the binary, and a reply corpus: of ten reply shapes models produce, 0.2.1's parser (copied into the test unchanged) understood 6 and 0.3.0 understands 10. The empty placeholder tests, examples and docs files are gone.
- GitLab CI runs the tests with all and with no default features, clippy and rustdoc with `-D warnings` and an MSRV check, and builds a Linux release on version tags. Minimum Rust version 1.88 (checked with `cargo +1.88.0`). Issue templates. Examples: `custom_judge`, `grounded_check`, `basic_usage`.

## 0.2.0 (2026-09-25)

- New command-line tool: `llm_affector check <text|--file PATH|->` and `llm_affector critique <PATH|->`, with `--json`, `--help`, `--version` and exit code 1 when hallucinations are found. Replaces the hard-coded demo binary.
- `LLM_BASE_URL`, `LLM_MODEL` and `LLM_TIMEOUT_SECONDS` are now read (they were documented in `.env.example` but ignored). Default model is now `gpt-4o-mini`.
- Prebuilt binaries for Windows, macOS (Apple Silicon and Intel) and Linux on every GitHub Release, with SHA256SUMS.txt.
- License metadata set to MIT to match the LICENSE file. Cargo.lock is committed.

## 0.1.0

- Initial release: `detect_hallucination` and `critique_code`.
