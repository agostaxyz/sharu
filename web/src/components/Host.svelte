<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { createSignal, signallingMode, type Signal } from '../lib/signal';
  import { captureDisplay, canShare, isFirefox, type Captured } from '../lib/capture';
  import { loadHostSession, saveHostSession, clearHostSession, type HostSession } from '../lib/settings';
  import { withBase } from '../lib/base';
  import { generatePassphrase } from '../lib/code';
  import { HostMesh } from '../lib/mesh';
  import type { IceServer, ServerMsg, ViewerInfo } from '../lib/proto';

  // embedded = shown inside the "Share your screen" card on the home page; otherwise it is the full /<code> page.
  let { embedded = false }: { embedded?: boolean } = $props();

  type Phase = 'setup' | 'connecting' | 'live';
  let phase = $state<Phase>('setup');
  let passphrase = $state('');
  let suggested = $state(generatePassphrase());
  let activePass = $state('');
  let code = $state('');
  let viewers = $state<ViewerInfo[]>([]);
  let problem = $state('');
  let status = $state('');
  let saved = $state<HostSession | null>(null);
  let captured = $state<Captured | null>(null);
  let preview: HTMLVideoElement | undefined = $state();
  let dialog: HTMLDialogElement | undefined = $state();
  let client: Signal | null = null;
  let mesh: HostMesh | null = null;
  let mode: 'create' | 'resume' = 'create';
  const firefox = isFirefox();
  const supported = canShare();
  const serverless = signallingMode() === 'p2p';
  const link = $derived(code ? `${location.origin}${withBase(`/${code}`)}` : '');

  // The address bar shows the livestream link while hosting. The tab flag (sessionStorage) survives reloads of THIS tab only,
  // so opening the same link in another tab, even in this browser, gives the viewer experience.
  function showSessionUrl(c: string) {
    try { sessionStorage.setItem('sharu.hostTab', c.toLowerCase()); } catch { /* ignore */ }
    history.replaceState(null, '', withBase(`/${c}`));
    document.documentElement.dataset.hosting = '1';
    if (!embedded) document.title = 'Sharing · sharu';
  }
  function leaveSessionView() {
    try { sessionStorage.removeItem('sharu.hostTab'); } catch { /* ignore */ }
    delete document.documentElement.dataset.hosting;
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
  $effect(() => { if (preview) preview.srcObject = captured?.stream ?? null; });
  // Warn before leaving while actively sharing. If the source simply stops, closing is fine.
  $effect(() => {
    if (!captured) return;
    const warn = (e: BeforeUnloadEvent) => { e.preventDefault(); e.returnValue = ''; };
    window.addEventListener('beforeunload', warn);
    return () => window.removeEventListener('beforeunload', warn);
  });

  async function connect() {
    if (mode === 'create') activePass = passphrase.trim() || suggested;
    phase = 'connecting'; problem = '';
    client?.close();
    client = await createSignal('host', {
      open: () => {
        const s = loadHostSession();
        if (mode === 'resume' && s) client?.send({ type: 'resume', code: s.code, host_token: s.token });
        else client?.send({ type: 'create', has_audio: captured?.hasAudio ?? false, passphrase: activePass });
      },
      message: onMessage,
      close: () => { if (phase === 'live') status = 'Connection lost. Reconnecting…'; },
    });
    client.connect();
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
        code = m.code; mode = 'resume'; phase = 'live'; status = 'Session ready.';
        saveHostSession({ code: m.code, token: m.host_token, passphrase: activePass }); showSessionUrl(m.code); startMesh(m.ice_servers, []); break;
      case 'resumed':
        activePass = loadHostSession()?.passphrase ?? activePass;
        code = m.code; viewers = m.viewers; phase = 'live'; status = 'Connected.'; showSessionUrl(m.code); startMesh(m.ice_servers, m.viewers);
        if (captured) client?.send({ type: 'set_audio', has_audio: captured.hasAudio });
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
    captured = null; clearHostSession(); saved = null; mode = 'create'; phase = 'setup'; code = ''; viewers = [];
    passphrase = ''; suggested = generatePassphrase();
    dialog?.close();
    leaveSessionView();
  }

  async function copy() {
    try { await navigator.clipboard.writeText(link); status = 'Link copied.'; } catch { status = 'Copy failed. Select the link and copy it manually.'; }
  }
  async function copyPass() {
    try { await navigator.clipboard.writeText(activePass); status = 'Passphrase copied.'; } catch { status = 'Copy failed. Select the passphrase and copy it manually.'; }
  }
</script>

{#if !embedded}<h1>Share your screen</h1>{/if}
{#if !supported}
  <p class="notice">This browser cannot share a screen. Use a desktop browser; phones and tablets can watch.</p>
{:else}
  {#if phase === 'setup'}
    {#if saved}
      <p class="notice">You have an unfinished session: <code>{saved.code}</code>
        <button class="btn btn-sm" type="button" onclick={() => { mode = 'resume'; void connect(); }}>Resume it</button></p>
    {/if}
    <form class={embedded ? '' : 'card'} onsubmit={(e) => { e.preventDefault(); clearHostSession(); saved = null; mode = 'create'; void connect(); }}>
      <label for="pass">Passphrase</label>
      <input id="pass" type="text" autocomplete="off" autocapitalize="off" spellcheck="false" placeholder={suggested} bind:value={passphrase} aria-describedby="pass-help" />
      <p id="pass-help" class="muted">Viewers need it together with the link. Leave it empty to use the suggestion.</p>
      <p class="error" role="alert">{problem}</p>
      <button class="btn btn-primary btn-xl" type="submit">Create Session</button>
    </form>
  {:else if phase === 'connecting'}
    <p role="status">Connecting…</p><p class="error" role="alert">{problem}</p>
  {:else}
    <section class="card card-compact" aria-labelledby="link-h">
      <h2 id="link-h" class="card-title">Share this link</h2>
      <div class="row row-tight"><p class="code">{link}</p><button class="btn btn-primary btn-sm" type="button" onclick={copy}>Copy link</button></div>
    </section>
    <section class="card card-compact" aria-labelledby="pass-h">
      <h2 id="pass-h" class="card-title">Passphrase</h2>
      <div class="row row-tight"><p class="code">{activePass}</p><button class="btn btn-sm" type="button" onclick={copyPass}>Copy passphrase</button></div>
    </section>
    {#if serverless}
      <p class="notice">Serverless mode (experimental): no relay server is used. Viewers can see each other's connection details, and anyone who knows the link can try to join, so add a passphrase for private sessions.</p>
    {/if}
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
    <button class="btn btn-primary" type="button" onclick={() => { dialog?.close(); void share(); }}>Share again</button>
    <button class="btn btn-danger" type="button" onclick={endSession}>End session</button>
  </div>
</dialog>
