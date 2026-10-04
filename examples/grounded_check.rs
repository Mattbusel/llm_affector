//! Did a retrieval-augmented answer stick to its documents? Checks an answer
//! against the passages it was given, with a local Ollama model by default.
//!
//!     ollama pull qwen2.5:7b
//!     cargo run --example grounded_check
//!
//! Or set LLM_BASE_URL / LLM_MODEL / LLM_API_KEY for another server.

use llm_affector::{LlmClient, Verdict};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let base = std::env::var("LLM_BASE_URL").unwrap_or_else(|_| "http://localhost:11434/v1".into());
    let model = std::env::var("LLM_MODEL").unwrap_or_else(|_| "qwen2.5:7b".into());
    let mut builder = LlmClient::builder().base_url(base).model(model).json_mode(true);
    if let Ok(key) = std::env::var("LLM_API_KEY") {
        builder = builder.api_key(key);
    }
    let judge = builder.build()?;

    let sources = [
        "The Eiffel Tower was built for the 1889 World's Fair and opened on 31 March 1889.",
        "It was the tallest man-made structure in the world until 1930.",
    ];
    let answer = "The Eiffel Tower opened in 1889 and is still the tallest structure in the world.";
    match judge.check_against_sources(answer, &sources).await {
        Ok(Verdict::Pass) => println!("PASS: every claim is supported by the sources"),
        Ok(Verdict::Fail(issues)) => {
            for i in issues {
                println!("FAIL {:?}: {}", i.claim, i.explanation);
            }
        }
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(2);
        }
    }
    Ok(())
}
