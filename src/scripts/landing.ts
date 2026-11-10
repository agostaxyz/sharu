// Audio capture limits: shown on the home page for the browsers they affect.
const ua = navigator.userAgent;
const firefox = /Firefox\//.test(ua);
const linux = /Linux/.test(ua) && !/Android/.test(ua);
const kind = firefox && linux ? 'both' : firefox ? 'firefox' : linux ? 'linux' : null;
if (kind) document.querySelector(`[data-audio="${kind}"]`)?.removeAttribute('hidden');
