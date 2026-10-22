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
