export const canShare = (): boolean => !!navigator.mediaDevices?.getDisplayMedia;
export const isFirefox = (): boolean => /Firefox\//.test(navigator.userAgent);

export interface Captured { stream: MediaStream; hasAudio: boolean }

/** Audio processing is explicitly disabled so films keep their SFX and voices intact. */
export async function captureDisplay(): Promise<Captured> {
  const stream = await navigator.mediaDevices.getDisplayMedia({
    video: { frameRate: { ideal: 30 } },
    audio: { echoCancellation: false, noiseSuppression: false, autoGainControl: false },
  });
  const [video] = stream.getVideoTracks();
  if (video) (video as MediaStreamTrack & { contentHint?: string }).contentHint = 'motion';
  return { stream, hasAudio: stream.getAudioTracks().length > 0 };
}
