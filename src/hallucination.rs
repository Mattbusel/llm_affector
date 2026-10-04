use crate::client::LlmClient;
use crate::errors::LlmAffectorError;
use crate::types::{HallucinationResponse, Verdict};

/// Check LLM-generated text for hallucinations with a second LLM call.
///
/// Builds an [`LlmClient`] from the environment (`LLM_API_KEY`,
/// `LLM_BASE_URL`, `LLM_MODEL`, ...; see [`LlmClient::new`]) and asks it to
/// fact-check `input`. To reuse one client or configure it in code, call
/// [`LlmClient::detect_hallucination`] instead.
///
/// Returns [`Verdict::Pass`] if the judge finds nothing,
/// [`Verdict::Fail`] with the problem claims otherwise.
pub async fn detect_hallucination(input: &str) -> Result<Verdict, LlmAffectorError> {
    LlmClient::new()?.detect_hallucination(input).await
}

/// Check `text` against `sources` with a client built from the environment
/// (see [`LlmClient::new`]): every claim the sources do not support is an
/// issue, even if it happens to be true. Use this for retrieval-augmented
/// answers, where the question is "did the answer stick to the documents".
pub async fn check_against_sources(text: &str, sources: &[&str]) -> Result<Verdict, LlmAffectorError> {
    LlmClient::new()?.check_against_sources(text, sources).await
}

pub(crate) fn build_hallucination_prompt(input: &str) -> String {
    format!(
        "You are an expert fact-checker. Identify any hallucinations or unsupported claims in this answer and return a JSON object with verdict 'PASS' or 'FAIL' and a list of issues. Each issue should have 'claim' and 'explanation' fields.\n\nAnswer to analyze:\n{}\n\nReturn only valid JSON with this structure:\n{{\n  \"verdict\": \"PASS\" or \"FAIL\",\n  \"issues\": [\n    {{\n      \"claim\": \"specific claim that is problematic\",\n      \"explanation\": \"why this claim is unsupported or incorrect\"\n    }}\n  ]\n}}",
        input
    )
}

pub(crate) fn build_grounded_prompt(input: &str, sources: &[&str]) -> String {
    let mut s = String::from(
        "You are a strict fact-checker. Check the answer below ONLY against the numbered sources. \
         A claim is an issue if the sources do not support it, even if you believe it is true, \
         or if the sources contradict it. Return a JSON object with verdict 'PASS' (every claim \
         is supported) or 'FAIL', and a list of issues, each with 'claim' and 'explanation' \
         (name the source number when one contradicts the claim).\n\n",
    );
    for (i, src) in sources.iter().enumerate() {
        s.push_str(&format!("Source [{}]:\n{}\n\n", i + 1, src.trim()));
    }
    s.push_str(&format!(
        "Answer to check:\n{}\n\nReturn only valid JSON with this structure:\n\
         {{\n  \"verdict\": \"PASS\" or \"FAIL\",\n  \"issues\": [{{\"claim\": \"...\", \"explanation\": \"...\"}}]\n}}",
        input.trim()
    ));
    s
}

pub(crate) fn parse_verdict(response: &str) -> Result<Verdict, LlmAffectorError> {
    let parsed: HallucinationResponse = crate::json::parse_object(response)?;
    match parsed.verdict.trim().to_uppercase().as_str() {
        "PASS" => Ok(Verdict::Pass),
        "FAIL" => Ok(Verdict::Fail(parsed.issues.unwrap_or_default())),
        _ => Err(LlmAffectorError::InvalidResponse(format!(
            "Invalid verdict: {}",
            parsed.verdict
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verdicts() {
        assert_eq!(parse_verdict(r#"{"verdict":"pass"}"#).unwrap(), Verdict::Pass);
        let v = parse_verdict(
            r#"```json
{"verdict":"FAIL","issues":[{"claim":"c","explanation":"e"}]}
```"#,
        )
        .unwrap();
        match v {
            Verdict::Fail(i) => assert_eq!(i[0].claim, "c"),
            Verdict::Pass => panic!("expected FAIL"),
        }
        assert!(parse_verdict(r#"{"verdict":"MAYBE"}"#).is_err());
    }

    #[test]
    fn grounded_prompt_numbers_sources() {
        let p = build_grounded_prompt("It opened in 1889.", &["Doc one.", "Doc two."]);
        assert!(p.contains("Source [1]:\nDoc one."));
        assert!(p.contains("Source [2]:\nDoc two."));
        assert!(p.contains("It opened in 1889."));
    }

    #[test]
    fn prompt_contains_input() {
        assert!(build_hallucination_prompt("The moon is cheese").contains("The moon is cheese"));
    }
}
