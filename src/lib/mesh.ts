// SPDX-License-Identifier: AGPL-3.0-or-later
// WebRTC star mesh: the host sends one connection per viewer; viewers only receive.
// No audio/video processing: capture has AEC/NS/AGC off and Opus is tuned for stereo at high bitrate.

export type SendSignal = (to: number, data: unknown) => void;

const MAX_VIDEO_BPS = 8_000_000; // per viewer; the mesh multiplies this by the viewer count
const PLAYOUT_MS = 30; // small jitter buffer: low latency without audible glitches

interface SdpMsg { sdp: { type: 'offer' | 'answer'; sdp: string } }
interface CandMsg { candidate: RTCIceCandidateInit }
const isObj = (d: unknown): d is Record<string, unknown> => typeof d === 'object' && d !== null;
const asSdp = (d: unknown): SdpMsg | null => {
  if (!isObj(d) || !isObj(d.sdp)) return null;
  const { type, sdp } = d.sdp;
  return (type === 'offer' || type === 'answer') && typeof sdp === 'string' ? (d as unknown as SdpMsg) : null;
};
const asCand = (d: unknown): CandMsg | null =>
  isObj(d) && isObj(d.candidate) && typeof d.candidate.candidate === 'string' ? (d as unknown as CandMsg) : null;

/** Asks for stereo, high-bitrate, FEC-protected, non-DTX Opus. Pure string function (unit-tested). */
export function tuneOpus(sdp: string | undefined): string {
  if (!sdp) return '';
  const m = sdp.match(/a=rtpmap:(\d+) opus\/48000\/2/i);
  if (!m) return sdp;
  const pt = m[1]!;
  return sdp.replace(new RegExp(`a=fmtp:${pt} ([^\\r\\n]*)`), (_line, params: string) => {
    const kv = new Map<string, string>();
    for (const part of params.split(';').map((s) => s.trim()).filter(Boolean)) {
      const i = part.indexOf('=');
      kv.set(i < 0 ? part : part.slice(0, i), i < 0 ? '' : part.slice(i + 1));
    }
    kv.set('stereo', '1'); kv.set('sprop-stereo', '1'); kv.set('maxaveragebitrate', '256000');
    kv.set('useinbandfec', '1'); kv.set('usedtx', '0');
    return `a=fmtp:${pt} ${[...kv].map(([k, v]) => (v === '' ? k : `${k}=${v}`)).join(';')}`;
  });
}

interface Peer {
  pc: RTCPeerConnection;
  video: RTCRtpTransceiver;
  audio: RTCRtpTransceiver;
  queue: Promise<void>;
  pending: RTCIceCandidateInit[];
  remoteSet: boolean;
}

export class HostMesh {
  private peers = new Map<number, Peer>();
  private stream: MediaStream | null = null;

  constructor(private ice: RTCIceServer[], private send: SendSignal) {}

  /** Swap the shared source (or null to stop). No renegotiation: tracks are replaced in place. */
  setStream(stream: MediaStream | null): void {
    this.stream = stream;
    for (const p of this.peers.values()) this.attach(p);
  }

  addViewer(id: number): void {
    this.removeViewer(id);
    const pc = new RTCPeerConnection({ iceServers: this.ice });
    // Both m-lines exist from the start, so audio can appear later via replaceTrack.
    const video = pc.addTransceiver('video', { direction: 'sendonly' });
    const audio = pc.addTransceiver('audio', { direction: 'sendonly' });
    const peer: Peer = { pc, video, audio, queue: Promise.resolve(), pending: [], remoteSet: false };
    this.peers.set(id, peer);
    pc.onicecandidate = (e) => { if (e.candidate) this.send(id, { candidate: e.candidate.toJSON() }); };
    pc.onnegotiationneeded = async () => {
      try {
        const offer = await pc.createOffer();
        await pc.setLocalDescription({ type: 'offer', sdp: tuneOpus(offer.sdp) });
        this.send(id, { sdp: { type: 'offer', sdp: pc.localDescription?.sdp ?? '' } });
      } catch { /* peer closed meanwhile */ }
    };
    pc.onconnectionstatechange = () => { if (pc.connectionState === 'failed') pc.restartIce(); };
    this.attach(peer);
  }

