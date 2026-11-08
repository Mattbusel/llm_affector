# llm_affector

A second LLM checks the first and returns pass or fail.

- Flags false claims in text.
- Checks answers against your own docs.
- Reviews Rust code for risks.
- Exits 1 on problem claims, so it can gate CI.
- Works with OpenAI or a local Ollama.

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
