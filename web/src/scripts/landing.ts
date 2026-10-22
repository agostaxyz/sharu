import { extractCode, normalizeCode } from '../lib/code';
import { withBase } from '../lib/base';

const form = document.getElementById('join-form') as HTMLFormElement;
const input = document.getElementById('code') as HTMLInputElement;
const error = document.getElementById('code-error') as HTMLElement;

form.addEventListener('submit', (e) => {
  e.preventDefault();
  const code = extractCode(input.value);
  if (!normalizeCode(code)) {
    error.textContent = 'That does not look like a session code. It has three or four words and a two-digit number, like Caesar-23-Pyramid.';
    input.setAttribute('aria-invalid', 'true');
    input.focus();
    return;
  }
  location.assign(withBase(`/${encodeURIComponent(code)}`));
});
input.addEventListener('input', () => { error.textContent = ''; input.removeAttribute('aria-invalid'); });

if (!navigator.mediaDevices?.getDisplayMedia) {
  document.getElementById('share-action')?.setAttribute('hidden', '');
  document.getElementById('share-unsupported')?.removeAttribute('hidden');
}