  removeViewer(id: number): void {
    this.peers.get(id)?.pc.close();
    this.peers.delete(id);
  }

  handleSignal(from: number, data: unknown): void {
    const p = this.peers.get(from);
    if (!p) return;
    p.queue = p.queue.then(() => this.process(p, data)).catch(() => {});
  }

  closeAll(): void {
    for (const id of [...this.peers.keys()]) this.removeViewer(id);
  }

  private attach(p: Peer): void {
    const v = this.stream?.getVideoTracks()[0] ?? null;
    const a = this.stream?.getAudioTracks()[0] ?? null;
    if (v) (v as MediaStreamTrack & { contentHint?: string }).contentHint = 'motion';
    void p.video.sender.replaceTrack(v).catch(() => {});
    void p.audio.sender.replaceTrack(a).catch(() => {});
  }

  private async process(p: Peer, data: unknown): Promise<void> {
    const sdp = asSdp(data);
    if (sdp?.sdp.type === 'answer') {
      await p.pc.setRemoteDescription(sdp.sdp);
      p.remoteSet = true;
      for (const c of p.pending.splice(0)) await p.pc.addIceCandidate(c).catch(() => {});
      await this.tuneVideo(p);
      return;
    }
    const cand = asCand(data);
    if (!cand) return;
    if (p.remoteSet) await p.pc.addIceCandidate(cand.candidate).catch(() => {});
    else p.pending.push(cand.candidate);
  }

  private async tuneVideo(p: Peer): Promise<void> {
    try {
      const params = p.video.sender.getParameters() as RTCRtpSendParameters & { degradationPreference?: string };
      if (!params.encodings?.length) params.encodings = [{}];
      params.encodings[0]!.maxBitrate = MAX_VIDEO_BPS;
      params.degradationPreference = 'maintain-framerate';
      await p.video.sender.setParameters(params);
    } catch { /* not supported everywhere */ }
  }
}

export class ViewerMesh {
  private pc: RTCPeerConnection | null = null;
  private queue: Promise<void> = Promise.resolve();
  private pending: RTCIceCandidateInit[] = [];
  private remoteSet = false;

  constructor(
    private ice: RTCIceServer[],
    private send: (data: unknown) => void,
    private onStream: (s: MediaStream) => void,
    private onVideoActive: (active: boolean) => void,
  ) {}

  /** (Re)creates the peer connection; call on join and whenever the host comes back. */
  reset(): void {
    this.pc?.close();
    this.pending = []; this.remoteSet = false;
    const remote = new MediaStream();
    const pc = new RTCPeerConnection({ iceServers: this.ice });
    this.pc = pc;
    pc.ontrack = (e) => {
      remote.addTrack(e.track);
      (e.receiver as RTCRtpReceiver & { jitterBufferTarget?: number | null }).jitterBufferTarget = PLAYOUT_MS;
      if (e.track.kind === 'video') {
        this.onVideoActive(!e.track.muted);
        e.track.onunmute = () => this.onVideoActive(true);
        e.track.onmute = () => this.onVideoActive(false);
      }
    };
    pc.onicecandidate = (e) => { if (e.candidate) this.send({ candidate: e.candidate.toJSON() }); };
    this.onStream(remote);
    this.onVideoActive(false);
  }

  handleSignal(data: unknown): void {
    this.queue = this.queue.then(() => this.process(data)).catch(() => {});
  }

  close(): void {
    this.pc?.close();
    this.pc = null;
  }

  private async process(data: unknown): Promise<void> {
    const pc = this.pc;
    if (!pc) return;
    const sdp = asSdp(data);
    if (sdp?.sdp.type === 'offer') {
      await pc.setRemoteDescription(sdp.sdp);
      this.remoteSet = true;
      for (const c of this.pending.splice(0)) await pc.addIceCandidate(c).catch(() => {});
      const answer = await pc.createAnswer();
      await pc.setLocalDescription({ type: 'answer', sdp: tuneOpus(answer.sdp) });
      this.send({ sdp: { type: 'answer', sdp: pc.localDescription?.sdp ?? '' } });
      return;
    }
    const cand = asCand(data);
    if (!cand) return;
    if (this.remoteSet) await pc.addIceCandidate(cand.candidate).catch(() => {});
    else this.pending.push(cand.candidate);
  }
}
