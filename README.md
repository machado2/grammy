# Grammy

A small Grammarly-like web app (conservative suggestions + click-to-apply) with a Rust backend.

## Run

1. Install Rust (stable) if you don’t have it.
2. Start the server:

```bash
cargo run
```

Run it from `backend/`.

3. Open:

http://127.0.0.1:3000

## Tests

Run from `backend/`:

```bash
cargo test
```

### Live API smoke test (optional)

There is an opt-in live test that calls the real API (costs tokens). It is ignored by default.

Run it from `backend/`:

```powershell
$env:GRAMMY_RUN_LIVE_TESTS = "1"
# Uses OPENAI_API_KEY by default (or GRAMMY_LLM_API_KEY override)
cargo test -- --ignored
```

Note (Windows): if the server is currently running, `cargo run` / `cargo test` may fail with an "Access denied" error when it tries to overwrite `target\\debug\\grammy_backend.exe`. Stop the running server first.

## LLM suggestions

This app is LLM-powered: it asks a model for conservative, localized edits (good for catching "this sounds weird" phrasing).

Environment variables:

- **`OPENAI_API_KEY`**
  - Required.
  - Uses an OpenAI-compatible `chat/completions` endpoint.
- **`GRAMMY_LLM_API_KEY`** (optional)
  - Overrides `OPENAI_API_KEY` if set.
- **`GRAMMY_LLM_API_BASE`** (optional)
  - Default: `https://api.openai.com/v1`
- **`GRAMMY_LLM_MODEL`** (optional)
  - Default: `gpt-5-mini-2025-08-07`

Example (PowerShell):

```powershell
$env:OPENAI_API_KEY = "YOUR_KEY_HERE"
$env:GRAMMY_LLM_MODEL = "gpt-5-mini-2025-08-07"
cargo run
```

## What it does

- `POST /api/check` returns a list of LLM-generated suggestions.
- `POST /api/apply` applies one suggestion (with conflict detection) and returns updated text + refreshed suggestions.

## Notes

This intentionally does not “rewrite” your text. It only proposes small mechanical fixes you can accept or ignore.
