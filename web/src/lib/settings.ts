const get = (k: string): string | null => { try { return localStorage.getItem(k); } catch { return null; } };
const set = (k: string, v: string) => { try { localStorage.setItem(k, v); } catch { /* storage unavailable */ } };
const del = (k: string) => { try { localStorage.removeItem(k); } catch { /* ignore */ } };

export const getNickname = () => get('sharu.nickname') ?? '';
export const setNickname = (n: string) => set('sharu.nickname', n);

export interface HostSession { code: string; token: string }
export function loadHostSession(): HostSession | null {
  try { const v = JSON.parse(get('sharu.host') ?? 'null'); return v?.code && v?.token ? v : null; } catch { return null; }
}
export const saveHostSession = (s: HostSession) => set('sharu.host', JSON.stringify(s));
export const clearHostSession = () => del('sharu.host');
