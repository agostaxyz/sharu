#!/usr/bin/env sh
# One-off: add dependencies at their latest versions, then commit pnpm-lock.yaml.
set -eu
pnpm add astro @astrojs/svelte svelte @vite-pwa/astro
pnpm add -D typescript @astrojs/check vitest @playwright/test @axe-core/playwright
pnpm add @fontsource/atkinson-hyperlegible-next @fontsource/jetbrains-mono @fontsource/opendyslexic
pnpm exec playwright install chromium firefox
