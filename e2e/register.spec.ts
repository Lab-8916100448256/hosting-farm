import { test, expect } from '@playwright/test';

test.describe.configure({ mode: 'serial' });

let page: Page;

test.beforeAll(async ({ browser }) => {
  page = await browser.newPage();
});

test.afterAll(async () => {
  await page.close();
});

test.describe('basic register flow', () => {
  test('sign up', async () => {
    await page.goto('http://localhost:5151/');
    await expect(page.getByRole('link', { name: 'Hosting Farm' })).toBeVisible();
    await page.getByRole('link', { name: 'Sign up' }).click();
    await page.getByRole('textbox', { name: 'Full name' }).fill('Mr Test');
    await page.getByRole('textbox', { name: 'Email address' }).fill('mr.test@example.com');
    await page.getByRole('textbox', { name: 'Password', exact: true }).fill('test!');
    await page.getByRole('textbox', { name: 'Confirm password' }).fill('test!');
    await page.getByRole('button', { name: 'Create account' }).click();
    await expect(page.getByRole('main')).toContainText('Sign in to your account');
  });

  test('log in', async () => {
    await page.goto('http://localhost:5151/auth/login');
    await page.getByRole('textbox', { name: 'Email address' }).fill('mr.test@example.com');
    await page.getByRole('textbox', { name: 'Password', exact: true }).fill('test!');
    await page.getByRole('button', { name: 'Sign in' }).click();
  });

  test('edit admin team', async () => {
    await page.getByRole('link', { name: 'View your teams' }).click();
    await expect(page.getByRole('main')).toContainText('Default administrators team created automatically.');
    await page.getByRole('link', { name: 'View details' }).click();
    await page.getByRole('link', { name: 'Edit Team' }).click();
    await page.getByRole('textbox', { name: 'Description' }).click();
    await page.getByRole('textbox', { name: 'Description' }).press('ControlOrMeta+a');
    await page.getByRole('textbox', { name: 'Description' }).fill('Application administrators');
    await page.getByRole('button', { name: 'Save' }).click();
    await expect(page.getByRole('main')).toContainText('Application administrators');
  });
});
