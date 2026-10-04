//! End-to-end tests against a local mock OpenAI-compatible server.

use std::time::{Duration, Instant};

use llm_affector::{LlmAffectorError, LlmClient, Verdict};
use serde_json::json;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

fn reply(content: &str) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(json!({
        "choices": [{"message": {"role": "assistant", "content": content}}]
    }))
}

fn client(server: &MockServer, retries: usize) -> LlmClient {
    LlmClient::builder()
        .base_url(format!("{}/v1", server.uri()))
        .model("judge-model")
        .max_retries(retries)
        .build()
        .unwrap()
}

#[tokio::test]
async fn check_parses_a_fail_wrapped_in_prose() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(reply(
            "Here is my assessment:\n```JSON\n{\"verdict\": \"FAIL\", \"issues\": [{\"claim\": \"finished in 1999\", \"explanation\": \"It opened in 1889.\"}]}\n```",
        ))
        .expect(1)
        .mount(&server)
        .await;

    let v = client(&server, 0)
        .detect_hallucination("The Eiffel Tower was finished in 1999.")
        .await
        .unwrap();
    match v {
        Verdict::Fail(issues) => {
            assert_eq!(issues.len(), 1);
            assert_eq!(issues[0].claim, "finished in 1999");
        }
        Verdict::Pass => panic!("expected FAIL"),
    }
}

#[tokio::test]
async fn request_carries_model_prompt_and_key() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(header("authorization", "Bearer sk-test"))
        .respond_with(|req: &Request| {
            let body: serde_json::Value = serde_json::from_slice(&req.body).unwrap();
            assert_eq!(body["model"], "judge-model");
            let prompt = body["messages"][0]["content"].as_str().unwrap();
            assert!(prompt.contains("fn divide"));
            reply(r#"{"risks":["b == 0 panics"],"improvements":[],"missing_tests":["divide by zero"]}"#)
        })
        .expect(1)
        .mount(&server)
        .await;

    let c = LlmClient::builder()
        .base_url(format!("{}/v1", server.uri()))
        .model("judge-model")
        .api_key("sk-test")
        .build()
        .unwrap();
    let r = c
        .critique_code("fn divide(a: i32, b: i32) -> i32 { a / b }")
        .await
        .unwrap();
    assert_eq!(r.risks, vec!["b == 0 panics"]);
    assert_eq!(r.missing_tests, vec!["divide by zero"]);
}

#[tokio::test]
async fn no_key_means_no_authorization_header() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(|req: &Request| {
            assert!(req.headers.get("authorization").is_none());
            reply(r#"{"verdict":"PASS","issues":[]}"#)
        })
        .expect(1)
        .mount(&server)
        .await;
    let v = client(&server, 0).detect_hallucination("Water is wet.").await.unwrap();
    assert_eq!(v, Verdict::Pass);
}

#[tokio::test]
async fn rate_limit_is_retried_after_the_servers_retry_after() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("retry-after", "1")
                .set_body_string("slow down"),
        )
        .up_to_n_times(1)
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(reply(r#"{"verdict":"PASS"}"#))
        .expect(1)
        .mount(&server)
        .await;

    let start = Instant::now();
    let v = client(&server, 2).detect_hallucination("x").await.unwrap();
    assert_eq!(v, Verdict::Pass);
    assert!(
        start.elapsed() >= Duration::from_millis(950),
        "waited only {:?}",
        start.elapsed()
    );
}

#[tokio::test]
async fn server_errors_stop_after_max_retries() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(503).insert_header("retry-after", "0"))
        .expect(3)
        .mount(&server)
        .await;
    let e = client(&server, 2).detect_hallucination("x").await.unwrap_err();
    assert!(matches!(e, LlmAffectorError::ApiError { status: 503, .. }), "{e}");
}

#[tokio::test]
async fn auth_errors_are_not_retried() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(401).set_body_string("bad key"))
        .expect(1)
        .mount(&server)
        .await;
    let e = client(&server, 3).critique_code("fn a() {}").await.unwrap_err();
    match e {
        LlmAffectorError::ApiError { status, body } => {
            assert_eq!(status, 401);
            assert_eq!(body, "bad key");
        }
        other => panic!("unexpected {other}"),
    }
}

#[tokio::test]
async fn null_content_is_an_invalid_response_not_a_json_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "choices": [{"message": {"role": "assistant", "content": null, "refusal": "no"}}]
        })))
        .mount(&server)
        .await;
    let e = client(&server, 0).detect_hallucination("x").await.unwrap_err();
    assert!(matches!(e, LlmAffectorError::InvalidResponse(_)), "{e}");
}

#[tokio::test]
async fn grounded_check_sends_the_sources_and_json_mode() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(|req: &Request| {
            let body: serde_json::Value = serde_json::from_slice(&req.body).unwrap();
            assert_eq!(body["response_format"]["type"], "json_object");
            let prompt = body["messages"][0]["content"].as_str().unwrap();
            assert!(prompt.contains("Source [1]:\nThe tower opened on 31 March 1889."));
            assert!(prompt.contains("Source [2]:\nIt is 330 m tall."));
            reply(r#"{"verdict":"FAIL","issues":[{"claim":"opened in 1999","explanation":"Source [1] says 1889."}]}"#)
        })
        .expect(1)
        .mount(&server)
        .await;
    let c = LlmClient::builder()
        .base_url(format!("{}/v1", server.uri()))
        .json_mode(true)
        .build()
        .unwrap();
    let v = c
        .check_against_sources(
            "The tower opened in 1999.",
            &["The tower opened on 31 March 1889.", "It is 330 m tall."],
        )
        .await
        .unwrap();
    assert!(matches!(v, Verdict::Fail(ref i) if i[0].explanation.contains("Source [1]")));
    let e = c.check_against_sources("x", &[]).await.unwrap_err();
    assert!(matches!(e, LlmAffectorError::ConfigError(_)));
}

#[tokio::test]
async fn json_mode_is_off_by_default() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(|req: &Request| {
            let body: serde_json::Value = serde_json::from_slice(&req.body).unwrap();
            assert!(body.get("response_format").is_none());
            reply(r#"{"verdict":"PASS"}"#)
        })
        .expect(1)
        .mount(&server)
        .await;
    assert_eq!(client(&server, 0).detect_hallucination("x").await.unwrap(), Verdict::Pass);
}

#[tokio::test]
async fn transport_errors_include_the_cause() {
    let c = LlmClient::builder()
        .base_url("http://127.0.0.1:9/v1")
        .max_retries(0)
        .build()
        .unwrap();
    let e = c.detect_hallucination("x").await.unwrap_err().to_string();
    assert!(e.contains("error sending request"), "{e}");
    // The reason (connection refused) follows the generic reqwest message.
    assert!(e.matches(": ").count() >= 3, "{e}");
}
