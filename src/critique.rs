use crate::client::LlmClient;
use crate::errors::LlmAffectorError;
use crate::types::CritiqueReport;

/// Review a Rust snippet for bugs, style problems and missing tests.
///
/// Builds an [`LlmClient`] from the environment (see [`LlmClient::new`]).
/// To reuse one client or configure it in code, call
/// [`LlmClient::critique_code`] instead.
pub async fn critique_code(code: &str) -> Result<CritiqueReport, LlmAffectorError> {
    LlmClient::new()?.critique_code(code).await
}

pub(crate) fn build_critique_prompt(code: &str) -> String {
    format!(
        "You are a Rust expert. Review this code snippet and provide a JSON report listing risks, suggested improvements, and any missing tests. Return only valid JSON with this exact structure:\n\n{{\n  \"risks\": [\"list of potential bugs or security issues\"],\n  \"improvements\": [\"list of code quality and style suggestions\"],\n  \"missing_tests\": [\"list of test scenarios that should be added\"]\n}}\n\nCode to analyze:\n```rust\n{}\n```",
        code
    )
}

pub(crate) fn parse_critique(response: &str) -> Result<CritiqueReport, LlmAffectorError> {
    crate::json::parse_object(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_report_after_prose_and_skips_other_objects() {
        let r = parse_critique(
            r#"Example {"a": 1}. Report: {"risks":["divide by zero"],"improvements":[],"missing_tests":["b = 0"]}"#,
        )
        .unwrap();
        assert_eq!(r.risks, vec!["divide by zero"]);
        assert!(r.improvements.is_empty());
        assert_eq!(r.missing_tests, vec!["b = 0"]);
    }

    #[test]
    fn prompt_contains_code() {
        assert!(build_critique_prompt("fn x() {}").contains("fn x() {}"));
    }
}
