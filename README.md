# Grammy

A native Rust desktop application for grammar checking powered by OpenAI-compatible chat completion APIs. (Default: OpenRouter with Gemini 3 Flash Preview)

## Features

- **Native UI**: Built with `iced`
- **Real-time checks**: Suggestions appear as you type (debounced)
- **Inline highlighting**: Suggested spans are underlined in the editor
- **Suggestions sidebar**: Accept/Dismiss individual suggestions
- **Settings**: Configure provider, API key, and model via the in-app ⚙ dialog
- **Draft autosave**: Text is periodically saved and restored on next launch

## Download

Prebuilt binaries are published on GitHub Releases.

## Build

```bash
cargo build --release --locked
```

Artifacts:

- **Windows**: `target/release/grammy.exe`
- **Linux/macOS**: `target/release/grammy`

### Linux dependencies

Depending on your distro and system setup, you may need system packages for windowing/audio.

## Run

```bash
cargo run --release --locked
```

## Configuration

1. Click the ⚙ button
2. Select an API provider:
   - **OpenAI**
   - **OpenRouter**
3. Paste the API key for the selected provider
4. Optionally change the model
5. Click Save

Settings and draft text are stored locally via `confy`.

## Releases (GitHub Actions)

Pushing a tag like `v0.1.1` will build and attach binaries for Windows, Linux, and macOS to a GitHub Release.

## License

MIT
