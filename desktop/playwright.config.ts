import { defineConfig, devices } from "@playwright/test";

/**
 * Playwright configuration for Tauri desktop end-to-end tests.
 *
 * Tests connect to the already-built application via WebView2's Chrome
 * DevTools Protocol (CDP). Launch the app with:
 *
 *   WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222
 *
 * before connecting Playwright.
 */
export default defineConfig({
  testDir: "./e2e",
  fullyParallel: false,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: 1,
  reporter: "list",
  use: {
    trace: "on-first-retry",
  },
  projects: [
    {
      name: "tauri-desktop",
    },
  ],
});
