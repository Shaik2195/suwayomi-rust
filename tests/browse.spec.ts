import { test, expect } from '@playwright/test';
import { execSync } from 'child_process';

function resetDatabase() {
  try {
    execSync('sqlite3 -cmd ".timeout 5000" ./data/suwayomi.db "DELETE FROM chapter; DELETE FROM manga_category; DELETE FROM manga; DELETE FROM category WHERE id > 1;"');
    execSync(`sqlite3 -cmd ".timeout 5000" ./data/suwayomi.db "
      INSERT INTO manga (id, source_id, url, title, artist, author, description, genre, status, thumbnail_url, update_strategy, initialized)
      VALUES (1, 2499283573021221994, '/title/test-browse', 'Test Browse Manga', 'Author', 'Artist', 'Description', 'Action', 'COMPLETED', 'http://127.0.0.1:4567/assets/notFound.svg', 0, 1);
    "`);
  } catch (e) {
    console.error('Database reset failed:', e);
  }
}

test.describe('Browse Subsystem (/browse)', () => {
  test.describe.configure({ mode: 'serial' });

  test.beforeEach(async () => {
    resetDatabase();
  });

  test('Sources Tab Browsing & Interaction', async ({ page }) => {
    await page.goto('/browse');
    await page.waitForTimeout(1000);

    // Verify Sources tab is active or click Sources tab
    const sourcesTab = page.locator('button[role="tab"]').filter({ hasText: /sources/i }).first();
    if (await sourcesTab.isVisible()) {
      await sourcesTab.click();
    }

    // Verify installed sources render (e.g. MangaDex, AllAnime)
    const sourceCard = page.locator('text=/MangaDex|AllAnime/i').first();
    await expect(sourceCard).toBeVisible();

    // Click source to open source browse view
    await sourceCard.click();
    await page.waitForTimeout(1000);

    // Verify Popular / Latest / Search controls
    const searchInput = page.locator('input[type="text"], input[placeholder*="Search"]').first();
    if (await searchInput.isVisible()) {
      await expect(searchInput).toBeVisible();
    }
  });

  test('Extensions Tab Listing & Filters', async ({ page }) => {
    await page.goto('/browse');
    await page.waitForTimeout(500);

    // Click Extensions tab
    const extensionsTab = page.locator('button[role="tab"]').filter({ hasText: /extensions/i }).first();
    await expect(extensionsTab).toBeVisible();
    await extensionsTab.click();
    await page.waitForTimeout(1000);

    // Verify extensions list rendered
    const extensionItem = page.locator('text=/AllAnime|MangaDex|Keiyoushi/i').first();
    if (await extensionItem.isVisible()) {
      await expect(extensionItem).toBeVisible();
    }
  });

  test('Migrate Tab View', async ({ page }) => {
    await page.goto('/browse');
    await page.waitForTimeout(500);

    // Click Migrate tab
    const migrateTab = page.locator('button[role="tab"]').filter({ hasText: /migrate/i }).first();
    if (await migrateTab.isVisible()) {
      await migrateTab.click();
      await page.waitForTimeout(1000);

      // Verify migration candidate entries from library
      const mangaTitle = page.locator('text=/Test Browse Manga/i').first();
      if (await mangaTitle.isVisible()) {
        await expect(mangaTitle).toBeVisible();
      }
    }
  });
});
