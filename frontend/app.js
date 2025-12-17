const checkBtn = document.getElementById('checkBtn');
const textInput = document.getElementById('textInput');
const suggestionsEl = document.getElementById('suggestions');
const statusEl = document.getElementById('status');

let lastMatches = [];

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

function renderMatches(matches) {
  lastMatches = matches;
  suggestionsEl.innerHTML = '';

  if (!matches.length) {
    suggestionsEl.innerHTML = '<div class="card"><div class="cardTitle">No suggestions.</div></div>';
    return;
  }

  for (const m of matches) {
    const card = document.createElement('div');
    card.className = 'card';

    const title = document.createElement('div');
    title.className = 'cardTitle';
    title.textContent = m.message;

    const snippet = document.createElement('div');
    snippet.className = 'snippet';
    snippet.innerHTML =
      `Original: ${escapeHtml(m.original)}\n` +
      `Replace:  ${escapeHtml(m.replacement)}`;

    const footer = document.createElement('div');
    footer.className = 'cardFooter';

    const accept = document.createElement('button');
    accept.className = 'btn primary';
    accept.textContent = 'Accept';
    accept.addEventListener('click', async () => {
      await applySuggestion(m);
    });

    const pill = document.createElement('div');
    pill.className = 'pill';
    pill.textContent = m.rule;

    footer.appendChild(accept);
    footer.appendChild(pill);

    card.appendChild(title);
    card.appendChild(snippet);
    card.appendChild(footer);

    suggestionsEl.appendChild(card);
  }
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
  const text = textInput.value || '';
  checkBtn.disabled = true;
  setStatus('Checking...');

  try {
    const data = await apiPost('/api/check', { text });
    renderMatches(data.matches || []);
    setStatus(`${(data.matches || []).length} suggestion(s)`);
  } catch (err) {
    setStatus(err.message);
  } finally {
    checkBtn.disabled = false;
  }
}

async function applySuggestion(suggestion) {
  const text = textInput.value || '';
  checkBtn.disabled = true;
  setStatus('Applying...');

  try {
    const data = await apiPost('/api/apply', { text, suggestion });
    textInput.value = data.text;
    renderMatches(data.matches || []);
    setStatus(`${(data.matches || []).length} suggestion(s)`);
  } catch (err) {
    setStatus(err.message);
  } finally {
    checkBtn.disabled = false;
  }
}

checkBtn.addEventListener('click', checkText);

setStatus('Ready');
renderMatches([]);
