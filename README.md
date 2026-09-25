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
- Works with any OpenAI-compatible endpoint (`LLM_BASE_URL`, `LLM_MODEL`).
- Ships as a command-line tool too: `llm_affector check "some claim"` or `llm_affector critique src/main.rs`.

## Install

### Download the command-line tool (no Rust needed)

Grab the file for your system from the [latest release](https://github.com/Mattbusel/llm_affector/releases/latest):

| System | File |
|--------|------|
| Windows | `llm_affector-vX.Y.Z-x86_64-pc-windows-msvc.zip` |
| macOS (Apple Silicon) | `llm_affector-vX.Y.Z-aarch64-apple-darwin.tar.gz` |
| macOS (Intel) | `llm_affector-vX.Y.Z-x86_64-apple-darwin.tar.gz` |
| Linux (x86_64) | `llm_affector-vX.Y.Z-x86_64-unknown-linux-gnu.tar.gz` |

Unzip it, set your key, and run it from a terminal:

```bash
export LLM_API_KEY=sk-...        # Windows PowerShell: $env:LLM_API_KEY="sk-..."
llm_affector check "The Eiffel Tower was finished in 1999."
llm_affector critique src/main.rs
llm_affector --help
```

`check` exits with 1 when it finds problem claims, so it can gate a script. Add `--json` for machine-readable output. `SHA256SUMS.txt` in the release lets you verify the download.

The binaries are not code-signed. Windows SmartScreen may say "unknown publisher": click **More info**, then **Run anyway**. On macOS, if it is blocked, right-click the file and choose **Open** (or run `xattr -d com.apple.quarantine llm_affector`).

### With Cargo

```bash
cargo install llm_affector      # the CLI
cargo add llm_affector          # the library, in your own project
```

### From source

```bash
git clone https://github.com/Mattbusel/llm_affector
cd llm_affector
cargo run --release -- check "some claim"
```

## Quick start

```toml
[dependencies]
llm_affector = "0.2"
tokio = { version = "1", features = ["full"] }
```

Set an OpenAI API key in the environment or in a `.env` file (loaded automatically):

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
cargo run -- check "some claim"    # the CLI (src/main.rs)
cargo run --example basic_usage    # library examples
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
| `src/client.rs` | `LlmClient`: reqwest call to an OpenAI-compatible Chat Completions endpoint (default `gpt-4o-mini`, temperature 0.1, 30 s timeout; all configurable) |
| `src/types.rs` | `Verdict`, `Issue`, `CritiqueReport` and the OpenAI wire types |
| `src/errors.rs` | `LlmAffectorError` |
| `src/main.rs` | the `llm_affector` command-line tool |
| `examples/basic_usage.rs` | runnable library demo |

## Status and limitations

Early prototype (0.2.0).

- Speaks the OpenAI Chat Completions format only. Configure it with `LLM_API_KEY` (required), `LLM_BASE_URL`, `LLM_MODEL` (default `gpt-4o-mini`) and `LLM_TIMEOUT_SECONDS` (default 30), from the environment or a `.env` file.
- Each call creates a new HTTP client.
- The hallucination check relies only on the judge model's own knowledge; there is no retrieval or source grounding, so treat a `Pass` as "the judge found nothing", not as verified.
- `tests/` and the `concurrent_analysis` / `error_handling` examples are placeholders with no content yet.
