//! Judge replies in the shapes models actually produce, parsed by this
//! release and by 0.2.1's parser (copied below unchanged), to compare how
//! many each understands.

use llm_affector::judge::{detect_hallucination_with, Judge};
use llm_affector::LlmAffectorError;
use serde::Deserialize;

const CORPUS: &[&str] = &[
    // 1. bare object
    r#"{"verdict": "PASS", "issues": []}"#,
    // 2. lower-case json fence
    "```json\n{\"verdict\": \"FAIL\", \"issues\": [{\"claim\": \"c\", \"explanation\": \"e\"}]}\n```",
    // 3. upper-case fence
    "```JSON\n{\"verdict\": \"PASS\"}\n```",
    // 4. untagged fence
    "```\n{\"verdict\": \"PASS\"}\n```",
    // 5. prose before
    "Here is my assessment:\n{\"verdict\": \"PASS\", \"issues\": []}",
    // 6. prose before and after
    "Sure.\n{\"verdict\": \"FAIL\", \"issues\": [{\"claim\": \"x\", \"explanation\": \"y\"}]}\nLet me know if you need more.",
    // 7. prose then a fenced block
    "After checking each claim:\n\n```json\n{\"verdict\": \"PASS\"}\n```",
    // 8. braces inside strings
    r#"{"verdict": "FAIL", "issues": [{"claim": "uses {braces}", "explanation": "a } inside"}]}"#,
    // 9. lower-case verdict
    r#"{"verdict": "fail", "issues": [{"claim": "c", "explanation": "e"}]}"#,
    // 10. a schema echo before the answer
    "The format is {\"verdict\": ...}. Answer: {\"verdict\": \"PASS\"}",
];

/// 0.2.1's `parse_hallucination_response` and verdict check, unchanged.
mod v021 {
    use super::Deserialize;

    #[derive(Deserialize)]
    pub struct HallucinationResponse {
        pub verdict: String,
        #[allow(dead_code)]
        pub issues: Option<Vec<serde_json::Value>>,
    }

    pub fn parses(response: &str) -> bool {
        let json_text = if response.contains("```json") {
            response
                .split("```json")
                .nth(1)
                .and_then(|s| s.split("```").next())
                .unwrap_or(response)
                .trim()
        } else if response.contains("```") {
            response.split("```").nth(1).unwrap_or(response).trim()
        } else {
            response.trim()
        };
        match serde_json::from_str::<HallucinationResponse>(json_text) {
            Ok(r) => matches!(r.verdict.to_uppercase().as_str(), "PASS" | "FAIL"),
            Err(_) => false,
        }
    }
}

struct Canned(&'static str);

impl Judge for Canned {
    async fn complete(&self, _prompt: &str) -> Result<String, LlmAffectorError> {
        Ok(self.0.to_string())
    }
}

#[tokio::test]
async fn this_release_understands_every_reply_shape() {
    let mut old = 0;
    let mut new = 0;
    for (i, reply) in CORPUS.iter().enumerate() {
        let ok_new = detect_hallucination_with(&Canned(reply), "x").await.is_ok();
        let ok_old = v021::parses(reply);
        println!("reply {:>2}: 0.2.1 {:<5} 0.3.0 {}", i + 1, ok_old, ok_new);
        old += usize::from(ok_old);
        new += usize::from(ok_new);
        assert!(ok_new, "0.3.0 failed on reply {}", i + 1);
    }
    println!("0.2.1 parsed {old}/{n}, 0.3.0 parsed {new}/{n}", n = CORPUS.len());
    assert_eq!(new, CORPUS.len());
    assert_eq!(old, 6, "0.2.1 parses replies 1, 2, 4, 7, 8 and 9 only");
}
