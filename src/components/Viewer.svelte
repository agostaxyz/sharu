<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { createSignal, type Signal } from '../lib/signal';
  import AudioBars from './AudioBars.svelte';
  import { extractCode, normalizeCode } from '../lib/code';
  import { stripBase, withBase } from '../lib/base';
  import { getNickname, setNickname } from '../lib/settings';
  import { ViewerMesh } from '../lib/mesh';
  import type { HostStatus, HostMsg } from '../lib/proto';

  type Phase = 'invalid' | 'form' | 'connecting' | 'watching' | 'closed';
  let phase = $state<Phase>('form');
  let code = $state('');
  let nickname = $state('');
  let passphrase = $state('');
  let problem = $state('');
  let closedReason = $state('');
  let hostStatus = $state<HostStatus>('live');
  let hasAudio = $state(false);
  let hostMuted = $state(false);
  let count = $state(0);
  let shownCount = $state(0);
  let volume = $state(100);
  let fullscreen = $state(false);
  let barVisible = $state(true);
  let hideTimer = 0;
  let supportsVolume = $state(true);
  let video: HTMLVideoElement | undefined = $state();
  let stage: HTMLElement | undefined = $state();
  let client: Signal | null = null;
  let mesh: ViewerMesh | null = null;
  let stream = $state<MediaStream | null>(null);
  let videoActive = $state(false);

  const hostText = $derived(
    hostStatus === 'live' ? 'Live'
    : hostStatus === 'left' ? 'The host closed their tab. Waiting a few seconds in case they come back…'
    : hostStatus === 'lost' ? 'The host lost their connection. Waiting for them to reconnect…'
    : 'The host ended the session.'
  );

  onMount(() => {
    code = extractCode(stripBase(location.pathname));
    if (!normalizeCode(code)) { phase = 'invalid'; return; }
    nickname = getNickname();
    const probe = document.createElement('video');
    probe.volume = 0.5;
    supportsVolume = probe.volume === 0.5; // iOS Safari ignores video.volume
    const onFs = () => (fullscreen = !!document.fullscreenElement);
    document.addEventListener('fullscreenchange', onFs);
    return () => document.removeEventListener('fullscreenchange', onFs);
  });
  onDestroy(() => { mesh?.close(); client?.close(); });

  $effect(() => { if (video) video.volume = volume / 100; });
  $effect(() => { if (video) { video.srcObject = stream; if (stream) video.play().catch(() => {}); } });
  // Announce the watcher count politely: debounce so screen readers are not flooded.
  $effect(() => { const c = count; const t = setTimeout(() => (shownCount = c), 1500); return () => clearTimeout(t); });

  async function join(e: Event) {
    e.preventDefault(); // this click is also the user gesture that allows audio playback
    problem = ''; phase = 'connecting';
    client?.close();
    client = await createSignal('viewer', {
      open: () => client?.send({ type: 'join', code, nickname: nickname.trim() || undefined, passphrase: passphrase.trim() || undefined }),
      message: onMessage,
    });
    client.connect();
  }

  function onMessage(m: HostMsg) {
    switch (m.type) {
      case 'joined':
        phase = 'watching'; problem = ''; nickname = m.nickname; setNickname(m.nickname);
        hostStatus = m.host; hasAudio = m.has_audio; hostMuted = m.muted ?? false; count = m.viewer_count;
        mesh?.close();
        mesh = new ViewerMesh(m.ice_servers, (data) => client?.send({ type: 'signal', to: 0, data }), (s) => (stream = s), (a) => (videoActive = a));
        mesh.reset();
        break;
      case 'signal': mesh?.handleSignal(m.data); break;
      case 'viewer_count': count = m.count; break;
      case 'host_state':
        if (m.status === 'live' && hostStatus !== 'live') mesh?.reset(); // host came back: expect a fresh offer
        hostStatus = m.status;
        if (m.status === 'ended') { mesh?.close(); client?.close(); phase = 'closed'; closedReason = 'The host ended the session.'; }
        break;
      case 'audio_changed': hasAudio = m.has_audio; hostMuted = m.muted ?? false; break;
      case 'kicked': mesh?.close(); client?.close(); phase = 'closed'; closedReason = 'The host removed you from this session.'; break;
      case 'error': {
        const back = () => { phase = 'form'; client?.close(); };
        if (m.code === 'wrong_passphrase') { problem = 'That passphrase is not right.'; back(); }
        else if (m.code === 'not_found') { phase = 'closed'; closedReason = 'No session was found for that code. If the host set a passphrase, check it too.'; }
        else if (m.code === 'session_full') { phase = 'closed'; closedReason = 'This session is full.'; }
        else if (m.code === 'nickname_taken') { problem = 'That nickname is taken. Pick another or leave it empty.'; back(); }
        else if (m.code === 'nickname_invalid') { problem = 'Nicknames use 2 to 24 letters or digits, no spaces.'; back(); }
        else if (m.code === 'rate_limited') { problem = 'Too many attempts. Wait a minute and try again.'; back(); }
        else { problem = 'Something went wrong. Please try again.'; back(); }
        break;
      }
    }
  }

  // In fullscreen the controls fade out after a moment, like most video players.
  function nudgeBar() {
    barVisible = true;
    clearTimeout(hideTimer);
    hideTimer = window.setTimeout(() => (barVisible = false), 2500);
  }

  async function toggleFullscreen() {
    if (document.fullscreenElement) { await document.exitFullscreen(); return; }
    if (stage?.requestFullscreen) await stage.requestFullscreen();
    else (video as (HTMLVideoElement & { webkitEnterFullscreen?: () => void }) | undefined)?.webkitEnterFullscreen?.();
  }
