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

test.describe('Library Page (/library)', () => {
  test.describe.configure({ mode: 'serial' });

  test.beforeEach(async () => {
    resetDatabase();
  });

  test('Header & Navigation Controls', async ({ page }) => {
    await page.goto('/library');
    await expect(page).toHaveURL('/library');

    // Search bar check & interaction
    const searchButton = page.locator('button[aria-label="Search"]').first();
    await expect(searchButton).toBeVisible();
    await searchButton.click();
    const searchInput = page.locator('input[placeholder*="Search"], input[type="text"]').first();
    await expect(searchInput).toBeVisible();
    await searchInput.fill('Test Manga');
    await searchInput.press('Escape');

    // Random manga button ("Open random entry")
    const randomButton = page.locator('button[aria-label="Open random entry"]').first();
    await expect(randomButton).toBeVisible();

    // Global update button
    const globalUpdateButton = page.locator('button[aria-label*="Global update"]').first();
    await expect(globalUpdateButton).toBeVisible();
    await globalUpdateButton.click();

    // Settings drawer / filter button
    const settingsButton = page.locator('header button[aria-label="Settings"], .MuiToolbar-root button[aria-label="Settings"]').first();
    await expect(settingsButton).toBeVisible();

    // Verify left sidebar navigation links
    const sidebarLinks = [
      { name: 'Library', href: '/library' },
      { name: 'Updates', href: '/updates' },
      { name: 'History', href: '/history' },
      { name: 'Browse', href: '/browse' },
      { name: 'Downloads', href: '/downloads' },
      { name: 'Settings', href: '/settings' },
      { name: 'About', href: '/about' },
    ];

    for (const link of sidebarLinks) {
      const linkEl = page.locator(`a[href="${link.href}"]`).first();
      await expect(linkEl).toBeVisible();
    }

    // Verify library manga grid loads with cover image and title
    const mangaTitle = page.locator('text=Test Manga Alpha');
    await expect(mangaTitle).toBeVisible();
  });

  test('Selection & Batch Operations Toolbar', async ({ page }) => {
    await page.goto('/library');
    await page.waitForTimeout(1000);

    // Hover card and open option menu
    const card = page.locator('.MuiCard-root').first();
    await card.hover();
    const optionsButton = page.locator('button[aria-label="Options"]').first();
    await optionsButton.click();

    // Click "Select" to enter batch selection mode
    await page.locator('text=Select').click();

    // Verify batch selection toolbar elements
    await expect(page.locator('button:has-text("1 manga")')).toBeVisible();
    await expect(page.locator('[aria-label="Clear"]')).toBeVisible();
    await expect(page.locator('[aria-label="Cancel"]')).toBeVisible();

    // Open batch actions menu
    await page.locator('button:has-text("1 manga")').click();

    // Test "Change categories of selected"
    const setCategoriesOption = page.locator('text=Change categories of selected');
    await expect(setCategoriesOption).toBeVisible();
    await setCategoriesOption.click();

    // Set categories dialog opens
    const dialog = page.locator('.MuiDialog-root');
    await expect(dialog).toBeVisible();

    // Toggle category "Default" and save / close dialog
    const defaultCheckbox = dialog.locator('input[type="checkbox"]').first();
    if (await defaultCheckbox.isVisible()) {
      await defaultCheckbox.click({ force: true });
    }
    await page.keyboard.press('Escape');
    await page.waitForTimeout(500);

    // Test "Mark selected as read"
    if (!(await page.locator('button:has-text("1 manga")').isVisible())) {
      await card.hover();
      await optionsButton.click();
      await page.locator('text=Select').click();
    }
    await page.locator('button:has-text("1 manga")').click();
    const markReadOption = page.locator('text=Mark selected as read');
    await expect(markReadOption).toBeVisible();
    await markReadOption.click();
    await page.waitForTimeout(500);

    // Test "Download selected"
    await page.goto('/library');
    await page.waitForTimeout(500);
    await card.hover();
    await optionsButton.click();
    await page.locator('text=Select').click();
    await page.locator('button:has-text("1 manga")').click();
    const downloadOption = page.locator('text=Download selected');
    await expect(downloadOption).toBeVisible();
    await downloadOption.click();
    await page.waitForTimeout(500);

    // Test "Migrate selected"
    await page.goto('/library');
    await page.waitForTimeout(500);
    await card.hover();
    await optionsButton.click();
    await page.locator('text=Select').click();
    await page.locator('button:has-text("1 manga")').click();
    const migrateOption = page.locator('text=Migrate selected');
    await expect(migrateOption).toBeVisible();
    await migrateOption.click();
    await page.waitForTimeout(500);

    // Test "Remove selected from the library"
    await page.goto('/library');
    await page.waitForTimeout(500);
    await card.hover();
    await optionsButton.click();
    await page.locator('text=Select').click();
    await page.locator('button:has-text("1 manga")').click();

    const removeOption = page.locator('text=Remove selected from the library');
    await expect(removeOption).toBeVisible();
    await removeOption.click();

    // Confirmation modal should appear or manga removed
    const confirmBtn = page.locator('.MuiDialog-root button:has-text("Remove"), .MuiDialog-root button:has-text("OK"), .MuiDialog-root button:has-text("Yes")').first();
    if (await confirmBtn.isVisible()) {
      await confirmBtn.click();
    }
  });
});
