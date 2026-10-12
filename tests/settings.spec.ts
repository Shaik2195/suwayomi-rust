import { test, expect } from '@playwright/test';
import { execSync } from 'child_process';

function resetDatabase() {
  try {
    execSync('sqlite3 -cmd ".timeout 5000" ./data/suwayomi.db "DELETE FROM chapter; DELETE FROM manga_category; DELETE FROM manga; DELETE FROM category WHERE id > 1;"');
  } catch (e) {
    console.error('Database reset failed:', e);
  }
}

test.describe('Settings Subsystem (/settings)', () => {
  test.describe.configure({ mode: 'serial' });

  test.beforeEach(async () => {
    resetDatabase();
  });

  test('Settings Sections Navigation', async ({ page }) => {
    await page.goto('/settings');
    await expect(page).toHaveURL('/settings');

    // Verify main settings navigation items
    const generalLink = page.locator('text=/General/i').first();
    const appearanceLink = page.locator('text=/Appearance/i').first();
    const categoriesLink = page.locator('text=/Categories/i').first();
    const trackingLink = page.locator('text=/Tracking/i').first();

    await expect(generalLink).toBeVisible();
    await expect(appearanceLink).toBeVisible();
    await expect(categoriesLink).toBeVisible();
    await expect(trackingLink).toBeVisible();
  });

  test('Tracking Settings (/settings/tracking)', async ({ page }) => {
    await page.goto('/settings/tracking');
    await expect(page).toHaveURL('/settings/tracking');

    // Verify presence of supported trackers
    await expect(page.locator('text=/AniList/i').first()).toBeVisible();
    await expect(page.locator('text=/MyAnimeList/i').first()).toBeVisible();
    await expect(page.locator('text=/Kitsu/i').first()).toBeVisible();
    await expect(page.locator('text=/Shikimori/i').first()).toBeVisible();
    await expect(page.locator('text=/Bangumi/i').first()).toBeVisible();
  });

  test('Category Management (/settings/categories)', async ({ page }) => {
    await page.goto('/settings/categories');
    await expect(page).toHaveURL('/settings/categories');

    // Verify Default category is present
    await expect(page.locator('text=/Default/i').first()).toBeVisible();

    // Find "Add category" or fab button
    const addCategoryBtn = page.locator('button[aria-label*="Add"], button:has-text("Add"), button:has-text("New")').first();
    if (await addCategoryBtn.isVisible()) {
      await addCategoryBtn.click();
      await page.waitForTimeout(500);

      // Enter category name in dialog
      const nameInput = page.locator('.MuiDialog-root input[type="text"]').first();
      if (await nameInput.isVisible()) {
        await nameInput.fill('Favorites');
        const submitBtn = page.locator('.MuiDialog-root button:has-text("OK"), .MuiDialog-root button:has-text("Add")').first();
        await submitBtn.click();
        await page.waitForTimeout(500);

        // Verify newly created category is displayed
        await expect(page.locator('text=Favorites').first()).toBeVisible();
      }
    }
  });
});
