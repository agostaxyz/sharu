const b = import.meta.env.BASE_URL.replace(/\/$/, '');
export const withBase = (path: string): string => `${b}${path}`;
export const stripBase = (pathname: string): string =>
  pathname.startsWith(b) ? pathname.slice(b.length) : pathname;
