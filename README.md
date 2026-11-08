# llm_affector

<!-- outcome-block -->
**Catches false claims and risky code before they ship. A second LLM checks the first and returns pass or fail. Gate CI on it.**

Exits 1 when it finds problem claims. Checks answers against your own documents. Works with OpenAI or a local Ollama.

If it doesn't do what this README says, [open an issue](https://github.com/Mattbusel/llm_affector/issues) and I'll fix it.
<!-- outcome-block -->

A Rust library and CLI for "LLM as judge" checks on generated text or code.

- `detect_hallucination` flags false or unsupported claims in text.
- `check_against_sources` flags claims your documents don't support (for RAG answers).
- `critique_code` reviews a Rust snippet for risks, fixes and missing tests.

Results are typed (`Verdict::Pass` / `Verdict::Fail(issues)`, or a `CritiqueReport`), not free text. Works with OpenAI and any server that speaks the OpenAI Chat Completions format: local Ollama, OpenRouter, Gemini, vLLM, LM Studio or a company proxy.

## Install

```bash
cargo install llm_affector      # the command-line tool
cargo add llm_affector          # the library, in your own project
```

## Quick start (command line)

```bash
export LLM_API_KEY=sk-...        # Windows PowerShell: $env:LLM_API_KEY="sk-..."
llm_affector check "The Eiffel Tower was finished in 1999."
llm_affector critique src/main.rs
llm_affector check --json - < answer.txt
```

`check` exits 1 when it finds problem claims and 2 on errors, so it can gate a script or CI job. `--json` gives machine-readable output. `--source FILE` (repeatable) checks the text only against those documents:

```bash
llm_affector check --source docs/eiffel.txt "The Eiffel Tower opened in 1999."
```

