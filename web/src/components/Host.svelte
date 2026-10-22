<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { SignalClient, signalUrl } from '../lib/signal';
  import { captureDisplay, canShare, isFirefox, type Captured } from '../lib/capture';
  import { loadHostSession, saveHostSession, clearHostSession } from '../lib/settings';
  import { withBase } from '../lib/base';
  import { HostMesh } from '../lib/mesh';
  import type { IceServer, ServerMsg, ViewerInfo } from '../lib/proto';

  type Phase = 'setup' | 'connecting' | 'live';
  let phase = $state<Phase>('setup');
  let passphrase = $state('');
  let code = $state('');
  let viewers = $state<ViewerInfo[]>([]);
  let problem = $state('');
  let status = $state('');
  let captured = $state<Captured | null>(null);
  let preview: HTMLVideoElement | undefined = $state();
  let dialog: HTMLDialogElement | undefined = $state();
  let client: SignalClient | null = null;
  let mesh: HostMesh | null = null;
  let mode: 'create' | 'resume' = 'create';
  const firefox = isFirefox();
  const supported = canShare();
  const link = $derived(code ? `${location.origin}${withBase(`/${code}`)}` : '');

  const pagehide = () => client?.send({ type: 'leaving' });
  onMount(() => {
    window.addEventListener('pagehide', pagehide);
    if (loadHostSession()) { mode = 'resume'; connect(); } // refresh or reconnect after a crash
  });
  onDestroy(() => { window.removeEventListener('pagehide', pagehide); mesh?.closeAll(); client?.close(); });
  $effect(() => { if (preview) preview.srcObject = captured?.stream ?? null; });
  // Warn before leaving while actively sharing. If the source simply stops, closing is fine.
  $effect(() => {
    if (!captured) return;
    const warn = (e: BeforeUnloadEvent) => { e.preventDefault(); e.returnValue = ''; };
    window.addEventListener('beforeunload', warn);
    return () => window.removeEventListener('beforeunload', warn);
  });

  function connect() {
    const url = signalUrl();
    if (!url) { problem = 'This site has no signalling server configured yet.'; return; }
    phase = 'connecting'; problem = '';
    client = new SignalClient(url, {
      open: () => {
        const saved = loadHostSession();
        if (mode === 'resume' && saved) client?.send({ type: 'resume', code: saved.code, host_token: saved.token });
        else client?.send({ type: 'create', has_audio: captured?.hasAudio ?? false, passphrase: passphrase.trim() || undefined });
      },
      message: onMessage,
      close: () => { if (phase === 'live') status = 'Connection lost. Reconnecting…'; },
    });
    client.connect();
  }

  // The address bar shows the livestream link while hosting, and /host/ otherwise.
  function showSessionUrl(c: string) {
    history.replaceState({ sharuHost: true }, '', withBase(`/${c}`)); // marks THIS tab as the host view (survives reloads)
    document.title = 'Sharing · sharu';
  }
  function showSetupUrl() {
    history.replaceState(null, '', withBase('/host/'));
    document.title = 'Share your screen · sharu';
  }

  function startMesh(ice: IceServer[], existing: ViewerInfo[]) {
    mesh?.closeAll();
    mesh = new HostMesh(ice, (to, data) => client?.send({ type: 'signal', to, data }));
    if (captured) mesh.setStream(captured.stream);
    for (const v of existing) mesh.addViewer(v.peer);
  }

  function onMessage(m: ServerMsg) {
    switch (m.type) {
      case 'created':
        code = m.code; mode = 'resume'; phase = 'live'; status = 'Session ready.'; showSessionUrl(m.code);
        saveHostSession({ code: m.code, token: m.host_token }); startMesh(m.ice_servers, []); break;
      case 'resumed':
        code = m.code; viewers = m.viewers; phase = 'live'; status = 'Connected.'; showSessionUrl(m.code); startMesh(m.ice_servers, m.viewers);
        if (captured) client?.send({ type: 'set_audio', has_audio: captured.hasAudio });
        break;
      case 'viewer_joined': viewers = [...viewers, { peer: m.peer, nickname: m.nickname }]; status = `${m.nickname} joined.`; mesh?.addViewer(m.peer); break;
      case 'viewer_left': viewers = viewers.filter((v) => v.peer !== m.peer); mesh?.removeViewer(m.peer); break;
      case 'signal': mesh?.handleSignal(m.from, m.data); break;
      case 'error':
        if (m.code === 'not_found' || m.code === 'forbidden') { clearHostSession(); mode = 'create'; phase = 'setup'; showSetupUrl(); problem = 'Your previous session has expired. Start a new one.'; client?.close(); }
        else problem = m.message;
        break;
    }
  }

  async function share() {
    try {
      const c = await captureDisplay();
      captured?.stream.getTracks().forEach((t) => t.stop());
      captured = c;
      mesh?.setStream(c.stream);
      client?.send({ type: 'set_audio', has_audio: c.hasAudio });
      c.stream.getVideoTracks()[0]?.addEventListener('ended', () => {
        captured = null;
        mesh?.setStream(null);
        client?.send({ type: 'set_audio', has_audio: false });
        dialog?.showModal();
      });
    } catch { problem = 'Sharing was cancelled or blocked by the browser.'; }
  }

  function endSession() {
    client?.send({ type: 'end' });
    mesh?.closeAll(); mesh = null;
    client?.close();
    captured?.stream.getTracks().forEach((t) => t.stop());
    captured = null; clearHostSession(); mode = 'create'; phase = 'setup'; code = ''; viewers = []; dialog?.close(); showSetupUrl();
  }

  async function copy() {
    try { await navigator.clipboard.writeText(link); status = 'Link copied.'; } catch { status = 'Copy failed. Select the link and copy it manually.'; }
  }
