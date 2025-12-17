const textInput = document.getElementById('textInput');
const statusEl = document.getElementById('status');
const suggestionsEl = document.getElementById('suggestions');

let lastMatches = [];
let lastText = '';
let checkTimeout = null;
let isChecking = false;
let activePopup = null;
let popupHideTimeout = null;

const DEBOUNCE_MS = 600;

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

async function apiPost(path, body) {
  const res = await fetch(path, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  });

  const data = await res.json().catch(() => null);
  if (!res.ok) {
    const msg = data && data.error ? data.error : `Request failed (${res.status})`;
    throw new Error(msg);
  }
  return data;
}

async function checkText() {
  const text = getTextContent();
  if (text === lastText && lastMatches.length > 0) return;
  if (!text.trim()) {
    lastMatches = [];
    lastText = text;
    setStatus('Ready');
    return;
  }

  isChecking = true;
  setStatus('Checking...');

  try {
    const data = await apiPost('/api/check', { text });
    lastMatches = data.matches || [];
    lastText = text;
    renderWithHighlights(text, lastMatches);
    renderSidebar(lastMatches);
    setStatus(lastMatches.length ? `${lastMatches.length} suggestion(s)` : 'All good!');
  } catch (err) {
    setStatus(err.message);
  } finally {
    isChecking = false;
  }
}

function scheduleCheck() {
  if (checkTimeout) clearTimeout(checkTimeout);
  checkTimeout = setTimeout(() => {
    checkText();
  }, DEBOUNCE_MS);
}

async function applySuggestion(suggestion) {
  hidePopup();
  const text = getTextContent();

  try {
    const data = await apiPost('/api/apply', { text, suggestion });
    const newText = data.text;

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
  } catch (err) {
    setStatus(err.message);
    // Text changed, trigger re-check
    scheduleCheck();
  }
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
