//! The `llm_affector` binary against a local mock server.

use std::process::Command;

use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

fn run(server: &MockServer, args: &[&str]) -> (i32, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_llm_affector"))
        .args(args)
        .env("LLM_BASE_URL", format!("{}/v1", server.uri()))
        .env_remove("LLM_API_KEY")
        .env("LLM_MAX_RETRIES", "0")
        .current_dir(std::env::temp_dir())
        .output()
        .unwrap();
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr),
    )
}

#[tokio::test(flavor = "multi_thread")]
async fn check_with_sources_exits_1_on_fail_and_prints_json() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(|req: &Request| {
            let body: serde_json::Value = serde_json::from_slice(&req.body).unwrap();
            let prompt = body["messages"][0]["content"].as_str().unwrap();
            assert!(prompt.contains("Source [1]:\nopened in 1889"), "{prompt}");
            ResponseTemplate::new(200).set_body_json(json!({"choices": [{"message": {"role": "assistant",
                "content": "{\"verdict\":\"FAIL\",\"issues\":[{\"claim\":\"1999\",\"explanation\":\"Source [1] says 1889\"}]}"}}]}))
        })
        .mount(&server)
        .await;
    let doc = std::env::temp_dir().join("llm_affector_cli_source.txt");
    std::fs::write(&doc, "opened in 1889").unwrap();
    let doc = doc.to_string_lossy().into_owned();
    let (code, out) = run(&server, &["check", "--json", "--source", &doc, "It", "opened", "in", "1999."]);
    assert_eq!(code, 1, "{out}");
    let v: serde_json::Value = serde_json::from_str(out.trim()).unwrap();
    assert_eq!(v["verdict"], "FAIL");
    assert_eq!(v["issues"][0]["claim"], "1999");
}

#[tokio::test(flavor = "multi_thread")]
async fn pass_exits_0_and_missing_source_file_exits_2() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"choices": [{"message": {"role": "assistant", "content": "{\"verdict\":\"PASS\",\"issues\":[]}"}}]})))
        .mount(&server)
        .await;
    let (code, out) = run(&server, &["check", "Water is wet."]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("PASS"));
    let (code, out) = run(&server, &["check", "--source", "no-such-file.txt", "x"]);
    assert_eq!(code, 2, "{out}");
    assert!(out.contains("cannot read"));
}
