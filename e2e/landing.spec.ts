import AxeBuilder from '@axe-core/playwright';
import { expect, test } from '@playwright/test';

test('landing page has no detectable accessibility violations', async ({ page }) => {
  await page.goto('./');
  const results = await new AxeBuilder({ page }).analyze();
  expect(results.violations).toEqual([]);
});

test('the one action on the home page is starting a live', async ({ page }) => {
  await page.goto('./');
  await expect(page.getByRole('button', { name: 'Start a live' })).toBeVisible();
});

test('a session link opens the join page', async ({ page }) => {
  await page.goto('./Caesar-23-Pyramid');
  await expect(page.getByRole('button', { name: 'Join Session' })).toBeVisible();
});