</script>

{#if phase === 'invalid'}
  <h1>Page not found</h1>
  <p>This address is not a valid session code.</p>
  <a class="btn btn-primary" href={withBase('/')}>Back to start</a>
{:else if phase === 'form' || phase === 'connecting'}
  <h1>Join session</h1>
  <p class="code" aria-label="Session code">{code}</p>
  <form class="card" onsubmit={join}>
    <label for="nick">Nickname (optional)</label>
    <input id="nick" type="text" autocomplete="nickname" spellcheck="false" bind:value={nickname} aria-describedby="nick-help" />
    <p id="nick-help" class="muted">Leave empty for a random one. Only the host sees nicknames.</p>
    <p class="error" role="alert">{problem}</p>
    <button class="btn btn-primary" type="submit" disabled={phase === 'connecting'}>{phase === 'connecting' ? 'Connecting…' : 'Join Session'}</button>
  </form>
{:else if phase === 'watching'}
  <h1>Watch</h1>
  <p role="status" aria-live="polite">{problem || (hostStatus === 'live' ? '🔴 Live' : hostText)}</p>
  {#if hostStatus === 'live' && !videoActive}<p role="status" class="muted">Waiting for the host to share a screen…</p>{/if}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="watch-wrap" bind:this={stage} ondblclick={toggleFullscreen} onpointermove={nudgeBar}>
    <div class={videoActive ? 'stage' : 'stage aspect-video'}>
      <video bind:this={video} autoplay playsinline aria-label="Shared screen"></video>
    </div>
    {#if fullscreen}
      <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
      <div class={`absolute inset-x-0 bottom-0 z-20 flex items-center gap-6 bg-linear-to-t from-black/80 to-transparent px-6 py-4 transition-opacity ${barVisible ? 'opacity-100' : 'pointer-events-none opacity-0'}`} role="group" aria-label="Playback controls" ondblclick={(e) => e.stopPropagation()}>
        <button class="btn btn-primary btn-sm" type="button" onclick={toggleFullscreen}>Exit fullscreen</button>
        {#if supportsVolume && hasAudio && !hostMuted}
          <label class="flex items-center gap-3 text-sm font-bold text-white">Volume
            <input type="range" min="0" max="100" step="1" bind:value={volume} aria-valuetext={`${volume} percent`} />
          </label>
        {/if}
      </div>
    {/if}
  </div>
  <div class="mt-4 flex items-center justify-between gap-4">
    <div class="pill-bar">
      <span class="text-lg" role="status" aria-live="polite" aria-label={`${shownCount} watching`}>👤 {shownCount}</span>
      {#if !hasAudio}
        <span class="pill" role="status">No audio in the host's source</span>
      {:else if hostMuted}
        <span class="pill" role="status">The host muted the sound</span>
      {:else}
        {#if supportsVolume}
          <label class="grid justify-items-center gap-0.5 text-sm">Volume
            <input id="vol" type="range" class="w-40" min="0" max="100" step="1" bind:value={volume} aria-valuetext={`${volume} percent`} />
          </label>
        {/if}
        <AudioBars {stream} level={volume / 100} tone="dark" />
      {/if}
    </div>
    <button class="btn shrink-0" type="button" aria-pressed={fullscreen} onclick={toggleFullscreen}>{fullscreen ? 'Exit fullscreen' : 'Fullscreen'}</button>
  </div>
{:else}
  <h1>Session closed</h1>
  <p role="status">{closedReason}</p>
  <a class="btn btn-primary" href={withBase('/')}>Back to start</a>
{/if}
