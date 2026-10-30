// Messages exchanged between the host and its viewers (carried over the peer-to-peer channel).
type PeerId = number;
export type HostStatus = 'live' | 'left' | 'lost' | 'ended';
export type ErrorCode = 'bad_message' | 'not_found' | 'wrong_passphrase' | 'session_full' | 'nickname_taken'
  | 'nickname_invalid' | 'forbidden' | 'rate_limited' | 'already_joined' | 'internal';
export interface IceServer { urls: string[]; username?: string; credential?: string }
export interface ViewerInfo { peer: PeerId; nickname: string }

export type ClientMsg =
  | { type: 'create'; passphrase?: string; has_audio: boolean }
  | { type: 'resume'; code: string; host_token: string }
  | { type: 'join'; code: string; nickname?: string; passphrase?: string }
  | { type: 'signal'; to: PeerId; data: unknown }
  | { type: 'set_audio'; has_audio: boolean }
  | { type: 'kick'; peer: PeerId }
  | { type: 'leaving' } | { type: 'end' } | { type: 'ping' };

export type ServerMsg =
  | { type: 'created'; code: string; host_token: string; ice_servers: IceServer[]; max_viewers: number }
  | { type: 'resumed'; code: string; has_audio: boolean; viewers: ViewerInfo[]; ice_servers: IceServer[] }
  | { type: 'joined'; peer: PeerId; nickname: string; host: HostStatus; has_audio: boolean; viewer_count: number; ice_servers: IceServer[] }
  | { type: 'viewer_joined'; peer: PeerId; nickname: string }
  | { type: 'viewer_left'; peer: PeerId }
  | { type: 'viewer_count'; count: number }
  | { type: 'signal'; from: PeerId; data: unknown }
  | { type: 'host_state'; status: HostStatus; grace_secs?: number }
  | { type: 'audio_changed'; has_audio: boolean }
  | { type: 'kicked' } | { type: 'pong' }
  | { type: 'error'; code: ErrorCode; message: string };
