const textInput = document.getElementById('textInput');
const statusEl = document.getElementById('status');
const suggestionsEl = document.getElementById('suggestions');
const settingsBtn = document.getElementById('settingsBtn');
const settingsModal = document.getElementById('settingsModal');
const apiKeyInput = document.getElementById('apiKeyInput');
const modelInput = document.getElementById('modelInput');
const saveSettingsBtn = document.getElementById('saveSettingsBtn');
const closeSettingsBtn = document.getElementById('closeSettingsBtn');

let lastMatches = [];
let lastText = '';
let checkTimeout = null;
let currentAbortController = null;
let activePopup = null;
let popupHideTimeout = null;

const DEBOUNCE_MS = 600;
const STORAGE_KEY_API = 'grammy_api_key';
const STORAGE_KEY_MODEL = 'grammy_model';
const DEFAULT_MODEL = 'gpt-4o-mini';

// Settings management
function getApiKey() {
  return localStorage.getItem(STORAGE_KEY_API) || '';
}

function setApiKey(key) {
  localStorage.setItem(STORAGE_KEY_API, key);
}

function getModel() {
  return localStorage.getItem(STORAGE_KEY_MODEL) || DEFAULT_MODEL;
}

function setModel(model) {
  localStorage.setItem(STORAGE_KEY_MODEL, model || DEFAULT_MODEL);
}

function openSettings() {
  apiKeyInput.value = getApiKey();
  modelInput.value = getModel();
  settingsModal.classList.add('open');
}

function closeSettings() {
  settingsModal.classList.remove('open');
}

function saveSettings() {
  setApiKey(apiKeyInput.value.trim());
  setModel(modelInput.value.trim());
  closeSettings();
  setStatus('Settings saved');
}

settingsBtn.addEventListener('click', openSettings);
closeSettingsBtn.addEventListener('click', closeSettings);
saveSettingsBtn.addEventListener('click', saveSettings);
settingsModal.addEventListener('click', (e) => {
  if (e.target === settingsModal) closeSettings();
});

function setStatus(text) {
  statusEl.textContent = text;
}

function escapeHtml(str) {
  return str
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
    .replaceAll("'", '&#039;');
}

function getTextContent() {
  return textInput.innerText || '';
}

function setTextContent(text) {
  textInput.innerText = text;
}

// Render text with highlighted suggestions
function renderWithHighlights(text, matches) {
  if (!matches.length) {
    textInput.innerHTML = escapeHtml(text) || '';
    return;
  }

  // Sort by offset
  const sorted = [...matches].sort((a, b) => a.offset - b.offset);
  let html = '';
  let pos = 0;

  for (const m of sorted) {
    if (m.offset > pos) {
      html += escapeHtml(text.slice(pos, m.offset));
    }
    const original = text.slice(m.offset, m.offset + m.length);
    html += `<span class="suggestion" data-id="${m.id}">${escapeHtml(original)}</span>`;
    pos = m.offset + m.length;
  }

  if (pos < text.length) {
    html += escapeHtml(text.slice(pos));
  }

  // Save selection
  const sel = window.getSelection();
  let savedOffset = 0;
  if (sel.rangeCount > 0) {
    const range = sel.getRangeAt(0);
    savedOffset = getCaretOffset(textInput, range);
  }

  textInput.innerHTML = html;

  // Restore caret
  restoreCaret(textInput, savedOffset);
}

function getCaretOffset(element, range) {
  const preRange = range.cloneRange();
  preRange.selectNodeContents(element);
  preRange.setEnd(range.startContainer, range.startOffset);
  return preRange.toString().length;
}

function restoreCaret(element, offset) {
  const sel = window.getSelection();
  const walker = document.createTreeWalker(element, NodeFilter.SHOW_TEXT, null);
  let current = 0;
  let node;

  while ((node = walker.nextNode())) {
    const len = node.textContent.length;
    if (current + len >= offset) {
      const range = document.createRange();
      range.setStart(node, Math.min(offset - current, len));
      range.collapse(true);
      sel.removeAllRanges();
      sel.addRange(range);
      return;
    }
    current += len;
  }
}

