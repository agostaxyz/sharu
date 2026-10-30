import { describe, expect, it } from 'vitest';
import { tuneOpus } from './mesh';

const SDP = 'v=0\r\nm=audio 9 UDP/TLS/RTP/SAVPF 111\r\na=rtpmap:111 opus/48000/2\r\na=fmtp:111 minptime=10;useinbandfec=0\r\na=rtpmap:0 PCMU/8000\r\n';

describe('tuneOpus', () => {
  it('asks for stereo, high bitrate, FEC on, DTX off', () => {
    const out = tuneOpus(SDP);
    expect(out).toContain('stereo=1');
    expect(out).toContain('maxaveragebitrate=256000');
    expect(out).toContain('useinbandfec=1');
    expect(out).toContain('usedtx=0');
    expect(out).toContain('minptime=10');
    expect(out).not.toContain('useinbandfec=0');
  });
  it('leaves SDP without Opus untouched', () => {
    const plain = 'v=0\r\nm=video 9 UDP/TLS/RTP/SAVPF 96\r\n';
    expect(tuneOpus(plain)).toBe(plain);
  });
  it('handles undefined', () => expect(tuneOpus(undefined)).toBe(''));
});
