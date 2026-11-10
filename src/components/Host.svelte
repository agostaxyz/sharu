<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { createSignal, type Signal } from '../lib/signal';
  import { captureDisplay, canShare, isFirefox, type Captured } from '../lib/capture';
  import { loadHostSession, saveHostSession, clearHostSession, type HostSession } from '../lib/settings';
  import { withBase } from '../lib/base';
  import { HostMesh } from '../lib/mesh';
  import AudioBars from './AudioBars.svelte';
  import type { IceServer, HostMsg, ViewerInfo } from '../lib/proto';

  // embedded = shown inside the "Share your screen" card on the home page; otherwise it is the full /<code> page.
  let { embedded = false }: { embedded?: boolean } = $props();

  type Phase = 'setup' | 'connecting' | 'live';
  let phase = $state<Phase>('setup');
  let passphrase = $state('');
  let activePass = $state('');
  let code = $state('');
  let viewers = $state<ViewerInfo[]>([]);
  let problem = $state('');
  let status = $state('');
  let saved = $state<HostSession | null>(null);
  let captured = $state<Captured | null>(null);
  let muted = $state(false);
  let paused = $state(false);
  let pauseStream = $state<MediaStream | null>(null);
  let pauseTimer = 0;
  let preview: HTMLVideoElement | undefined = $state();
  let dialog: HTMLDialogElement | undefined = $state();
  let client: Signal | null = null;
  let mesh: HostMesh | null = null;
  let mode: 'create' | 'resume' = 'create';
  const firefox = isFirefox();
  const supported = canShare();
  const link = $derived(code ? `${location.origin}${withBase(`/${code}`)}` : '');

  // The address bar shows the livestream link while hosting. The tab flag (sessionStorage) survives reloads of THIS tab only,
  // so opening the same link in another tab, even in this browser, gives the viewer experience.
  function showSessionUrl(c: string) {
    try { sessionStorage.setItem('sharu.hostTab', c.toLowerCase()); } catch { /* ignore */ }
    history.replaceState(null, '', withBase(`/${c}`));
    document.documentElement.dataset.hosting = '1';
    const h = document.getElementById('share-h');
    if (h) h.textContent = '🔴 Sharing your screen';
    if (!embedded) document.title = 'Sharing · sharu';
  }
  function leaveSessionView() {
    try { sessionStorage.removeItem('sharu.hostTab'); } catch { /* ignore */ }
    delete document.documentElement.dataset.hosting;
    const h = document.getElementById('share-h');
    if (h) h.textContent = 'Share your screen';
    if (embedded) history.replaceState(null, '', withBase('/'));
    else location.assign(withBase('/'));
  }

  const pagehide = () => client?.send({ type: 'leaving' });
  onMount(() => {
    window.addEventListener('pagehide', pagehide);
    const s = loadHostSession();
    if (s && embedded) saved = s;                 // home page: offer to resume instead of doing it silently
    else if (s) { mode = 'resume'; void connect(); } // /<code> page: this tab is the host, resume right away
  });
  onDestroy(() => { window.removeEventListener('pagehide', pagehide); mesh?.closeAll(); client?.close(); });
  $effect(() => { if (preview) preview.srcObject = (paused && pauseStream) || (captured?.stream ?? null); });
  // Muting disables the audio track: viewers receive silence, and no renegotiation is needed.
  $effect(() => {
    const off = muted || paused; // pausing blacks out the video and silences the audio, no renegotiation needed
    captured?.stream.getAudioTracks().forEach((t) => { t.enabled = !off; });
    captured?.stream.getVideoTracks().forEach((t) => { t.enabled = !paused; });
  });
  // Warn before leaving while actively sharing. If the source simply stops, closing is fine.
  $effect(() => {
    if (!captured) return;
    const warn = (e: BeforeUnloadEvent) => { e.preventDefault(); e.returnValue = ''; };
    window.addEventListener('beforeunload', warn);
    return () => window.removeEventListener('beforeunload', warn);
  });

  async function connect() {
    if (mode === 'create') activePass = '';
    phase = 'connecting'; problem = '';
    client?.close();
    client = await createSignal('host', {
      open: () => {
        const s = loadHostSession();
        if (mode === 'resume' && s) client?.send({ type: 'resume', code: s.code, host_token: s.token });
        else client?.send({ type: 'create', has_audio: captured?.hasAudio ?? false, passphrase: activePass });
      },
      message: onMessage,
    });
    client.connect();
  }

  function startMesh(ice: IceServer[], existing: ViewerInfo[]) {
    mesh?.closeAll();
    mesh = new HostMesh(ice, (to, data) => client?.send({ type: 'signal', to, data }));
    if (captured) mesh.setStream(captured.stream);
    for (const v of existing) mesh.addViewer(v.peer);
  }

  function onMessage(m: HostMsg) {
    switch (m.type) {
      case 'created':
        code = m.code; mode = 'resume'; phase = 'live'; status = 'Session ready.';
        saveHostSession({ code: m.code, token: m.host_token, passphrase: activePass }); showSessionUrl(m.code); startMesh(m.ice_servers, []); break;
      case 'resumed':
        activePass = loadHostSession()?.passphrase ?? activePass;
        code = m.code; viewers = m.viewers; phase = 'live'; status = 'Connected.'; showSessionUrl(m.code); startMesh(m.ice_servers, m.viewers);
        if (captured) client?.send({ type: 'set_audio', has_audio: captured.hasAudio, muted });
        break;
      case 'viewer_joined': viewers = [...viewers, { peer: m.peer, nickname: m.nickname }]; status = `${m.nickname} joined.`; mesh?.addViewer(m.peer); break;
      case 'viewer_left': viewers = viewers.filter((v) => v.peer !== m.peer); mesh?.removeViewer(m.peer); break;
      case 'signal': mesh?.handleSignal(m.from, m.data); break;
      case 'error':
        if (m.code === 'not_found' || m.code === 'forbidden') {
          clearHostSession(); mode = 'create'; saved = null; client?.close(); phase = 'setup';
          problem = 'Your previous session has expired. Start a new one.';
          if (!embedded) leaveSessionView();
        } else problem = m.message;
        break;
    }
  }

  async function share() {
    try {
      const c = await captureDisplay();
      captured?.stream.getTracks().forEach((t) => t.stop());
      captured = c;
      stopPause();
      mesh?.setStream(c.stream);
      client?.send({ type: 'set_audio', has_audio: c.hasAudio, muted });
      c.stream.getVideoTracks()[0]?.addEventListener('ended', () => {
        captured = null;
        stopPause();
        mesh?.setStream(null);
        client?.send({ type: 'set_audio', has_audio: false });
        dialog?.showModal();
      });
    } catch { problem = 'Sharing was cancelled or blocked by the browser.'; }
  }

  function endSession() {
    client?.send({ type: 'end' });
    stopPause();
    mesh?.closeAll(); mesh = null;
    client?.close();
    captured?.stream.getTracks().forEach((t) => t.stop());
    captured = null; clearHostSession(); saved = null; mode = 'create'; phase = 'setup'; code = ''; viewers = [];
    passphrase = '';
    dialog?.close();
    leaveSessionView();
  }

  async function copy() {
    try { await navigator.clipboard.writeText(link); status = 'Link copied.'; } catch { status = 'Copy failed. Select the link and copy it manually.'; }
  }
  // Pausing broadcasts a "paused" card as the video itself, so viewers AND the host preview see the same thing.
  function makePauseStream(): MediaStream {
    const c = document.createElement('canvas');
    c.width = 1280; c.height = 720;
    const g = c.getContext('2d')!;
    const draw = () => {
      g.fillStyle = '#0b120e'; g.fillRect(0, 0, 1280, 720);
      g.fillStyle = '#3fb950'; g.beginPath(); g.arc(640, 270, 80, 0, Math.PI * 2); g.fill();
      g.fillStyle = '#0b120e'; g.beginPath(); g.moveTo(621, 238); g.lineTo(621, 302); g.lineTo(677, 270); g.closePath(); g.fill();
      g.textAlign = 'center';
      g.fillStyle = '#e4efe8'; g.font = 'bold 64px system-ui, sans-serif'; g.fillText('Sharu', 640, 430);
      g.fillStyle = '#8aa092'; g.font = '32px system-ui, sans-serif'; g.fillText('The host paused the live. Be patient.', 640, 490);
    };
    draw();
    pauseTimer = window.setInterval(draw, 500); // keeps frames flowing for the encoder
    return c.captureStream(5);
  }

  function stopPause() {
    clearInterval(pauseTimer);
    pauseStream?.getTracks().forEach((t) => t.stop());
    pauseStream = null;
    paused = false;
  }

  function togglePause() {
    if (paused) {
      stopPause();
      if (captured) mesh?.setStream(captured.stream);
      return;
    }
    pauseStream = makePauseStream();
    paused = true;
    mesh?.setStream(pauseStream); // no audio track: viewers hear silence while paused
  }

  function toggleMute() {
    muted = !muted;
    client?.send({ type: 'set_audio', has_audio: true, muted });
  }

