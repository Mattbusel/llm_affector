# llm_affector: a model that grades its own answer passes its own mistakes

**Why this exists:** "Check your answer" prompts ask the model that wrote the claim whether the claim is true. It agrees with itself, and the hallucination ships.

**Why this instead of the usual approach:** Self-consistency and self-critique reuse the same context and the same blind spots. llm_affector sends the answer to a second model call that did not write it, optionally with your source documents, and gets back each unsupported or false claim with a pass/fail verdict. The CLI exits 1 on a failed check, so it can gate CI.

**Why adopt it today:** `cargo install llm_affector` and `llm_affector check "<answer>" --source docs/policy.md` in CI, or call it as a library after each generation. Works with OpenAI or a local Ollama.

**Built for:** teams putting LLM answers in front of users or into docs who need a hard fail on unsupported claims, not a confidence score.

**Tradeoffs we chose on purpose:**

- It costs a second model call per check. That is the price of a judge that did not write the answer.
- The judge can be wrong too; grounding it in your sources is what makes the verdict checkable.

## Install

```bash
cargo install llm_affector
```

## Use

```bash
export LLM_API_KEY=sk-...
llm_affector check "The Eiffel Tower was finished in 1999."
```

[crates.io](https://crates.io/crates/llm_affector)

License: MIT, see [LICENSE](LICENSE).

More: [docs/DETAILS.md](docs/DETAILS.md)
