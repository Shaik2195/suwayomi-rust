import { test, expect } from '@playwright/test';
import { execSync } from 'child_process';

function resetDatabase() {
  try {
    execSync('sqlite3 -cmd ".timeout 5000" ./data/suwayomi.db "DELETE FROM chapter; DELETE FROM manga_category; DELETE FROM manga; DELETE FROM category WHERE id > 1;"');
    execSync(`sqlite3 -cmd ".timeout 5000" ./data/suwayomi.db "
      INSERT INTO manga (id, source_id, url, title, artist, author, description, genre, status, thumbnail_url, update_strategy, initialized)
      VALUES (1, 2499283573021221994, '/title/test-1', 'Test Manga Alpha', 'Test Artist', 'Test Author', 'A test manga description', 'Action, Drama', 'ONGOING', 'http://127.0.0.1:4567/assets/notFound.svg', 0, 1);

      INSERT INTO chapter (id, manga_id, url, name, date_upload, chapter_number, scanlator, read, bookmark, last_page_read, date_fetch, source_order, page_count)
      VALUES (1, 1, '/chapter/1', 'Chapter 1 - Beginning', 1700000000000, 1.0, 'Test Scanlator', 0, 0, 0, 1700000000000, 0, 10);

      INSERT INTO chapter (id, manga_id, url, name, date_upload, chapter_number, scanlator, read, bookmark, last_page_read, date_fetch, source_order, page_count)
      VALUES (2, 1, '/chapter/2', 'Chapter 2 - Continuation', 1700000000000, 2.0, 'Test Scanlator', 0, 0, 0, 1700000000000, 1, 10);
    "`);
  } catch (e) {
    console.error('Database reset failed:', e);
  }
}

test.describe('Manga Details View (/manga/:id)', () => {
  test.describe.configure({ mode: 'serial' });

  test.beforeEach(async () => {
    resetDatabase();
  });

  test('Metadata & Information Display', async ({ page }) => {
    await page.goto('/manga/1');
    await expect(page).toHaveURL('/manga/1');

    // Title, Author, Artist, Status, Source
    await expect(page.locator('h1, h2, h3, .MuiTypography-root').filter({ hasText: 'Test Manga Alpha' }).first()).toBeVisible();
    await expect(page.locator('text=Test Author').first()).toBeVisible();
    await expect(page.locator('text=Test Artist').first()).toBeVisible();
    await expect(page.locator('text=MangaDex').first()).toBeVisible();
  });

  test('Primary Actions & Toolbar Controls', async ({ page }) => {
    await page.goto('/manga/1');

    // Test "In Library" / "Add To Library" toggle button
    const libraryToggleBtn = page.locator('button:has-text("ADD TO LIBRARY"), button:has-text("IN LIBRARY")').first();
    await expect(libraryToggleBtn).toBeVisible();

    // Test "Tracking" button -> navigates to tracking settings / modal
    const trackingBtn = page.locator('button:has-text("TRACKING")').first();
    await expect(trackingBtn).toBeVisible();
    await trackingBtn.click();
    await expect(page.locator('text=/anilist/i').first()).toBeVisible();
    await expect(page.locator('text=/myanimelist/i').first()).toBeVisible();

    // Test "Reload data from source" button
    await page.goto('/manga/1');
    const reloadBtn = page.locator('button[aria-label="Reload data from source"]').first();
    await expect(reloadBtn).toBeVisible();
    await reloadBtn.click();
  });

  test('Chapter List & Context Menu Actions', async ({ page }) => {
    await page.goto('/manga/1');

    // Chapter titles render
    await expect(page.locator('text=Chapter 1 - Beginning').first()).toBeVisible();
    await expect(page.locator('text=Chapter 2 - Continuation').first()).toBeVisible();

    // Test "Filter and sort" drawer
    const filterBtn = page.locator('button[aria-label="Filter and sort"]').first();
    await expect(filterBtn).toBeVisible();
    await filterBtn.click();
    await page.keyboard.press('Escape');
    await page.waitForTimeout(300);

    // Single chapter "more" menu
    const moreBtn = page.locator('button[aria-label="more"]').first();
    await expect(moreBtn).toBeVisible();
    await moreBtn.click();

    // Verify context menu options: Download, Mark as read, Mark previous as read, Add bookmark
    await expect(page.locator('text=Download').first()).toBeVisible();
    await expect(page.locator('text=Mark as read').first()).toBeVisible();
    await expect(page.locator('text=Mark previous as read').first()).toBeVisible();
    await expect(page.locator('text=Add bookmark').first()).toBeVisible();

    // Click "Add bookmark"
    await page.locator('text=Add bookmark').first().click();
    await page.waitForTimeout(500);

    // Navigate to reader by clicking chapter link
    const chapterLink = page.locator('a[href="/manga/1/chapter/0"]').first();
    await expect(chapterLink).toBeVisible();
    await chapterLink.click();
    await expect(page).toHaveURL(/\/manga\/1\/chapter\/0/);
  });
});
