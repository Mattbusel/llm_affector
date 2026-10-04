//! Pull a JSON object out of a model reply.
//!
//! Judge models are asked for "only JSON", but real replies often add a
//! sentence before it, wrap it in a ```JSON fence (any case, any language
//! tag), or add a note after it. The 0.2 parser only handled a lower-case
//! ```json fence or a bare object, and failed on everything else.

use serde::de::DeserializeOwned;

use crate::errors::LlmAffectorError;

/// Find the first balanced `{...}` in `text` that parses as a JSON object of
/// type `T`. Braces inside JSON strings are ignored while matching.
pub(crate) fn parse_object<T: DeserializeOwned>(text: &str) -> Result<T, LlmAffectorError> {
    let mut first_err: Option<String> = None;
    for (start, _) in text.match_indices('{') {
        let Some(end) = matching_brace(&text[start..]) else {
            continue;
        };
        let candidate = &text[start..start + end + 1];
        match serde_json::from_str::<T>(candidate) {
            Ok(v) => return Ok(v),
            Err(e) => {
                if first_err.is_none() {
                    first_err = Some(e.to_string());
                }
            }
        }
    }
    #[cfg(feature = "json-repair")]
    if let Some(v) = repaired(text) {
        return Ok(v);
    }
    let preview: String = text.chars().take(200).collect();
    Err(LlmAffectorError::InvalidResponse(match first_err {
        Some(e) => format!("Failed to parse JSON: {e}; the model replied: {preview}"),
        None => format!("no JSON object in the model's reply: {preview}"),
    }))
}

/// Last resort with the `json-repair` feature: let llm_json fix trailing
/// commas, single quotes, unquoted keys and unclosed brackets, then parse.
#[cfg(feature = "json-repair")]
fn repaired<T: DeserializeOwned>(text: &str) -> Option<T> {
    let start = text.find('{')?;
    let fixed = llm_json::repair_json(&text[start..], &Default::default()).ok()?;
    serde_json::from_str(&fixed).ok()
}

/// Byte offset of the `}` that closes the `{` at the start of `s`.
fn matching_brace(s: &str) -> Option<usize> {
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for (i, c) in s.char_indices() {
        if in_string {
            match c {
                _ if escaped => escaped = false,
                '\\' => escaped = true,
                '"' => in_string = false,
                _ => {}
            }
            continue;
        }
        match c {
            '"' => in_string = true,
            '{' => depth += 1,
            '}' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Deserialize, Debug, PartialEq)]
    struct V {
        verdict: String,
    }

    #[test]
    fn bare_object() {
        let v: V = parse_object(r#"{"verdict":"PASS"}"#).unwrap();
        assert_eq!(v.verdict, "PASS");
    }

    #[test]
    fn upper_case_fence_and_prose() {
        // 0.2 only recognised a lower-case ```json fence; with ```JSON it took
        // the language tag as part of the JSON and failed.
        let reply = "Here is my review:\n```JSON\n{\"verdict\": \"FAIL\"}\n```\nHope that helps.";
        let v: V = parse_object(reply).unwrap();
        assert_eq!(v.verdict, "FAIL");
    }

    #[test]
    fn prose_before_unfenced_json() {
        // 0.2 tried to parse the whole reply and failed.
        let reply = "Sure. {\"verdict\": \"PASS\"}";
        let v: V = parse_object(reply).unwrap();
        assert_eq!(v.verdict, "PASS");
    }

    #[test]
    fn braces_inside_strings_do_not_confuse_matching() {
        let reply = r#"{"verdict": "FAIL", "note": "uses } and { and \" quotes"} trailing }"#;
        let v: V = parse_object(reply).unwrap();
        assert_eq!(v.verdict, "FAIL");
    }

    #[test]
    fn skips_objects_of_the_wrong_shape() {
        let reply = r#"Example: {"x": 1}. Answer: {"verdict": "PASS"}"#;
        let v: V = parse_object(reply).unwrap();
        assert_eq!(v.verdict, "PASS");
    }

    #[test]
    fn broken_json_needs_the_repair_feature() {
        // Trailing comma and single quotes, as small models often write.
        let reply = "{'verdict': 'PASS',}";
        let parsed = parse_object::<V>(reply);
        #[cfg(feature = "json-repair")]
        assert_eq!(parsed.unwrap().verdict, "PASS");
        #[cfg(not(feature = "json-repair"))]
        assert!(parsed.is_err());
    }

    #[test]
    fn no_json_is_a_clear_error() {
        let e = parse_object::<V>("I cannot help with that.").unwrap_err();
        assert!(e.to_string().contains("no JSON object"), "{e}");
    }
}
