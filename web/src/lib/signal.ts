import type { ClientMsg, ServerMsg } from './proto';

export interface SignalEvents {
  open(): void;
  message(m: ServerMsg): void;
  close(info: { willRetry: boolean }): void;
}

/** WebSocket client with heartbeat and exponential-backoff reconnect. */
export interface Signal { connect(): void; send(m: ClientMsg): void; close(): void }

export class SignalClient implements Signal {
  private ws: WebSocket | null = null;
  private attempt = 0;
  private stopped = false;
  private timers: number[] = [];

  constructor(private url: string, private ev: SignalEvents, private reconnect = true) {}

  connect(): void {
    this.stopped = false;
    const ws = new WebSocket(this.url);
    this.ws = ws;
    ws.onopen = () => {
      this.attempt = 0;
      this.timers.push(window.setInterval(() => this.send({ type: 'ping' }), 25_000));
      this.ev.open();
    };
    ws.onmessage = (e) => {
      try { this.ev.message(JSON.parse(String(e.data)) as ServerMsg); } catch { /* ignore malformed */ }
    };
    ws.onclose = () => {
      this.clear();
      const willRetry = this.reconnect && !this.stopped;
      this.ev.close({ willRetry });
      if (willRetry) {
        const delay = Math.min(10_000, 500 * 2 ** this.attempt++) + Math.random() * 300;
        this.timers.push(window.setTimeout(() => this.connect(), delay));
      }
    };
  }

  send(m: ClientMsg): void {
    if (this.ws?.readyState === WebSocket.OPEN) this.ws.send(JSON.stringify(m));
  }

  close(): void {
    this.stopped = true;
    this.clear();
    this.ws?.close(1000);
  }

  private clear(): void {
    this.timers.forEach((t) => { clearInterval(t); clearTimeout(t); });
    this.timers = [];
  }
}

export type SignallingMode = 'server' | 'p2p';

/** server = our Rust signalling server; p2p = serverless rendezvous (used on GitHub Pages). Override with PUBLIC_SIGNALLING. */
export function signallingMode(): SignallingMode {
  const forced = import.meta.env.PUBLIC_SIGNALLING as string | undefined;
  if (forced === 'p2p' || forced === 'server') return forced;
  if (import.meta.env.PUBLIC_SIGNAL_URL) return 'server';
  return location.hostname.endsWith('.github.io') ? 'p2p' : 'server';
}

export function signalUrl(): string {
  const configured = import.meta.env.PUBLIC_SIGNAL_URL as string | undefined;
  return configured || `${location.protocol === 'https:' ? 'wss:' : 'ws:'}//${location.host}/ws`;
}

/** The serverless transport is loaded lazily so server-mode builds never download it. */
export async function createSignal(role: 'host' | 'viewer', ev: SignalEvents, reconnect = true): Promise<Signal> {
  if (signallingMode() === 'p2p') {
    const { P2PHost, P2PViewer } = await import('./p2p');
    return role === 'host' ? new P2PHost(ev) : new P2PViewer(ev);
  }
  return new SignalClient(signalUrl(), ev, reconnect);
}
