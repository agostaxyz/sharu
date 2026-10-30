import { extractCode, normalizeCode } from '../lib/code';
import { withBase } from '../lib/base';

const form = document.getElementById('join-form') as HTMLFormElement;
const input = document.getElementById('code') as HTMLInputElement;
const error = document.getElementById('code-error') as HTMLElement;

form.addEventListener('submit', (e) => {
  e.preventDefault();
  const code = extractCode(input.value);
  if (!normalizeCode(code)) {
    error.textContent = 'That does not look like a session link. Paste the whole link the host sent you.';
    input.setAttribute('aria-invalid', 'true');
    input.focus();
    return;
  }
  location.assign(withBase(`/${encodeURIComponent(code)}`));
});
input.addEventListener('input', () => { error.textContent = ''; input.removeAttribute('aria-invalid'); });

// Audio capture limits: shown on the home page for the browsers they affect.
const ua = navigator.userAgent;
const firefox = /Firefox\//.test(ua);
const linux = /Linux/.test(ua) && !/Android/.test(ua);
const kind = firefox && linux ? 'both' : firefox ? 'firefox' : linux ? 'linux' : null;
if (kind) document.querySelector(`[data-audio="${kind}"]`)?.removeAttribute('hidden');
