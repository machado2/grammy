# Grammy Desktop

A native Rust desktop application for grammar checking powered by OpenAI, built with egui.

## Features

- **Native Performance**: Built with Rust and egui for fast, responsive UI
- **Real-time Grammar Checking**: Suggestions appear as you type (with debouncing)
- **Inline Highlighting**: Problematic text is highlighted in red with underlines
- **Suggestions Panel**: View all suggestions in a sidebar with one-click accept
- **Settings Persistence**: API key and model are stored locally via confy
- **Dark Theme**: Modern dark UI matching the original web app design

## Building

```bash
cd grammy-desktop
cargo build --release
```

The executable will be at `target/release/grammy.exe` (Windows) or `target/release/grammy` (Linux/macOS).

## Running

```bash
cargo run --release
```

Or run the built executable directly.

## Configuration

1. Click the ⚙ button in the top-right corner
2. Enter your OpenAI API Key
3. Optionally change the model (default: `gpt-4o-mini`)
4. Click Save

Your settings are stored locally in your system's config directory.

## Usage

1. Type or paste text into the editor
2. Wait ~600ms after typing for automatic grammar check
3. Suggestions appear highlighted in the text and listed in the sidebar
4. Click "Accept" on any suggestion to apply the correction

## Tech Stack

- **eframe/egui**: Native GUI framework
- **reqwest**: HTTP client for OpenAI API
- **tokio**: Async runtime for non-blocking API calls
- **serde**: JSON serialization
- **confy**: Configuration management

## License

MIT
