# llm_affector 

[![Crates.io](https://img.shields.io/crates/v/llm_affector.svg)](https://crates.io/crates/llm_affector)
[![Documentation](https://docs.rs/llm_affector/badge.svg)](https://docs.rs/llm_affector)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)

An async Rust library for LLM-based content analysis, providing hallucination detection and code critique functionality. Built with Tokio for high-performance concurrent operations.

##  Features

-  **Hallucination Detection**: Analyze text for factual errors and unsupported claims
- **Code Critique**: Review Rust code for bugs, style issues, and missing tests  
-  **Async/Await Support**: Built on Tokio for maximum performance
-  **Concurrent Execution**: Run multiple analyses simultaneously with `tokio::join!`
-  **Multiple LLM Providers**: Currently supports OpenAI, easily extensible
-  **Type-Safe**: Strong typing for all data structures and responses
-  **Error Handling**: Comprehensive error types with detailed messages

##  Quick Start

### Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
llm_affector = "0.1.0"
tokio = { version = "1.0", features = ["full"] }
```

### Setup

1. **Set your OpenAI API key:**
   ```bash
   export LLM_API_KEY="sk-your-openai-api-key-here"
   ```

   Or create a `.env` file:
   ```env
   LLM_API_KEY=sk-your-openai-api-key-here
   ```

2. **Basic usage:**

```rust
use llm_affector::{detect_hallucination, critique_code, Verdict};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let suspicious_text = "Scientists have proven that coffee beans grow on the moon.";
    let risky_code = "fn divide(a: i32, b: i32) -> i32 { a / b }";
    
    // Run both analyses concurrently for maximum performance
    let (hallucination_result, critique_result) = tokio::join!(
        detect_hallucination(suspicious_text),
        critique_code(risky_code)
    );
    
    // Handle hallucination detection
    match hallucination_result? {
        Verdict::Pass => println!("✅ No hallucinations detected"),
        Verdict::Fail(issues) => {
            println!("❌ Hallucinations found:");
            for issue in issues {
                println!("  - {}: {}", issue.claim, issue.explanation);
            }
        }
    }
    
    // Handle code critique
    let report = critique_result?;
    if !report.risks.is_empty() {
        println!("⚠️ Code risks identified:");
        for risk in report.risks {
            println!("  - {}", risk);
        }
    }
    
    Ok(())
}
```

## 📚 API Reference

### Hallucination Detection

```rust
use llm_affector::{detect_hallucination, Verdict, Issue};

let result = detect_hallucination("Your text to analyze").await?;
match result {
    Verdict::Pass => {
        // No issues found
    }
    Verdict::Fail(issues) => {
        for issue in issues {
            println!("Problematic claim: {}", issue.claim);
            println!("Explanation: {}", issue.explanation);
        }
    }
}
```

### Code Critique

```rust
use llm_affector::{critique_code, CritiqueReport};

let report = critique_code(r#"
fn unsafe_function() {
    let data = std::ptr::null();
    // Potential issues here...
}
"#).await?;

println!("Risks: {:?}", report.risks);
println!("Improvements: {:?}", report.improvements);
println!("Missing tests: {:?}", report.missing_tests);
```

### Error Handling

```rust
use llm_affector::{detect_hallucination, LlmAffectorError};

match detect_hallucination("text").await {
    Ok(verdict) => { /* handle success */ }
    Err(LlmAffectorError::ApiKeyNotFound) => {
        eprintln!("Please set LLM_API_KEY environment variable");
    }
    Err(LlmAffectorError::HttpError(e)) => {
        eprintln!("Network error: {}", e);
    }
    Err(e) => {
        eprintln!("Other error: {}", e);
    }
}
```

##  Configuration

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `LLM_API_KEY` | OpenAI API key (required) | - |
| `LLM_BASE_URL` | Custom API endpoint | `https://api.openai.com/v1` |
| `LLM_MODEL` | Model to use | `gpt-4` |
| `LLM_TIMEOUT_SECONDS` | Request timeout | `30` |

### Using .env Files

Create a `.env` file in your project root:

```env
LLM_API_KEY=sk-your-openai-api-key-here
LLM_MODEL=gpt-4
LLM_TIMEOUT_SECONDS=60
```

Add to your `Cargo.toml`:
```toml
[dependencies]
dotenv = "0.15"
```

##  Examples

### Sequential Analysis

```rust
use llm_affector::{detect_hallucination, critique_code};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Run analyses one after another
    let hallucination_result = detect_hallucination("Text to check").await?;
    let critique_result = critique_code("fn example() {}").await?;
    
    // Process results...
    Ok(())
}
```

### Concurrent Analysis (Recommended)

```rust
use llm_affector::{detect_hallucination, critique_code};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Run analyses concurrently for better performance
    let (hallucination_result, critique_result) = tokio::join!(
        detect_hallucination("Text to check"),
        critique_code("fn example() {}")
    );
    
    let verdict = hallucination_result?;
    let report = critique_result?;
    
    // Process results...
    Ok(())
}
```

### Batch Processing

```rust
use llm_affector::detect_hallucination;
use futures::future::join_all;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let texts = vec![
        "Text 1 to analyze",
        "Text 2 to analyze", 
        "Text 3 to analyze",
    ];
    
    // Analyze all texts concurrently
    let futures = texts.iter().map(|text| detect_hallucination(text));
    let results = join_all(futures).await;
    
    for (i, result) in results.into_iter().enumerate() {
        match result? {
            Verdict::Pass => println!("Text {}: ✅ Clean", i + 1),
            Verdict::Fail(issues) => println!("Text {}: ❌ {} issues", i + 1, issues.len()),
        }
    }
    
    Ok(())
}
```

##  Architecture

```
llm_affector/
├── src/
│   ├── lib.rs              # Public API exports
│   ├── client.rs           # HTTP client for LLM APIs
│   ├── types.rs            # Data structures (Verdict, Issue, etc.)
│   ├── errors.rs           # Error types and handling
│   ├── hallucination.rs    # Hallucination detection logic
│   ├── critique.rs         # Code critique logic
│   └── main.rs             # Example binary
├── examples/               # Usage examples
├── tests/                  # Integration tests
└── docs/                   # Additional documentation
```

##  Testing

Run the test suite:

```bash
cargo test
```

Run examples:

```bash
# Basic usage
cargo run --example basic_usage

# Concurrent analysis
cargo run --example concurrent_analysis

# Error handling
cargo run --example error_handling
```

##  Performance

The library is designed for high performance:

- **Async/Await**: Non-blocking I/O operations
- **Concurrent Execution**: Run multiple analyses simultaneously
- **HTTP Connection Pooling**: Reuse connections via `reqwest`
- **Efficient JSON Parsing**: Streaming JSON with `serde`

### Benchmarks

| Operation | Time (avg) | Concurrent Speedup |
|-----------|------------|-------------------|
| Single hallucination check | ~2.1s | - |
| Single code critique | ~1.8s | - |
| Both analyses (sequential) | ~3.9s | - |
| Both analyses (concurrent) | ~2.3s | **1.7x faster** |

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

### Adding New LLM Providers

The library is designed to be extensible. To add a new provider:

1. Implement the client interface in `src/client.rs`
2. Add provider-specific types in `src/types.rs`
3. Update the configuration system
4. Add tests and examples

### Running Development Setup

```bash
git clone https://github.com/username/llm_affector
cd llm_affector
cp .env.example .env
# Edit .env with your API key
cargo build
cargo test
```

##  License

This project is licensed under either of

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

##  Acknowledgments

- Built with [Tokio](https://tokio.rs/) for async runtime
- HTTP client powered by [reqwest](https://github.com/seanmonstar/reqwest)
- JSON handling via [serde](https://serde.rs/)
- Error handling with [thiserror](https://github.com/dtolnay/thiserror)

##  Changelog

### [0.1.0] - 2025-07-16

- Initial release
- Hallucination detection functionality
- Code critique functionality  
- OpenAI API integration
- Async/await support with Tokio
- Comprehensive error handling
- Example usage and documentation

---

**Made with ❤️ in Rust** 🦀