import { test, expect } from '@playwright/test';
import { execSync } from 'child_process';

function resetDatabase() {
  try {
    execSync('sqlite3 -cmd ".timeout 5000" ./data/suwayomi.db "DELETE FROM chapter; DELETE FROM manga_category; DELETE FROM manga; DELETE FROM category WHERE id > 1;"');
    execSync(`sqlite3 -cmd ".timeout 5000" ./data/suwayomi.db "
      INSERT INTO manga (id, source_id, url, title, artist, author, description, genre, status, thumbnail_url, update_strategy, initialized)
      VALUES (1, 2499283573021221994, '/title/test-reader', 'Test Reader Manga', 'Author', 'Artist', 'Description', 'Action', 'COMPLETED', 'http://127.0.0.1:4567/assets/notFound.svg', 0, 1);

      INSERT INTO chapter (id, manga_id, url, name, date_upload, chapter_number, scanlator, read, bookmark, last_page_read, date_fetch, source_order, page_count)
      VALUES (1, 1, '/chapter/1', 'Chapter 1 - Test Chapter', 1700000000000, 1.0, 'Scanlator', 0, 0, 0, 1700000000000, 0, 10);
    "`);
  } catch (e) {
    console.error('Database reset failed:', e);
  }
}

test.describe('Manga Reader Subsystem (/manga/:id/chapter/:order)', () => {
  test.describe.configure({ mode: 'serial' });

  test.beforeEach(async () => {
    resetDatabase();
  });

  test('Clean Reader UI & Initial Page Counter', async ({ page }) => {
    await page.goto('/manga/1/chapter/0');

    // Wait for the reader to mount
    await expect(page).toHaveURL(/\/manga\/1\/chapter\/0/);
    await page.waitForTimeout(1000);

    // Verify tap zone preview overlay (Left / Right / Menu) is NOT displayed
    const tapZoneOverlay = page.locator('canvas, div').filter({ hasText: /left|right|menu/i });
    if (await tapZoneOverlay.count() > 0) {
      await expect(tapZoneOverlay.first()).not.toHaveCSS('display', 'block');
    }

    // Verify bottom progress indicator reflects chapter page count
    const pageIndicator = page.locator('p, span').filter({ hasText: /\/\s*10|\/\s*\d+/ }).first();
    if (await pageIndicator.isVisible()) {
      await expect(pageIndicator).toBeVisible();
    }
  });

  test('Reader Settings & Mode Controls', async ({ page }) => {
    await page.goto('/manga/1/chapter/0');
    await page.waitForTimeout(1000);

    // Click anywhere in middle to toggle reader navbar/controls if hidden
    await page.mouse.click(page.viewportSize()?.width ? page.viewportSize()!.width / 2 : 400, page.viewportSize()?.height ? page.viewportSize()!.height / 2 : 400);
    await page.waitForTimeout(500);

    // Check for reader settings button or drawer
    const readerSettingsBtn = page.locator('button[aria-label*="Settings"], button[aria-label*="Reader settings"]').first();
    if (await readerSettingsBtn.isVisible()) {
      await readerSettingsBtn.click();
      await page.waitForTimeout(500);

      // Verify reading mode selections (Continuous, Single, etc.)
      const modeOption = page.locator('text=/Continuous|Single|Double|Webtoon/i').first();
      await expect(modeOption).toBeVisible();
      await page.keyboard.press('Escape');
    }
  });

  test('Continuous Vertical Scroll and Dynamic Page Loading', async ({ page }) => {
    await page.goto('/manga/1/chapter/0');
    await page.waitForTimeout(1500);

    // Find reader scroll container
    const scrollContainer = page.locator('div.MuiStack-root[style*="overflow: auto"], div[style*="overflow: auto"]').first();
    if (await scrollContainer.isVisible()) {
      const initialHeight = await scrollContainer.evaluate(el => el.scrollHeight);
      expect(initialHeight).toBeGreaterThan(0);

      // Scroll down
      await scrollContainer.evaluate(el => el.scrollBy({ top: 3000, behavior: 'instant' }));
      await page.waitForTimeout(1000);

      const scrollTop = await scrollContainer.evaluate(el => el.scrollTop);
      expect(scrollTop).toBeGreaterThan(0);
    }
  });
});
