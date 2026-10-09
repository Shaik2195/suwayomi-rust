import { test, expect } from '@playwright/test';
import * as fs from 'fs';
import * as path from 'path';

test.describe('Suwayomi Server API & UI', () => {
  test('GraphQL Playground loads and works', async ({ page }) => {
    // Navigate to /graphql
    const response = await page.goto('/graphql');
    
    // Verify status code
    expect(response?.status()).toBe(200);
    
    // Verify it's the GraphQL Playground or Apollo Server HTML
    const content = await page.content();
    expect(content.toLowerCase()).toContain('graphql');
    
    // Capture screenshot
    const screenshotDir = path.join(__dirname, 'screenshots');
    if (!fs.existsSync(screenshotDir)) {
      fs.mkdirSync(screenshotDir, { recursive: true });
    }
    await page.screenshot({ path: path.join(screenshotDir, 'playground.png') });
  });

  test('GraphQL POST query execution', async ({ request }) => {
    const response = await request.post('/graphql', {
      data: {
        query: '{ categories { id name } }'
      }
    });

    expect(response.ok()).toBeTruthy();
    
    const json = await response.json();
    expect(json).toHaveProperty('data');
    expect(json.data).toHaveProperty('categories');
    expect(Array.isArray(json.data.categories)).toBeTruthy();
  });

  test('REST GET /api/v1/category returns valid JSON', async ({ request }) => {
    const response = await request.get('/api/v1/category');
    
    expect(response.ok()).toBeTruthy();
    
    const json = await response.json();
    // Assuming the response is an array or object containing category data
    expect(json).toBeDefined();
  });
});
