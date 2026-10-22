import AxeBuilder from '@axe-core/playwright';
import { expect, test } from '@playwright/test';

test('landing page has no detectable accessibility violations', async ({ page }) => {
  await page.goto('./');
  const results = await new AxeBuilder({ page }).analyze();
  expect(results.violations).toEqual([]);
});

test('invalid code shows an announced error', async ({ page }) => {
  await page.goto('./');
  await page.getByLabel('Session link or code').fill('nope');
  await page.getByRole('button', { name: 'Join' }).click();
  await expect(page.getByRole('alert').first()).toContainText('does not look like a session code');
});

test('valid code navigates to the viewer', async ({ page }) => {
  await page.goto('./');
  await page.getByLabel('Session link or code').fill('Caesar-23-Pyramid');
  await page.getByRole('button', { name: 'Join' }).click();
  await expect(page).toHaveURL(/Caesar-23-Pyramid$/);
});
