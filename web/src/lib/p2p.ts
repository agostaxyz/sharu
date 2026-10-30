// SPDX-License-Identifier: AGPL-3.0-or-later
// Serverless transport (experimental). The host's browser plays the role of the signalling server:
// the session code is the rendezvous room (Trystero, public relays); media still uses lib/mesh.ts.
// Same Signal interface as the WebSocket client, so Host.svelte / Viewer.svelte do not care which is used.
import { joinRoom } from 'trystero';
import type { ClientMsg, ErrorCode, IceServer, ServerMsg } from './proto';
import type { Signal, SignalEvents } from './signal';
import { generateCode, normalizeCode } from './code';
import { defaultNickname, sanitizeNickname } from './nick';

const APP_ID = 'sharu-p2p-v1';
const MAX_VIEWERS = 4;
const ICE: IceServer[] = [{ urls: ['stun:stun.cloudflare.com:3478'] }];
const LOST_GRACE_S = 60;
const LEFT_GRACE_S = 10;
const JOIN_TIMEOUT_MS = 20_000;

type Send = (data: unknown, to?: string) => unknown;
type On = (cb: (data: unknown, from: string) => void) => void;
type Room = ReturnType<typeof joinRoom>;
type RoomAction = {
  send: (data: unknown, opts?: { target?: string | string[] }) => unknown;
  onMessage: ((data: unknown, meta: { peerId: string }) => void) | null;
};
// Trystero >= 0.25: makeAction() returns an object, not a [send, receive] tuple.
const action = (room: Room, name: string): { send: Send; on: On } => {
  const a = room.makeAction(name) as unknown as RoomAction;
  return {
    send: (data, to) => a.send(data, to ? { target: to } : undefined),
    on: (cb) => { a.onMessage = (data, meta) => cb(data, meta.peerId); },
  };
};
const hex = (b: Uint8Array) => [...b].map((x) => x.toString(16).padStart(2, '0')).join('');
const randomHex = (n: number) => hex(crypto.getRandomValues(new Uint8Array(n)));
const sha256 = async (s: string) => hex(new Uint8Array(await crypto.subtle.digest('SHA-256', new TextEncoder().encode(s))));
const err = (code: ErrorCode, message: string): ServerMsg => ({ type: 'error', code, message });
const passKey = (code: string) => `sharu.p2p.pass.${code.toLowerCase()}`;
const roomConfig = (pass: string) => (pass ? { appId: APP_ID, password: pass } : { appId: APP_ID });

export class P2PHost implements Signal {
  private room: Room | null = null;
  private h2c: Send | null = null;
  private hello: Send | null = null;
  private code = ''; private token = ''; private pass = ''; private proof = '';
  private hasAudio = false;
  private next = 1;
  private viewers = new Map<number, { tid: string; nickname: string }>();
  private byTid = new Map<string, number>();
  private banned = new Set<string>();

  constructor(private ev: SignalEvents) {}

  connect(): void { setTimeout(() => this.ev.open(), 0); }
  close(): void { this.room?.leave(); this.room = null; }
  send(m: ClientMsg): void { void this.handle(m); }

  private emit(m: ServerMsg) { this.ev.message(m); }
  private tell(tid: string, m: ServerMsg) { this.h2c?.(m, tid); }
  private broadcast(m: ServerMsg) { for (const v of this.viewers.values()) this.tell(v.tid, m); }
  private announceCount() {
    const m: ServerMsg = { type: 'viewer_count', count: this.viewers.size };
    this.emit(m); this.broadcast(m);
  }
  private taken(nick: string) { return [...this.viewers.values()].some((v) => v.nickname.toLowerCase() === nick.toLowerCase()); }

