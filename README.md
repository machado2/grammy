# Grammy

A small Grammarly-like web app (conservative suggestions + click-to-apply) that runs entirely in the browser.

## Run

Simply serve this folder with any static file server:

```bash
# Using Python
python -m http.server 8000

# Using Node.js (npx)
npx serve .

# Or just open index.html directly in your browser
```

Then open http://localhost:8000 (or the appropriate URL for your server).

## Setup

1. Click the ⚙️ settings button in the header
2. Enter your OpenAI API key
3. Optionally change the model (default: `gpt-5-mini`)
4. Click Save

Your API key is stored in your browser's localStorage and is only sent directly to OpenAI's API.

## How it works

- Type or paste text in the editor
- The app automatically checks your text after you stop typing
- Suggestions appear as underlined text and in the sidebar
- Click "Accept" to apply a suggestion

## Privacy

- **Frontend-only**: No backend server required
- **API key stored locally**: Your key never leaves your browser except when calling OpenAI
- **Direct API calls**: Requests go directly from your browser to `api.openai.com`

## Notes

This intentionally does not "rewrite" your text. It only proposes small mechanical fixes you can accept or ignore.
