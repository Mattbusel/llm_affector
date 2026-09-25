# llm_affector

[![CI](https://github.com/Mattbusel/llm_affector/actions/workflows/ci.yml/badge.svg)](https://github.com/Mattbusel/llm_affector/actions/workflows/ci.yml)

A small async Rust library that uses a second LLM call to check the first: `detect_hallucination` flags unsupported or false claims in a piece of text, and `critique_code` reviews a Rust snippet for risks, improvements and missing tests. Both return typed results.

The idea is an "LLM as judge" step you can drop into a pipeline: generate an answer, then ask a model to audit it and get back structured JSON (`Verdict::Pass` / `Verdict::Fail(issues)`, or a `CritiqueReport`) instead of free text. The two checks are independent async functions, so you can run them concurrently with `tokio::join!`.

## Features

- **`detect_hallucination(text)`** returns `Verdict::Pass` or `Verdict::Fail(Vec<Issue>)`, where each `Issue` has the problematic `claim` and an `explanation`.
- **`critique_code(code)`** returns a `CritiqueReport { risks, improvements, missing_tests }`.
- Tolerates model replies wrapped in Markdown code fences before parsing the JSON.
- Typed errors (`LlmAffectorError`): missing API key, HTTP failure, non-2xx API status with body, unparseable response.
- `LlmClient::send_prompt` is public if you want to send your own judge prompts.

## Quick start

```toml
[dependencies]
llm_affector = { git = "https://github.com/Mattbusel/llm_affector" }
tokio = { version = "1", features = ["full"] }
```

The crate is not on crates.io. Set an OpenAI API key in the environment or in a `.env` file (loaded automatically):

```bash
export LLM_API_KEY=sk-...
```

```rust
use llm_affector::{critique_code, detect_hallucination, Verdict};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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

    let report = report?;
    println!("risks: {:?}", report.risks);
    println!("missing tests: {:?}", report.missing_tests);
    Ok(())
}
```

Try it from a clone:

```bash
git clone https://github.com/Mattbusel/llm_affector
cd llm_affector
cp .env.example .env   # then put your key in LLM_API_KEY
cargo run                          # demo binary (src/main.rs)
cargo run --example basic_usage    # more examples
```

## Handling errors

```rust
use llm_affector::{detect_hallucination, LlmAffectorError};

async fn check(text: &str) {
    match detect_hallucination(text).await {
        Ok(verdict) => println!("{verdict:?}"),
        Err(LlmAffectorError::ApiKeyNotFound) => eprintln!("set LLM_API_KEY"),
        Err(LlmAffectorError::ApiError { status, body }) => eprintln!("API returned {status}: {body}"),
        Err(LlmAffectorError::InvalidResponse(msg)) => eprintln!("model did not return valid JSON: {msg}"),
        Err(e) => eprintln!("{e}"),
    }
}
```

## How it works

| File | What it holds |
|---|---|
| `src/hallucination.rs` | fact-checker prompt, JSON parsing into `Verdict` |
| `src/critique.rs` | Rust reviewer prompt, JSON parsing into `CritiqueReport` |
| `src/client.rs` | `LlmClient`: reqwest call to OpenAI Chat Completions (`gpt-4`, temperature 0.1, 30 s timeout) |
| `src/types.rs` | `Verdict`, `Issue`, `CritiqueReport` and the OpenAI wire types |
| `src/errors.rs` | `LlmAffectorError` |
| `src/main.rs`, `examples/basic_usage.rs` | runnable demos |

## Status and limitations

Early prototype (0.1.0).

- OpenAI only, and the model (`gpt-4`), endpoint and timeout are hard-coded. `.env.example` lists `LLM_BASE_URL`, `LLM_MODEL` and `LLM_TIMEOUT_SECONDS`, but the code does not read them yet; only `LLM_API_KEY` is used.
- Each call creates a new HTTP client.
- The hallucination check relies only on the judge model's own knowledge; there is no retrieval or source grounding, so treat a `Pass` as "the judge found nothing", not as verified.
- `tests/` and the `concurrent_analysis` / `error_handling` examples are placeholders with no content yet.
