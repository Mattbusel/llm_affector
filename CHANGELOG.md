# Changelog

## 0.2.0 (2026-09-25)

- New command-line tool: `llm_affector check <text|--file PATH|->` and `llm_affector critique <PATH|->`, with `--json`, `--help`, `--version` and exit code 1 when hallucinations are found. Replaces the hard-coded demo binary.
- `LLM_BASE_URL`, `LLM_MODEL` and `LLM_TIMEOUT_SECONDS` are now read (they were documented in `.env.example` but ignored). Default model is now `gpt-4o-mini`.
- Prebuilt binaries for Windows, macOS (Apple Silicon and Intel) and Linux on every GitHub Release, with SHA256SUMS.txt.
- License metadata set to MIT to match the LICENSE file. Cargo.lock is committed.

## 0.1.0

- Initial release: `detect_hallucination` and `critique_code`.
