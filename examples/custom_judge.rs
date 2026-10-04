//! Use your own model client as the judge by implementing `Judge`. This
//! example's judge is a fixed reply, so it runs without any key; replace
//! `complete` with a call through async-openai, genai, rig or a local model.
//!
//!     cargo run --example custom_judge

use llm_affector::judge::{check_against_sources_with, detect_hallucination_with, Judge};
use llm_affector::{LlmAffectorError, Verdict};

struct FixedReply(&'static str);

impl Judge for FixedReply {
    async fn complete(&self, prompt: &str) -> Result<String, LlmAffectorError> {
        println!("(judge received a {}-character prompt)", prompt.len());
        Ok(self.0.to_string())
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), LlmAffectorError> {
    let judge = FixedReply(
        "Checked.\n```JSON\n{\"verdict\": \"FAIL\", \"issues\": [{\"claim\": \"opened in 1999\", \"explanation\": \"Source [1] says 31 March 1889.\"}]}\n```",
    );
    let answer = "The Eiffel Tower opened in 1999.";
    for (label, verdict) in [
        ("own knowledge", detect_hallucination_with(&judge, answer).await?),
        (
            "against sources",
            check_against_sources_with(&judge, answer, &["The tower opened on 31 March 1889."]).await?,
        ),
    ] {
        match verdict {
            Verdict::Pass => println!("{label}: PASS"),
            Verdict::Fail(issues) => {
                for i in issues {
                    println!("{label}: FAIL {:?}: {}", i.claim, i.explanation);
                }
            }
        }
    }
    Ok(())
}
