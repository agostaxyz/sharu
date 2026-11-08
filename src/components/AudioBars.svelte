<script lang="ts">
  // Decorative level meter. It only analyses the audio (never plays or alters it). `level` (0..1) is the volume slider.
  let { stream = null, level = 1, active = true, tone = 'brand' }: { stream?: MediaStream | null; level?: number; active?: boolean; tone?: 'brand' | 'dark' } = $props();

  const BARS = 16;
  const MIN = 0.3; // shortest bar as a fraction of the height: quiet audio never looks dead
  let canvas: HTMLCanvasElement | undefined = $state();

  $effect(() => {
    const c = canvas;
    const ctx = c?.getContext('2d');
    if (!c || !ctx) return;
    const s = stream; // tracked: restart when the stream changes
    const color = getComputedStyle(c).getPropertyValue(tone === 'dark' ? '--color-on-brand' : '--color-brand').trim() || '#3fb950';
    const still = matchMedia('(prefers-reduced-motion: reduce)').matches;
    const smooth = new Float32Array(BARS);
    let ac: AudioContext | null = null;
    let analyser: AnalyserNode | null = null;
    let data = new Uint8Array(0);
    let raf = 0;

    const draw = () => {
      if (!still) raf = requestAnimationFrame(draw);
      if (!ac && s && s.getAudioTracks().length > 0) { // the audio track can arrive after the stream does
        ac = new AudioContext();
        const source = ac.createMediaStreamSource(s);
        analyser = ac.createAnalyser();
        analyser.fftSize = 64;
        analyser.smoothingTimeConstant = 0.6;
        source.connect(analyser); // analysis only, not connected to the speakers
        data = new Uint8Array(analyser.frequencyBinCount);
        void ac.resume();
      }
      const on = active && !!analyser && !still;
      if (on) analyser!.getByteFrequencyData(data);
      const scale = 0.45 + 0.55 * Math.min(1, Math.max(0, level)); // quieter volume, slightly shorter bars
      const { width: W, height: H } = c;
      const bw = W / BARS;
      ctx.clearRect(0, 0, W, H);
      ctx.fillStyle = color;
      ctx.globalAlpha = on ? 1 : 0.35;
      for (let i = 0; i < BARS; i++) {
        const v = on ? Math.sqrt((data[Math.floor((i * data.length) / BARS)] ?? 0) / 255) : 0;
        smooth[i] = Math.max(v, (smooth[i] ?? 0) - 0.06); // quick attack, slow decay
        const h = (MIN + smooth[i]! * (1 - MIN)) * H * (on ? scale : 1);
        const x = i * bw + bw * 0.15;
        const w = bw * 0.7;
        ctx.beginPath();
        ctx.roundRect(x, (H - h) / 2, w, h, Math.min(w / 2, 6)); // grows from the centre line outward
        ctx.fill();
      }
    };
    draw();
    return () => { cancelAnimationFrame(raf); void ac?.close(); };
  });
</script>

<canvas bind:this={canvas} width="320" height="52" class="h-6.5 w-40" aria-hidden="true"></canvas>