</script>

{#if !embedded}<h1>{phase === 'live' ? '🔴 Sharing your screen' : 'Share your screen'}</h1>{/if}
{#if !supported}
  <p class="notice">This browser cannot share a screen. Use a desktop browser; phones and tablets can watch.</p>
{:else}
  {#if phase === 'setup'}
    {#if saved}
      <p class="notice">You have an unfinished session: <code>{saved.code}</code>
        <button class="btn btn-sm" type="button" onclick={() => { mode = 'resume'; void connect(); }}>Resume it</button></p>
    {/if}
    <form class={embedded ? 'flex flex-1 flex-col' : 'card'} onsubmit={(e) => { e.preventDefault(); clearHostSession(); saved = null; mode = 'create'; void connect(); }}>
            <p class="error" role="alert">{problem}</p>
      <button class="btn btn-primary btn-xl mt-auto" type="submit">Start a live</button>
    </form>
  {:else if phase === 'connecting'}
    <p role="status">Connecting…</p><p class="error" role="alert">{problem}</p>
  {:else}
    {#if captured}
      <div class="stage"><video bind:this={preview} autoplay muted playsinline aria-label="Preview of what you are sharing"></video></div>
    {:else}
      <div class="stage aspect-video">
        <button class="btn btn-primary btn-xl max-w-md" type="button" onclick={share}>Choose what to share</button>
      </div>
    {/if}
    <div class="row">
      {#if captured}<button class="btn btn-primary" type="button" onclick={share}>Change source</button>{/if}
      {#if captured?.hasAudio}<button class="btn" type="button" aria-pressed={muted} onclick={toggleMute}>{muted ? 'Unmute sound' : 'Mute sound'}</button>{/if}
      {#if captured}<button class="btn" type="button" aria-pressed={paused} onclick={togglePause}>{paused ? 'Resume live' : 'Pause live'}</button>{/if}
      <button class="btn btn-danger" type="button" onclick={endSession}>End session</button>
    </div>
    {#if captured && !captured.hasAudio}
      <p class="notice">This source has no audio, so viewers will not hear anything. That is how your browser shares this source, not a Sharu problem. {firefox ? 'Firefox cannot share audio yet; use a Chromium browser.' : 'Share a browser tab and tick “Share tab audio” to include sound.'}</p>
    {/if}
    <div class="pill-bar mt-2">
      <span class="text-lg" role="status" aria-live="polite" aria-label={`${viewers.length} watching`}>👤 {viewers.length}</span>
      {#if captured?.hasAudio}<AudioBars stream={captured.stream} active={!muted && !paused} tone="dark" />{/if}
    </div>
    {#if viewers.length}
      <ul class="mt-3 flex flex-wrap gap-2" aria-label="Viewers">
        {#each viewers as v (v.peer)}
          <li class="inline-flex items-center gap-3 rounded-[1.25rem] [corner-shape:squircle] border border-line bg-surface py-1 pl-4 pr-1">
            <span>{v.nickname}</span>
            <button class="btn btn-danger btn-sm" type="button" aria-label={`Kick ${v.nickname}`} onclick={() => client?.send({ type: 'kick', peer: v.peer })}>Kick</button>
          </li>
        {/each}
      </ul>
    {/if}
    <section class="card card-compact mt-6" aria-labelledby="link-h">
      <h2 id="link-h" class="card-title">Share this link</h2>
      <div class="row row-tight"><p class="code">{link}</p><button class="btn btn-primary btn-sm" type="button" onclick={copy}>Copy link</button></div>
    </section>
    <p role="status" aria-live="polite" class="muted">{status}</p>
    <p class="error" role="alert">{problem}</p>
  {/if}
{/if}

<dialog bind:this={dialog} aria-labelledby="ended-h">
  <h2 id="ended-h">Your shared source stopped</h2>
  <p>The window or screen you were sharing closed. Share something else, or end the session.</p>
  <div class="row">
    <button class="btn btn-primary" type="button" onclick={() => { dialog?.close(); void share(); }}>Share again</button>
    <button class="btn btn-danger" type="button" onclick={endSession}>End session</button>
  </div>
</dialog>
