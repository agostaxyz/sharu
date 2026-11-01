import { defineConfig, devices } from '@playwright/test';
export default defineConfig({
  testDir: 'e2e',
  webServer: { command: 'pnpm build && pnpm preview', url: 'http://localhost:4321/sharu/', reuseExistingServer: !process.env.CI },
  use: { baseURL: 'http://localhost:4321/sharu/' },
  projects: [
    { name: 'chromium', use: devices['Desktop Chrome'] },
    { name: 'firefox', use: devices['Desktop Firefox'] },
    { name: 'mobile', use: devices['Pixel 7'] },
  ],
});