  private async handle(m: ClientMsg): Promise<void> {
    switch (m.type) {
      case 'create':
        this.code = generateCode(); this.token = randomHex(16); this.pass = (m.passphrase ?? '').trim(); this.hasAudio = m.has_audio;
        this.proof = await sha256(this.token);
        try { localStorage.setItem(passKey(this.code), this.pass); } catch { /* ignore */ }
        this.start();
        this.emit({ type: 'created', code: this.code, host_token: this.token, ice_servers: ICE, max_viewers: MAX_VIEWERS });
        break;
      case 'resume':
        this.code = m.code; this.token = m.host_token; this.proof = await sha256(this.token);
        try { this.pass = localStorage.getItem(passKey(m.code)) ?? ''; } catch { /* ignore */ }
        this.start();
        this.emit({ type: 'resumed', code: m.code, has_audio: this.hasAudio, viewers: [], ice_servers: ICE }); // viewers re-announce themselves
        break;
      case 'signal': {
        const v = this.viewers.get(m.to);
        if (v) this.tell(v.tid, { type: 'signal', from: 0, data: m.data });
        break;
      }
      case 'set_audio':
        this.hasAudio = m.has_audio;
        this.broadcast({ type: 'audio_changed', has_audio: m.has_audio });
        break;
      case 'kick': {
        const v = this.viewers.get(m.peer);
        if (v) { this.banned.add(v.tid); this.tell(v.tid, { type: 'kicked' }); this.drop(v.tid); }
        break;
      }
      case 'leaving': this.broadcast({ type: 'host_state', status: 'left', grace_secs: LEFT_GRACE_S }); break;
      case 'end':
        this.broadcast({ type: 'host_state', status: 'ended' });
        setTimeout(() => this.close(), 400);
        break;
      default: break;
    }
  }

  private start(): void {
    this.room?.leave();
    const room = joinRoom(roomConfig(this.pass), normalizeCode(this.code) ?? this.code.toLowerCase());
    this.room = room;
    const c2h = action(room, 'c2h');
    this.h2c = action(room, 'h2c').send;
    this.hello = action(room, 'hello').send;
    room.onPeerJoin = (tid: string) => { this.hello?.({ proof: this.proof }, tid); };
    room.onPeerLeave = (tid: string) => this.drop(tid);
    c2h.on((d, tid) => {
      const m = d as ClientMsg | null;
      if (!m || typeof m !== 'object' || this.banned.has(tid)) return;
      if (m.type === 'join') this.onJoin(tid, m);
      else if (m.type === 'signal' && m.to === 0) {
        const id = this.byTid.get(tid);
        if (id !== undefined) this.emit({ type: 'signal', from: id, data: m.data });
      }
    });
  }

  private onJoin(tid: string, m: Extract<ClientMsg, { type: 'join' }>): void {
    const reply = (peer: number, nickname: string) => this.tell(tid, {
      type: 'joined', peer, nickname, host: 'live', has_audio: this.hasAudio, viewer_count: this.viewers.size, ice_servers: ICE,
    });
    const existing = this.byTid.get(tid);
    if (existing !== undefined) { reply(existing, this.viewers.get(existing)!.nickname); return; }
    if (this.viewers.size >= MAX_VIEWERS) { this.tell(tid, err('session_full', 'This session is full.')); return; }
    let nickname = '';
    const wanted = (m.nickname ?? '').trim();
    if (wanted) {
      const s = sanitizeNickname(wanted);
      if (!s) { this.tell(tid, err('nickname_invalid', 'Invalid nickname.')); return; }
      if (this.taken(s)) { this.tell(tid, err('nickname_taken', 'Nickname taken.')); return; }
      nickname = s;
    } else {
      for (let i = 0; i < 32 && !nickname; i++) { const c = defaultNickname(); if (!this.taken(c)) nickname = c; }
      if (!nickname) return;
    }
    const peer = this.next++;
    this.viewers.set(peer, { tid, nickname });
    this.byTid.set(tid, peer);
    reply(peer, nickname);
    this.emit({ type: 'viewer_joined', peer, nickname });
    this.announceCount();
  }

  private drop(tid: string): void {
    const id = this.byTid.get(tid);
    if (id === undefined) return;
    this.byTid.delete(tid);
    this.viewers.delete(id);
    this.emit({ type: 'viewer_left', peer: id });
    this.announceCount();
  }
}

