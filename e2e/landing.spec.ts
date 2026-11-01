import AxeBuilder from '@axe-core/playwright';
import { expect, test } from '@playwright/test';

test('landing page has no detectable accessibility violations', async ({ page }) => {
  await page.goto('./');
  const results = await new AxeBuilder({ page }).analyze();
  expect(results.violations).toEqual([]);
});

test('invalid link shows an announced error', async ({ page }) => {
  await page.goto('./');
  await page.getByLabel('Session link').fill('nope');
  await page.getByRole('button', { name: 'Join Session' }).click();
  await expect(page.locator('#code-error')).toContainText('does not look like a session link');
});

test('valid link navigates to the session page', async ({ page }) => {
  await page.goto('./');
  await page.getByLabel('Session link').fill('Caesar-23-Pyramid');
  await page.getByRole('button', { name: 'Join Session' }).click();
  await expect(page).toHaveURL(/Caesar-23-Pyramid$/);
});
