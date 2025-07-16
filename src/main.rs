use llm_affector::{detect_hallucination, critique_code, Verdict};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Sample data for demonstration
    let suspicious_text = "Scientists have proven that coffee beans grow on the moon and are harvested by trained dolphins.";
    
    let risky_code = r#"
use std::fs::File;
use std::io::Read;

fn read_file_contents(filename: &str) -> String {
    let mut file = File::open(filename).unwrap();
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();
    contents
}
"#;

    println!("Running concurrent LLM analysis...");
    
    // Execute both analyses concurrently for optimal performance
    let (hallucination_result, critique_result) = tokio::join!(
        detect_hallucination(suspicious_text),
        critique_code(risky_code)
    );

    // Process hallucination detection results
    match hallucination_result {
        Ok(Verdict::Pass) => {
            println!("✅ Text analysis: No hallucinations detected");
        }
        Ok(Verdict::Fail(issues)) => {
            println!("❌ Text analysis: Hallucinations detected!");
            for issue in issues {
                println!("  - Claim: {}", issue.claim);
                println!("    Explanation: {}", issue.explanation);
            }
        }
        Err(e) => {
            eprintln!("❌ Text analysis failed: {}", e);
        }
    }

    // Process code critique results
    match critique_result {
        Ok(report) => {
            println!("\n📝 Code critique results:");
            
            if !report.risks.is_empty() {
                println!("  Risks identified:");
                for risk in report.risks {
                    println!("    - {}", risk);
                }
            }
            
            if !report.improvements.is_empty() {
                println!("  Suggested improvements:");
                for improvement in report.improvements {
                    println!("    - {}", improvement);
                }
            }
            
            if !report.missing_tests.is_empty() {
                println!("  Missing tests:");
                for test in report.missing_tests {
                    println!("    - {}", test);
                }
            }
        }
        Err(e) => {
            eprintln!("❌ Code critique failed: {}", e);
        }
    }

    Ok(())
}