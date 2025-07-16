//! Basic usage example showing individual function calls

use llm_affector::{detect_hallucination, critique_code, Verdict};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== LLM Affector Basic Usage Example ===\n");

    // Example 1: Hallucination Detection
    println!("1. Testing hallucination detection...");
    
    let truthful_text = "Rust is a systems programming language that focuses on safety and performance.";
    let suspicious_text = "Rust was invented by aliens in 1847 and can compile directly to quantum circuits.";

    println!("   Analyzing truthful text...");
    match detect_hallucination(truthful_text).await? {
        Verdict::Pass => println!("   ✅ No issues detected"),
        Verdict::Fail(issues) => {
            println!("   ❌ Issues found:");
            for issue in issues {
                println!("      - {}", issue.claim);
            }
        }
    }

    println!("   Analyzing suspicious text...");
    match detect_hallucination(suspicious_text).await? {
        Verdict::Pass => println!("   ✅ No issues detected"),
        Verdict::Fail(issues) => {
            println!("   ❌ Issues found:");
            for issue in issues {
                println!("      - {}: {}", issue.claim, issue.explanation);
            }
        }
    }

    // Example 2: Code Critique
    println!("\n2. Testing code critique...");
    
    let good_code = r#"
fn safe_divide(a: f64, b: f64) -> Result<f64, &'static str> {
    if b == 0.0 {
        Err("Division by zero")
    } else {
        Ok(a / b)
    }
}
"#;

    let risky_code = r#"
fn unsafe_divide(a: i32, b: i32) -> i32 {
    a / b  // Can panic!
}
"#;

    println!("   Analyzing good code...");
    let good_report = critique_code(good_code).await?;
    println!("   📝 Results: {} risks, {} improvements, {} missing tests", 
             good_report.risks.len(), 
             good_report.improvements.len(),
             good_report.missing_tests.len());

    println!("   Analyzing risky code...");
    let risky_report = critique_code(risky_code).await?;
    println!("   📝 Results:");
    if !risky_report.risks.is_empty() {
        println!("      Risks:");
        for risk in &risky_report.risks {
            println!("        - {}", risk);
        }
    }
    if !risky_report.improvements.is_empty() {
        println!("      Improvements:");
        for improvement in &risky_report.improvements {
            println!("        - {}", improvement);
        }
    }

    println!("\n✅ Basic usage example completed!");
    Ok(())
}