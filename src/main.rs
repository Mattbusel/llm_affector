//! `llm_affector` command-line tool: run the hallucination check or the code
//! critique from a terminal.

use std::io::Read;
use std::process::ExitCode;

use llm_affector::{check_against_sources, critique_code, detect_hallucination, Verdict};

const HELP: &str = "\
llm_affector: ask a second LLM to audit text or code

Usage:
  llm_affector check <TEXT>           check a claim or answer for hallucinations
  llm_affector check --file <PATH>    check the contents of a file
  llm_affector check -                read the text from stdin
  llm_affector check --source DOC.txt <TEXT>
                                      check only against these documents (repeatable)
  llm_affector critique <PATH>        review a Rust source file
  llm_affector critique -             review code read from stdin
  llm_affector --help | --version

Options:
  --json      print the raw result as JSON instead of a readable summary

Exit codes: 0 = pass / critique printed, 1 = hallucinations found, 2 = error.

Environment (a .env file in the current folder is read too):
  LLM_API_KEY          API key (required for the default OpenAI endpoint)
  LLM_BASE_URL         OpenAI-compatible endpoint (default https://api.openai.com/v1)
  LLM_MODEL            model name (default gpt-4o-mini)
  LLM_TIMEOUT_SECONDS  timeout per attempt (default 30)
  LLM_MAX_RETRIES      retries on 429, 5xx and timeouts (default 2)
  LLM_JSON_MODE        1 to ask the server for a JSON reply (response_format)

Examples:
  llm_affector check \"The Eiffel Tower was finished in 1999.\"
  LLM_BASE_URL=http://localhost:11434/v1 LLM_MODEL=qwen2.5:7b llm_affector check \"...\"
";

fn read_input(args: &[String], file_flag: bool) -> Result<String, String> {
    let file_pos = args.iter().position(|a| a == "--file" || a == "-f");
    if let Some(i) = file_pos {
        let path = args.get(i + 1).ok_or("--file needs a path")?;
        return std::fs::read_to_string(path).map_err(|e| format!("cannot read {path}: {e}"));
    }
    let rest: Vec<&String> = args.iter().filter(|a| *a != "--json").collect();
    match rest.as_slice() {
        [] => Err("nothing to analyse; pass text, a file, or - for stdin".into()),
        [one] if one.as_str() == "-" => {
            let mut buf = String::new();
            std::io::stdin()
                .read_to_string(&mut buf)
                .map_err(|e| format!("cannot read stdin: {e}"))?;
            Ok(buf)
        }
        [one] if file_flag => {
            std::fs::read_to_string(one.as_str()).map_err(|e| format!("cannot read {one}: {e}"))
        }
        many => Ok(many
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join(" ")),
    }
}

#[tokio::main]
async fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() || args.iter().any(|a| a == "-h" || a == "--help") {
        print!("{HELP}");
        return ExitCode::SUCCESS;
    }
    if args.iter().any(|a| a == "-V" || a == "--version") {
        println!("llm_affector {}", env!("CARGO_PKG_VERSION"));
        return ExitCode::SUCCESS;
    }
    let json = args.iter().any(|a| a == "--json");
    let (cmd, rest) = (args[0].as_str(), &args[1..]);

    match cmd {
        "check" => {
            // --source PATH (repeatable): check against these documents.
            let mut sources = Vec::new();
            let mut others: Vec<String> = Vec::new();
            let mut it = rest.iter();
            while let Some(a) = it.next() {
                if a == "--source" || a == "-s" {
                    let Some(path) = it.next() else {
                        return fail("--source needs a path");
                    };
                    match std::fs::read_to_string(path) {
                        Ok(t) => sources.push(t),
                        Err(e) => return fail(&format!("cannot read {path}: {e}")),
                    }
                } else {
                    others.push(a.clone());
                }
            }
            let rest = &others[..];
            let text = match read_input(rest, false) {
                Ok(t) if !t.trim().is_empty() => t,
                Ok(_) => return fail("input is empty"),
                Err(e) => return fail(&e),
            };
            let result = if sources.is_empty() {
                detect_hallucination(&text).await
            } else {
                let refs: Vec<&str> = sources.iter().map(String::as_str).collect();
                check_against_sources(&text, &refs).await
            };
            match result {
                Ok(verdict) => {
                    let failed = matches!(verdict, Verdict::Fail(_));
                    if json {
                        let issues = match &verdict {
                            Verdict::Pass => Vec::new(),
                            Verdict::Fail(i) => i
                                .iter()
                                .map(|i| serde_json::json!({"claim": i.claim, "explanation": i.explanation}))
                                .collect(),
                        };
                        let v = serde_json::json!({
                            "verdict": if failed { "FAIL" } else { "PASS" },
                            "issues": issues,
                        });
                        println!("{v:#}");
                    } else {
                        match verdict {
                            Verdict::Pass => {
                                println!("PASS: the judge found no unsupported claims.")
                            }
                            Verdict::Fail(issues) => {
                                println!("FAIL: {} problem claim(s)", issues.len());
                                for issue in issues {
                                    println!("  - {}", issue.claim);
                                    println!("    {}", issue.explanation);
                                }
                            }
                        }
                    }
                    if failed {
                        ExitCode::from(1)
                    } else {
                        ExitCode::SUCCESS
                    }
                }
                Err(e) => fail(&e.to_string()),
            }
        }
        "critique" => {
            let code = match read_input(rest, true) {
                Ok(t) if !t.trim().is_empty() => t,
                Ok(_) => return fail("input is empty"),
                Err(e) => return fail(&e),
            };
            match critique_code(&code).await {
                Ok(report) => {
                    if json {
                        let v = serde_json::json!({
                            "risks": report.risks,
                            "improvements": report.improvements,
                            "missing_tests": report.missing_tests,
                        });
                        println!("{v:#}");
                    } else {
                        for (title, items) in [
                            ("Risks", &report.risks),
                            ("Improvements", &report.improvements),
                            ("Missing tests", &report.missing_tests),
                        ] {
                            println!("{title}:");
                            if items.is_empty() {
                                println!("  (none)");
                            }
                            for item in items {
                                println!("  - {item}");
                            }
                        }
                    }
                    ExitCode::SUCCESS
                }
                Err(e) => fail(&e.to_string()),
            }
        }
        other => fail(&format!(
            "unknown command '{other}'; run llm_affector --help"
        )),
    }
}

fn fail(msg: &str) -> ExitCode {
    eprintln!("error: {msg}");
    ExitCode::from(2)
}