// Render sidebar suggestions list
function renderSidebar(matches) {
  suggestionsEl.innerHTML = '';

  if (!matches.length) {
    suggestionsEl.innerHTML = '<div class="noSuggestions">No suggestions</div>';
    return;
  }

  for (const m of matches) {
    const card = document.createElement('div');
    card.className = 'card';
    card.dataset.id = m.id;

    const msg = document.createElement('div');
    msg.className = 'cardMsg';
    msg.textContent = m.message;

    const body = document.createElement('div');
    body.className = 'cardBody';

    const original = document.createElement('span');
    original.className = 'cardOriginal';
    original.textContent = m.original;

    const arrow = document.createElement('span');
    arrow.className = 'cardArrow';
    arrow.textContent = '→';

    const replacement = document.createElement('span');
    replacement.className = 'cardReplacement';
    replacement.textContent = m.replacement;

    body.appendChild(original);
    body.appendChild(arrow);
    body.appendChild(replacement);

    const acceptBtn = document.createElement('button');
    acceptBtn.className = 'cardAccept';
    acceptBtn.textContent = 'Accept';
    acceptBtn.addEventListener('click', (e) => {
      e.stopPropagation();
      applySuggestion(m);
    });

    card.appendChild(msg);
    card.appendChild(body);
    card.appendChild(acceptBtn);

    // Highlight corresponding text on hover
    card.addEventListener('mouseenter', () => {
      const el = textInput.querySelector(`.suggestion[data-id="${m.id}"]`);
      if (el) el.classList.add('highlight');
    });
    card.addEventListener('mouseleave', () => {
      const el = textInput.querySelector(`.suggestion[data-id="${m.id}"]`);
      if (el) el.classList.remove('highlight');
    });

    suggestionsEl.appendChild(card);
  }
}

function hidePopup(immediate = false) {
  if (popupHideTimeout) {
    clearTimeout(popupHideTimeout);
    popupHideTimeout = null;
  }
  
  if (immediate) {
    if (activePopup) {
      activePopup.remove();
      activePopup = null;
    }
  } else {
    // Delay hiding to allow mouse to reach popup
    popupHideTimeout = setTimeout(() => {
      if (activePopup) {
        activePopup.remove();
        activePopup = null;
      }
    }, 150);
  }
}

function cancelHidePopup() {
  if (popupHideTimeout) {
    clearTimeout(popupHideTimeout);
    popupHideTimeout = null;
  }
}

function showPopup(suggestionEl, match) {
  cancelHidePopup();
  
  if (activePopup && activePopup.dataset.matchId === match.id) {
    return; // Already showing this popup
  }
  
  hidePopup(true);

  const popup = document.createElement('div');
  popup.className = 'popup';
  popup.dataset.matchId = match.id;

  const msg = document.createElement('div');
  msg.className = 'popupMsg';
  msg.textContent = match.message;

  const body = document.createElement('div');
  body.className = 'popupBody';

  const replacement = document.createElement('span');
  replacement.className = 'popupReplacement';
  replacement.textContent = match.replacement;

  const btn = document.createElement('button');
  btn.className = 'popupBtn';
  btn.textContent = 'Accept';
  btn.addEventListener('click', (e) => {
    e.stopPropagation();
    applySuggestion(match);
  });

  body.appendChild(replacement);
  body.appendChild(btn);
  popup.appendChild(msg);
  popup.appendChild(body);

  // Keep popup visible when hovering over it
  popup.addEventListener('mouseenter', cancelHidePopup);
  popup.addEventListener('mouseleave', () => hidePopup());

  document.body.appendChild(popup);

  // Position popup
  const rect = suggestionEl.getBoundingClientRect();
  popup.style.left = `${rect.left + window.scrollX}px`;
  popup.style.top = `${rect.bottom + window.scrollY + 6}px`;

  activePopup = popup;
}

// OpenAI API integration
const SYSTEM_PROMPT = `You are a careful English writing assistant.
Your job: suggest minimal edits for grammar, clarity, and phrases that sound non-native/awkward.
Rules:
- Do NOT rewrite the whole text.
- Only propose small localized edits (replace a short span with a short span).
- Preserve the author's voice and meaning.
- Prefer fewer suggestions over many.

Return ONLY valid JSON with this exact schema:
{
  "matches": [
    {
      "message": "...",
      "start": 0,
      "end": 0,
      "replacement": "..."
    }
  ]
}

Where start/end are CHARACTER indices (Unicode scalar value count) into the ORIGINAL input text. end is exclusive.
If there is nothing to change, return {"matches": []}.`;

function generateId() {
  return crypto.randomUUID();
}

function convertLlmMatchesToSuggestions(text, matches) {
  const chars = [...text];
  const charLen = chars.length;
  
  // Build byte boundaries from char indices
  const boundaries = [];
  let bytePos = 0;
  for (const char of chars) {
    boundaries.push(bytePos);
    bytePos += char.length;
  }
  boundaries.push(bytePos);

  const out = [];
  for (const m of matches) {
    if (m.start > m.end || m.end > charLen) continue;
    
    const startB = boundaries[m.start];
    const endB = boundaries[m.end];
    
    if (startB === undefined || endB === undefined) continue;
    if (startB > endB || endB > text.length) continue;
    
    const original = text.slice(startB, endB);
    if (original === m.replacement) continue;
    
    out.push({
      id: generateId(),
      message: m.message,
      offset: startB,
      length: endB - startB,
      original: original,
      replacement: m.replacement,
      rule: 'llm'
    });
  }

  out.sort((a, b) => a.offset - b.offset);

  // Filter overlapping
  const filtered = [];
  let lastEnd = 0;
  for (const s of out) {
    const end = s.offset + s.length;
    if (s.offset < lastEnd) continue;
    lastEnd = end;
    filtered.push(s);
  }

  return filtered;
}