export class P2PViewer implements Signal {
  private room: Room | null = null;
  private c2h: Send | null = null;
  private hostTid = ''; private proof = '';
  private joinMsg: Extract<ClientMsg, { type: 'join' }> | null = null;
  private joined = false; private nickname = ''; private noticed = false;
  private joinTimer = 0; private goneTimer = 0;

  constructor(private ev: SignalEvents) {}

  connect(): void { setTimeout(() => this.ev.open(), 0); }
  close(): void {
    clearTimeout(this.joinTimer); clearTimeout(this.goneTimer);
    this.room?.leave(); this.room = null;
  }

  send(m: ClientMsg): void {
    if (m.type === 'join') { this.joinMsg = m; this.start(m); }
    else if (m.type === 'signal' && this.hostTid) this.c2h?.({ type: 'signal', to: 0, data: m.data }, this.hostTid);
  }

  private start(m: Extract<ClientMsg, { type: 'join' }>): void {
    const key = normalizeCode(m.code);
    if (!key) { this.ev.message(err('not_found', 'Invalid code.')); return; }
    this.room?.leave();
    const room = joinRoom(roomConfig((m.passphrase ?? '').trim()), key);
    this.room = room;
    this.c2h = action(room, 'c2h').send;
    action(room, 'hello').on((d, tid) => this.onHello(d, tid));
    action(room, 'h2c').on((d, tid) => this.onHost(d, tid));
    room.onPeerLeave = (tid: string) => this.onLeave(tid);
    // Nobody answers if the code is wrong, the host is offline, or the passphrase does not match.
    this.joinTimer = window.setTimeout(() => {
      if (this.joined) return;
      this.ev.message(err('not_found', 'No session found for that code and passphrase.'));
      this.close();
    }, JOIN_TIMEOUT_MS);
  }

  private sendJoin(): void {
    if (!this.joinMsg || !this.hostTid) return;
    const nickname = this.nickname || this.joinMsg.nickname;
    this.c2h?.({ type: 'join', code: this.joinMsg.code, ...(nickname ? { nickname } : {}) }, this.hostTid);
  }

  private onHello(d: unknown, tid: string): void {
    const proof = (d as { proof?: unknown } | null)?.proof;
    if (typeof proof !== 'string') return;
    if (this.hostTid && this.hostTid !== tid) return; // already following a host
    if (this.proof && this.proof !== proof) return;   // not the host we met first (weak guard, see README)
    const returned = !this.hostTid && this.joined;
    this.proof = proof; this.hostTid = tid;
    clearTimeout(this.goneTimer);
    if (returned) { this.noticed = false; this.ev.message({ type: 'host_state', status: 'live' }); }
    this.sendJoin();
  }

  private onHost(d: unknown, tid: string): void {
    if (tid !== this.hostTid) return;
    const m = d as ServerMsg | null;
    if (!m || typeof m !== 'object') return;
    if (m.type === 'joined') { this.joined = true; this.nickname = m.nickname; clearTimeout(this.joinTimer); }
    if (m.type === 'host_state' && m.status === 'left') { this.noticed = true; this.scheduleEnd(LEFT_GRACE_S); }
    this.ev.message(m);
    if (m.type === 'host_state' && m.status === 'ended') this.close();
  }

  private onLeave(tid: string): void {
    if (tid !== this.hostTid) return;
    this.hostTid = '';
    if (!this.joined) return;
    if (!this.noticed) this.ev.message({ type: 'host_state', status: 'lost', grace_secs: LOST_GRACE_S });
    this.scheduleEnd(this.noticed ? LEFT_GRACE_S : LOST_GRACE_S);
  }

  private scheduleEnd(seconds: number): void {
    clearTimeout(this.goneTimer);
    this.goneTimer = window.setTimeout(() => {
      this.ev.message({ type: 'host_state', status: 'ended' });
      this.close();
    }, seconds * 1000);
  }
}