</script>

<h1>Share your screen</h1>
{#if !supported}
  <p class="notice">This browser cannot share a screen. Use a desktop browser.</p>
{:else}
  {#if firefox}
    <p class="notice">Firefox does not support audio via <a href="https://developer.mozilla.org/en-US/docs/Web/API/MediaDevices/getDisplayMedia" rel="noopener noreferrer"><code>getDisplayMedia()</code></a> yet, sorry. Use a Chromium browser if you want to share sound. Video works fine.</p>
  {/if}
  {#if phase === 'setup'}
    <form class="card" onsubmit={(e) => { e.preventDefault(); mode = 'create'; connect(); }}>
      <label for="pass">Passphrase (optional)</label>
      <input id="pass" type="password" autocomplete="off" bind:value={passphrase} aria-describedby="pass-help" />
      <p id="pass-help" class="muted">Viewers need it in addition to the link.</p>
      <p class="error" role="alert">{problem}</p>
      <button class="btn btn-primary btn-xl" type="submit">Create session</button>
    </form>
  {:else if phase === 'connecting'}
    <p role="status">Connecting…</p><p class="error" role="alert">{problem}</p>
  {:else}
    <section class="card card-compact" aria-labelledby="link-h">
      <h2 id="link-h" class="card-title">Share this link</h2>
      <div class="row row-tight"><p class="code">{link}</p><button class="btn btn-primary btn-sm" type="button" onclick={copy}>Copy link</button></div>
    </section>
    <div class="row">
      <button class="btn btn-primary" type="button" onclick={share}>{captured ? 'Change source' : 'Choose what to share'}</button>
      <button class="btn btn-danger" type="button" onclick={endSession}>End session</button>
    </div>
    {#if captured}
      <div class="stage"><video bind:this={preview} autoplay muted playsinline aria-label="Preview of what you are sharing"></video></div>
      {#if !captured.hasAudio}
        <p class="notice">No audio was captured. {firefox ? 'Firefox cannot share audio yet; use a Chromium browser.' : 'Share a browser tab and tick "Share tab audio".'}</p>
      {/if}
    {/if}
    <section aria-labelledby="viewers-h">
      <h2 id="viewers-h">Watching: {viewers.length}</h2>
      <ul class="plain">
        {#each viewers as v (v.peer)}
          <li class="row"><span>{v.nickname}</span>
            <button class="btn btn-danger btn-sm" type="button" aria-label={`Kick ${v.nickname}`} onclick={() => client?.send({ type: 'kick', peer: v.peer })}>Kick</button></li>
        {/each}
      </ul>
    </section>
    <p role="status" aria-live="polite" class="muted">{status}</p>
    <p class="error" role="alert">{problem}</p>
  {/if}
{/if}

<dialog bind:this={dialog} aria-labelledby="ended-h">
  <h2 id="ended-h">Your shared source stopped</h2>
  <p>The window or screen you were sharing closed. Share something else, or end the session.</p>
  <div class="row">
    <button class="btn btn-primary" type="button" onclick={() => { dialog?.close(); share(); }}>Share again</button>
    <button class="btn btn-danger" type="button" onclick={endSession}>End session</button>
  </div>
</dialog>
