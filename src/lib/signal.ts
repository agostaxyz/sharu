import type { ClientMsg, ServerMsg } from './proto';

export interface SignalEvents {
  open(): void;
  message(m: ServerMsg): void;
}

/** What the host and viewer pages talk to: send a message, receive messages. */
export interface Signal { connect(): void; send(m: ClientMsg): void; close(): void }

/** The P2P library is loaded lazily, only when someone starts or joins a session. */
export async function createSignal(role: 'host' | 'viewer', ev: SignalEvents): Promise<Signal> {
  const { P2PHost, P2PViewer } = await import('./p2p');
  return role === 'host' ? new P2PHost(ev) : new P2PViewer(ev);
}