async function callOpenAI(text, signal) {
  const apiKey = getApiKey();
  if (!apiKey) {
    throw new Error('API key not set. Click ⚙️ to configure.');
  }

  const model = getModel();
  const url = 'https://api.openai.com/v1/chat/completions';

  const body = {
    model: model,
    messages: [
      { role: 'system', content: SYSTEM_PROMPT },
      { role: 'user', content: `Text:\n${text}` }
    ],
    response_format: { type: 'json_object' }
  };

  const res = await fetch(url, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'Authorization': `Bearer ${apiKey}`
    },
    body: JSON.stringify(body),
    signal
  });

  if (!res.ok) {
    const data = await res.json().catch(() => null);
    const msg = data?.error?.message || `OpenAI API error (${res.status})`;
    throw new Error(msg);
  }

  const data = await res.json();
  const content = data.choices?.[0]?.message?.content || '{"matches":[]}';
  
  let parsed;
  try {
    parsed = JSON.parse(content);
  } catch {
    throw new Error('Invalid JSON response from LLM');
  }

  return convertLlmMatchesToSuggestions(text, parsed.matches || []);
}

async function checkText() {
  const text = getTextContent();
  if (text === lastText && lastMatches.length > 0) return;
  if (!text.trim()) {
    lastMatches = [];
    lastText = text;
    setStatus('Ready');
    renderSidebar([]);
    return;
  }

  // Abort any in-flight request
  if (currentAbortController) {
    currentAbortController.abort();
  }
  currentAbortController = new AbortController();
  const signal = currentAbortController.signal;
  const textAtStart = text;

  setStatus('Checking...');

  try {
    const matches = await callOpenAI(text, signal);
    
    // Check if text changed while we were waiting
    const currentText = getTextContent();
    if (currentText !== textAtStart) {
      // Text changed, discard results - a new check will be scheduled
      return;
    }

    lastMatches = matches;
    lastText = text;
    renderWithHighlights(text, lastMatches);
    renderSidebar(lastMatches);
    setStatus(lastMatches.length ? `${lastMatches.length} suggestion(s)` : 'All good!');
  } catch (err) {
    if (err.name === 'AbortError') {
      // Request was aborted, ignore
      return;
    }
    setStatus(err.message);
  } finally {
    currentAbortController = null;
  }
}

function scheduleCheck() {
  if (checkTimeout) clearTimeout(checkTimeout);
  
  // Abort ongoing request when user types
  if (currentAbortController) {
    currentAbortController.abort();
    currentAbortController = null;
  }
  
  checkTimeout = setTimeout(() => {
    checkText();
  }, DEBOUNCE_MS);
}

function applySuggestion(suggestion) {
  hidePopup();
  const text = getTextContent();

  const start = suggestion.offset;
  const end = suggestion.offset + suggestion.length;

  // Validate range
  if (start > text.length || end > text.length || start > end) {
    setStatus('Invalid suggestion range');
    scheduleCheck();
    return;
  }

  // Check if text still matches
  const slice = text.slice(start, end);
  if (slice !== suggestion.original) {
    setStatus('Text changed; re-checking...');
    scheduleCheck();
    return;
  }

  // Apply replacement
  const newText = text.slice(0, start) + suggestion.replacement + text.slice(end);

  // Adjust remaining suggestions' offsets
  const delta = suggestion.replacement.length - suggestion.length;
  const updatedMatches = lastMatches
    .filter(m => m.id !== suggestion.id)
    .map(m => {
      if (m.offset > suggestion.offset) {
        return { ...m, offset: m.offset + delta };
      }
      return m;
    });

  lastMatches = updatedMatches;
  lastText = newText;
  renderWithHighlights(newText, lastMatches);
  renderSidebar(lastMatches);
  setStatus(lastMatches.length ? `${lastMatches.length} suggestion(s)` : 'All good!');
}

// Event listeners
textInput.addEventListener('input', () => {
  // Clear highlights on edit to avoid stale underlines
  const text = getTextContent();
  if (text !== lastText) {
    lastMatches = [];
    textInput.innerHTML = escapeHtml(text);
    restoreCaret(textInput, text.length);
  }
  scheduleCheck();
});

textInput.addEventListener('mouseover', (e) => {
  const el = e.target.closest('.suggestion');
  if (el) {
    const id = el.dataset.id;
    const match = lastMatches.find(m => m.id === id);
    if (match) showPopup(el, match);
  }
});

textInput.addEventListener('mouseout', (e) => {
  const el = e.target.closest('.suggestion');
  if (el) {
    hidePopup(); // Will delay hiding
  }
});

document.addEventListener('click', (e) => {
  if (activePopup && !activePopup.contains(e.target) && !e.target.closest('.suggestion')) {
    hidePopup();
  }
});

// Handle paste as plain text
textInput.addEventListener('paste', (e) => {
  e.preventDefault();
  const text = e.clipboardData.getData('text/plain');
  document.execCommand('insertText', false, text);
});

setStatus('Ready');
renderSidebar([]);
