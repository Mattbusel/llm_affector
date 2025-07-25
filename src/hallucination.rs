use crate::client::LlmClient;
use crate::errors::LlmAffectorError;
use crate::types::{Verdict, HallucinationResponse};

/// Detect hallucinations in LLM-generated text
/// 
/// Takes a text answer generated by an LLM and uses a secondary LLM call
/// to detect potential hallucinations, unsupported claims, or factual errors.
/// 
/// # Arguments
/// * `input` - The text to analyze for hallucinations
/// 
/// # Returns
/// * `Ok(Verdict::Pass)` if no issues are found
/// * `Ok(Verdict::Fail(issues))` if hallucinations are detected
/// * `Err(LlmAffectorError)` if the analysis fails
pub async fn detect_hallucination(input: &str) -> Result<Verdict, LlmAffectorError> {
    let client = LlmClient::new()?;
    
    let prompt = build_hallucination_prompt(input);
    let response_text = client.send_prompt(&prompt).await?;
    let hallucination_response = parse_hallucination_response(&response_text)?;

    match hallucination_response.verdict.to_uppercase().as_str() {
        "PASS" => Ok(Verdict::Pass),
        "FAIL" => Ok(Verdict::Fail(hallucination_response.issues.unwrap_or_default())),
        _ => Err(LlmAffectorError::InvalidResponse(
            format!("Invalid verdict: {}", hallucination_response.verdict)
        )),
    }
}

fn build_hallucination_prompt(input: &str) -> String {
    format!(
        "You are an expert fact-checker. Identify any hallucinations or unsupported claims in this answer and return a JSON object with verdict 'PASS' or 'FAIL' and a list of issues. Each issue should have 'claim' and 'explanation' fields.\n\nAnswer to analyze:\n{}\n\nReturn only valid JSON with this structure:\n{{\n  \"verdict\": \"PASS\" or \"FAIL\",\n  \"issues\": [\n    {{\n      \"claim\": \"specific claim that is problematic\",\n      \"explanation\": \"why this claim is unsupported or incorrect\"\n    }}\n  ]\n}}",
        input
    )
}

fn parse_hallucination_response(response: &str) -> Result<HallucinationResponse, LlmAffectorError> {
    // Extract JSON from response if it's wrapped in markdown code blocks
    let json_text = if response.contains("```json") {
        response
            .split("```json")
            .nth(1)
            .and_then(|s| s.split("```").next())
            .unwrap_or(response)
            .trim()
    } else if response.contains("```") {
        response
            .split("```")
            .nth(1)
            .unwrap_or(response)
            .trim()
    } else {
        response.trim()
    };

    serde_json::from_str(json_text)
        .map_err(|e| LlmAffectorError::InvalidResponse(format!("Failed to parse JSON: {}", e)))
}