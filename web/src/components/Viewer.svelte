<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { SignalClient, signalUrl } from '../lib/signal';
  import { extractCode, normalizeCode } from '../lib/code';
  import { stripBase, withBase } from '../lib/base';
  import { getNickname, setNickname } from '../lib/settings';
  import type { HostStatus, ServerMsg } from '../lib/proto';

  type Phase = 'invalid' | 'form' | 'connecting' | 'watching' | 'closed';
  let phase = $state<Phase>('form');
  let code = $state('');
  let nickname = $state('');
  let passphrase = $state('');
  let askPassphrase = $state(false);
  let problem = $state('');
  let closedReason = $state('');
  let hostStatus = $state<HostStatus>('live');
  let hasAudio = $state(false);
  let count = $state(0);
  let shownCount = $state(0);
  let volume = $state(100);
  let fullscreen = $state(false);
  let supportsVolume = $state(true);
  let video: HTMLVideoElement | undefined = $state();
  let stage: HTMLElement | undefined = $state();
  let client: SignalClient | null = null;

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
  onDestroy(() => client?.close());

  $effect(() => { if (video) video.volume = volume / 100; });
  // Announce the watcher count politely: debounce so screen readers are not flooded.
  $effect(() => { const c = count; const t = setTimeout(() => (shownCount = c), 1500); return () => clearTimeout(t); });

  function join(e: Event) {
    e.preventDefault(); // this click is also the user gesture that allows audio playback
    const url = signalUrl();
    if (!url) { problem = 'This site has no signalling server configured yet.'; return; }
    problem = ''; phase = 'connecting';
    client?.close();
    client = new SignalClient(url, {
      open: () => client?.send({ type: 'join', code, nickname: nickname.trim() || undefined, passphrase: passphrase.trim() || undefined }),
      message: onMessage,
      close: ({ willRetry }) => { if (phase === 'watching' && willRetry) problem = 'Connection lost. Reconnecting…'; },
    }, false);
    client.connect();
  }

  function onMessage(m: ServerMsg) {
    switch (m.type) {
      case 'joined':
        phase = 'watching'; problem = ''; nickname = m.nickname; setNickname(m.nickname);
        hostStatus = m.host; hasAudio = m.has_audio; count = m.viewer_count; break;
      case 'viewer_count': count = m.count; break;
      case 'host_state':
        hostStatus = m.status;
        if (m.status === 'ended') { phase = 'closed'; closedReason = 'The host ended the session.'; }
        break;
      case 'audio_changed': hasAudio = m.has_audio; break;
      case 'kicked': phase = 'closed'; closedReason = 'The host removed you from this session.'; break;
      case 'error': {
        const back = () => { phase = 'form'; client?.close(); };
        if (m.code === 'wrong_passphrase') { askPassphrase = true; problem = 'This session needs the right passphrase.'; back(); }
        else if (m.code === 'not_found') { phase = 'closed'; closedReason = 'No session with that code exists (anymore).'; }
        else if (m.code === 'session_full') { phase = 'closed'; closedReason = 'This session is full.'; }
        else if (m.code === 'nickname_taken') { problem = 'That nickname is taken. Pick another or leave it empty.'; back(); }
        else if (m.code === 'nickname_invalid') { problem = 'Nicknames use 2 to 24 letters or digits, no spaces.'; back(); }
        else if (m.code === 'rate_limited') { problem = 'Too many attempts. Wait a minute and try again.'; back(); }
        else { problem = 'Something went wrong. Please try again.'; back(); }
        break;
      }
    }
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
    {#if askPassphrase}
      <label for="pass">Passphrase</label>
      <input id="pass" type="password" autocomplete="off" bind:value={passphrase} />
    {/if}
    <p class="error" role="alert">{problem}</p>
    <button class="btn btn-primary" type="submit" disabled={phase === 'connecting'}>{phase === 'connecting' ? 'Connecting…' : 'Join'}</button>
  </form>
{:else if phase === 'watching'}
  <h1>Watching</h1>
  <p role="status" aria-live="polite">{problem || hostText}</p>
  <div class="stage" bind:this={stage}>
    <!-- WebRTC track attaches here in the next milestone (mesh). -->
    <video bind:this={video} autoplay playsinline aria-label="Shared screen"></video>
  </div>
  <div class="row">
    <p role="status" aria-live="polite" class="muted"><output>{shownCount}</output> {shownCount === 1 ? 'person' : 'people'} watching</p>
    {#if supportsVolume}
      <label class="row" for="vol">Volume
        <input id="vol" type="range" min="0" max="100" step="1" bind:value={volume} disabled={!hasAudio} aria-valuetext={`${volume} percent`} />
      </label>
    {/if}
    <button class="btn" type="button" aria-pressed={fullscreen} onclick={toggleFullscreen}>{fullscreen ? 'Exit fullscreen' : 'Fullscreen'}</button>
  </div>
  {#if !hasAudio}<p class="notice">The host is sharing without audio.</p>{/if}
{:else}
  <h1>Session closed</h1>
  <p role="status">{closedReason}</p>
  <a class="btn btn-primary" href={withBase('/')}>Back to start</a>
{/if}
