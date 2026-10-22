// SPDX-License-Identifier: AGPL-3.0-or-later
import { defineConfig } from 'astro/config';
import svelte from '@astrojs/svelte';
import AstroPWA from '@vite-pwa/astro';

// GitHub Pages project site: https://agostaxyz.github.io/sharu/
// Self-hosting at a domain root: SHARU_BASE=/ pnpm build
const base = process.env.SHARU_BASE ?? '/sharu';

export default defineConfig({
  site: process.env.SHARU_SITE ?? 'https://agostaxyz.github.io',
  base,
  output: 'static',
  build: { inlineStylesheets: 'never' },
  integrations: [
    svelte(),
    AstroPWA({
      registerType: 'autoUpdate',
      injectRegister: 'script',
      manifest: {
        name: 'sharu',
        short_name: 'sharu',
        description: 'Peer-to-peer screen sharing in your browser.',
        start_url: base.endsWith('/') ? base : `${base}/`,
        scope: base.endsWith('/') ? base : `${base}/`,
        display: 'standalone',
        background_color: '#0b120e',
        theme_color: '#0b120e',
        icons: [
          { src: 'icon-192.png', sizes: '192x192', type: 'image/png' },
          { src: 'icon-512.png', sizes: '512x512', type: 'image/png' },
          { src: 'icon-maskable-512.png', sizes: '512x512', type: 'image/png', purpose: 'maskable' },
        ],
      },
      // App shell only. Never cache signalling; unknown URLs (session codes) go to the network/404 page.
      workbox: { navigateFallback: undefined, globPatterns: ['**/*.{js,css,html,png,woff2}'] },
    }),
  ],
  vite: {
    build: { assetsInlineLimit: 0 }, // keep everything external so a strict CSP works
    server: { proxy: { '/ws': { target: 'http://127.0.0.1:8080', ws: true } } },
  },
});
