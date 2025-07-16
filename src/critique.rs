use crate::client::LlmClient;
use crate::errors::LlmAffectorError;
use crate::types::CritiqueReport;

/// Critique Rust code for potential issues
/// 
/// Takes a Rust source code snippet and uses an LLM to analyze it for
/// potential bugs, edge cases, style violations, and missing tests.
/// 
/// # Arguments
/// * `code` - The Rust source code to analyze
/// 
/// # Returns
/// * `Ok(CritiqueReport)` containing analysis results
/// * `Err(LlmAffectorError)` if the analysis fails
pub async fn critique_code(code: &str) -> Result<CritiqueReport, LlmAffectorError> {
    let client = LlmClient::new()?;
    
    let prompt = build_critique_prompt(code);
    let response_text = client.send_prompt(&prompt).await?;
    parse_critique_response(&response_text)
}

fn build_critique_prompt(code: &str) -> String {
    format!(
        "You are a Rust expert. Review this code snippet and provide a JSON report listing risks, suggested improvements, and any missing tests. Return only valid JSON with this exact structure:\n\n{{\n  \"risks\": [\"list of potential bugs or security issues\"],\n  \"improvements\": [\"list of code quality and style suggestions\"],\n  \"missing_tests\": [\"list of test scenarios that should be added\"]\n}}\n\nCode to analyze:\n```rust\n{}\n```",
        code
    )
}

fn parse_critique_response(response: &str) -> Result<CritiqueReport, LlmAffectorError> {
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