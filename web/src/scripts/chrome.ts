const btn = document.getElementById('font-toggle') as HTMLButtonElement | null;
if (btn) {
  const root = document.documentElement;
  const sync = () => btn.setAttribute('aria-pressed', String(root.dataset.font === 'dyslexic'));
  sync();
  btn.addEventListener('click', () => {
    const next = root.dataset.font === 'dyslexic' ? 'default' : 'dyslexic';
    root.dataset.font = next;
    try { localStorage.setItem('sharu.font', next); } catch { /* ignore */ }
    sync();
  });
}

// Logo easter egg on the home page: a small "press and play" animation. Silent and purely decorative.
// On other pages the logo is part of a link to the home page, so there is nothing to wire up.
const logo = document.getElementById('brand-logo');
if (logo) {
  logo.addEventListener('click', () => {
    logo.classList.remove('logo-pop');
    void logo.getBoundingClientRect(); // restart the animation if clicked again mid-way
    logo.classList.add('logo-pop');
  });
  logo.addEventListener('animationend', (e) => {
    if (e.animationName === 'logo-bounce') logo.classList.remove('logo-pop');
  });
}