No API key? Run a local model with [Ollama](https://ollama.com) and point the tool at it:

```bash
ollama pull qwen2.5:7b
export LLM_BASE_URL=http://localhost:11434/v1 LLM_MODEL=qwen2.5:7b
llm_affector check "The Moon is 3 km from Earth."
```

## Quick start (library)

```toml
[dependencies]
llm_affector = "0.3"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

```rust,no_run
use llm_affector::{critique_code, detect_hallucination, Verdict};

#[tokio::main]
async fn main() -> Result<(), llm_affector::LlmAffectorError> {
    // Reads LLM_API_KEY, LLM_BASE_URL and LLM_MODEL from the environment or a .env file.
    let answer = "Scientists have proven that coffee beans grow on the moon.";
    let code = "fn divide(a: i32, b: i32) -> i32 { a / b }";

    let (verdict, report) = tokio::join!(detect_hallucination(answer), critique_code(code));

    match verdict? {
        Verdict::Pass => println!("no unsupported claims found"),
        Verdict::Fail(issues) => {
            for issue in issues {
                println!("- {}: {}", issue.claim, issue.explanation);
            }
        }
    }
    println!("risks: {:?}", report?.risks);
    Ok(())
}
```

To set up the client in code, or reuse one client for many calls, use the builder:

```rust,no_run
# async fn demo() -> Result<(), llm_affector::LlmAffectorError> {
use std::time::Duration;
use llm_affector::LlmClient;

let judge = LlmClient::builder()
    .base_url("https://openrouter.ai/api/v1")
    .api_key(std::env::var("OPENROUTER_API_KEY").unwrap_or_default())
    .model("openai/gpt-4o-mini")
    .timeout(Duration::from_secs(60))
    .max_retries(3)
    .build()?;
let verdict = judge.detect_hallucination("Rust 1.0 shipped in 2015.").await?;
println!("{verdict:?}");
# Ok(()) }
```

## Why this crate

| You want | llm_affector | Alternatives |
|---|---|---|
| An LLM-as-judge check from Rust or a shell | One async call or one CLI command, typed result, exit codes | [DeepEval](https://github.com/confident-ai/deepeval) and [RAGAS](https://github.com/explodinggradients/ragas) (Python) have many more metrics and test-suite integration; [mira-eval](https://crates.io/crates/mira-eval) is a Rust evaluation framework for agents |
| To check a RAG answer against its documents | `check_against_sources` / `--source` | RAGAS "faithfulness", DeepEval "hallucination" metric |
| To use the model client you already have | Implement the one-method `Judge` trait (async-openai, genai, rig, a local model, a test double) | |
| Parsing judge replies that don't break | The first JSON object of the right shape anywhere in the reply; optional `json-repair` for broken JSON; optional `json_mode` | |

It is a thin tool. The judge is an LLM and can be wrong both ways. Without sources it only has its own knowledge.

## Bring your own client

```rust
use llm_affector::judge::{detect_hallucination_with, Judge};
use llm_affector::{LlmAffectorError, Verdict};

struct MyJudge; // wrap async-openai, genai, rig, a local model...

impl Judge for MyJudge {
    async fn complete(&self, prompt: &str) -> Result<String, LlmAffectorError> {
        // send `prompt`, return the reply text
        # let _ = prompt;
        Ok(r#"{"verdict": "PASS", "issues": []}"#.to_string())
    }
}

# #[tokio::main(flavor = "current_thread")]
# async fn main() -> Result<(), LlmAffectorError> {
let verdict = detect_hallucination_with(&MyJudge, "Rust 1.0 shipped in 2015.").await?;
assert_eq!(verdict, Verdict::Pass);
# Ok(())
# }
```

## Providers

Anything with an OpenAI-compatible `/chat/completions` endpoint works. Set `LLM_BASE_URL` (or `.base_url(...)`) to:

| Provider | Base URL | Key |
|---|---|---|
| OpenAI (default) | `https://api.openai.com/v1` | required |
| Ollama (local) | `http://localhost:11434/v1` | none |
| OpenRouter | `https://openrouter.ai/api/v1` | required |
| Google Gemini | `https://generativelanguage.googleapis.com/v1beta/openai` | required |
| vLLM / LM Studio | `http://localhost:8000/v1` / `http://localhost:1234/v1` | usually none |

## Configuration

| Variable | Default | Meaning |
|---|---|---|
| `LLM_API_KEY` | none | API key, sent as a Bearer token. Required only for the default OpenAI endpoint. |
| `LLM_BASE_URL` | `https://api.openai.com/v1` | The part before `/chat/completions`. |
| `LLM_MODEL` | `gpt-4o-mini` | Judge model. |
| `LLM_TIMEOUT_SECONDS` | `30` | Timeout per attempt. |
| `LLM_MAX_RETRIES` | `2` | Retries after the first attempt. |
| `LLM_JSON_MODE` | off | `1` sends `"response_format": {"type": "json_object"}` (OpenAI, Ollama, vLLM, Gemini's OpenAI endpoint, most OpenRouter models). |

It also reads a `.env` file in the current directory (see `.env.example`).

Requests that fail with HTTP 408, 429 or 5xx, time out, or can't connect are retried with exponential backoff and jitter. If the server sends `Retry-After` (seconds or a date), it waits that long instead, up to 60 seconds. Other errors (bad key, unknown model) fail right away.

## Handling errors

```rust,no_run
use llm_affector::{detect_hallucination, LlmAffectorError};

async fn check(text: &str) {
    match detect_hallucination(text).await {
        Ok(verdict) => println!("{verdict:?}"),
        Err(LlmAffectorError::ApiKeyNotFound) => eprintln!("set LLM_API_KEY"),
        Err(LlmAffectorError::ApiError { status, body }) => eprintln!("API returned {status}: {body}"),
        Err(LlmAffectorError::InvalidResponse(msg)) => eprintln!("the judge did not return the JSON asked for: {msg}"),
        Err(e) => eprintln!("{e}"),
    }
}
```

## Cargo features

| Feature | Default | What it adds |
|---|---|---|
| `json-repair` | off | If no valid JSON object is found, fixes the reply with [llm_json](https://crates.io/crates/llm_json) (trailing commas, single quotes, unquoted keys). Off by default because llm_json pulls in clap. |

## Examples

| Example | Needs | What it shows |
|---|---|---|
| `custom_judge` | nothing | Your own `Judge`, with a fixed reply |
| `grounded_check` | Ollama (or `LLM_BASE_URL`) | Checking a RAG answer against its passages |
| `basic_usage` | `LLM_API_KEY` | Both checks and the critique |

## How it works

The judge is asked for JSON. The reply is searched for the first JSON object of the right shape. A sentence before it, a code fence in any case (```` ```json ````, ```` ```JSON ````) or a note after it won't break parsing. On ten reply shapes in `tests/reply_corpus.rs`, the 0.2.1 parser read 6. This release reads all 10.

| File | What it holds |
|---|---|
| `src/hallucination.rs` | fact-checker prompt, parsing into `Verdict` |
| `src/critique.rs` | Rust reviewer prompt, parsing into `CritiqueReport` |
| `src/client.rs` | `LlmClient`: the HTTP call, builder, retries |
| `src/json.rs` | finds the JSON object in a reply |
| `src/judge.rs` | the `Judge` trait and the `*_with` functions |
| `src/main.rs` | the `llm_affector` command-line tool |

## Limitations

- The hallucination check uses only the judge model's own knowledge. There is no retrieval. A `Pass` means "the judge found nothing", not "verified".
- The judge is itself an LLM and can be wrong in both directions.

## License

MIT
