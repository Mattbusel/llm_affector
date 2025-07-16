//! # llm_affector
//! 
//! An async Rust library for LLM-based content analysis, providing hallucination
//! detection and code critique functionality.
//! 
//! ## Features
//! 
//! - **Hallucination Detection**: Analyze text for factual errors and unsupported claims
//! - **Code Critique**: Review Rust code for bugs, style issues, and missing tests
//! - **Async/Await Support**: Built on Tokio for high-performance concurrent operations
//! - **Multiple LLM Providers**: Currently supports OpenAI, easily extensible
//! 
//! ## Quick Start
//! 
//! ```rust,no_run
//! use llm_affector::{detect_hallucination, critique_code, Verdict};
//! 
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Set your API key in environment: LLM_API_KEY=your_key_here
//!     
//!     let text = "The Earth is flat and surrounded by ice walls.";
//!     let code = "fn divide(a: i32, b: i32) -> i32 { a / b }";
//!     
//!     // Run both analyses concurrently
//!     let (hallucination_result, critique_result) = tokio::join!(
//!         detect_hallucination(text),
//!         critique_code(code)
//!     );
//!     
//!     match hallucination_result? {
//!         Verdict::Pass => println!("✅ No hallucinations detected"),
//!         Verdict::Fail(issues) => {
//!             println!("❌ Hallucinations found:");
//!             for issue in issues {
//!                 println!("  - {}: {}", issue.claim, issue.explanation);
//!             }
//!         }
//!     }
//!     
//!     let report = critique_result?;
//!     println!("📝 Code critique: {} risks, {} improvements suggested", 
//!              report.risks.len(), report.improvements.len());
//!     
//!     Ok(())
//! }
//! ```

mod client;
mod types;
mod errors;
mod hallucination;
mod critique;

// Re-export public API
pub use client::LlmClient;
pub use types::{Verdict, Issue, CritiqueReport};
pub use errors::LlmAffectorError;
pub use hallucination::detect_hallucination;
pub use critique::critique_code;

// Type alias for convenience
pub type Result<T> = std::result::Result<T, LlmAffectorError>;