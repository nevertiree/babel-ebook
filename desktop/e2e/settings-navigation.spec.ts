import { chromium, test, expect } from "@playwright/test";
import { spawn, type ChildProcess } from "node:child_process";
import { cleanupBrowserProcesses, clearAppData, forceKill, getFreePort, waitForCdp } from "./e2e-helpers";

const __dirname = fileURLToPath(new URL(".", import.meta.url));
const APP_PATH = resolve(__dirname, "../../target/release/babel-ebook-desktop.exe");

import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

let appProcess: ChildProcess | null = null;
let cdpUrl: string;

test.beforeAll(async () => {
  await cleanupBrowserProcesses();
  clearAppData();

  const port = await getFreePort();
  cdpUrl = `http://localhost:${port}`;

  appProcess = spawn(APP_PATH, [], {
    env: {
      ...process.env,
      BABEL_EBOOK_E2E_CDP_PORT: String(port),
      BABEL_EBOOK_E2E_UI_LANGUAGE: "en",
    },
    stdio: ["ignore", "pipe", "pipe"],
  });
  appProcess.stdout?.on("data", (data) => {
    console.log(`[app stdout] ${data.toString().trim()}`);
  });
  appProcess.stderr?.on("data", (data) => {
    console.error(`[app stderr] ${data.toString().trim()}`);
  });

  const ready = await waitForCdp(cdpUrl);
  if (!ready) {
    await forceKill(appProcess);
    throw new Error(`Tauri app did not expose CDP port ${port} in time`);
  }
});

test.afterAll(async () => {
  await forceKill(appProcess);
});

test("navigates through all settings tabs and persists changes", async () => {
  test.setTimeout(120000);
  const browser = await chromium.connectOverCDP(cdpUrl);
  const context = browser.contexts()[0];
  const page = context.pages()[0];
  page.on("console", (msg) => {
    console.log(`[browser console] ${msg.type()}: ${msg.text()}`);
  });

  await expect(page.getByTestId("nav-translate")).toBeVisible({ timeout: 10000 });
  await page.getByTestId("nav-settings").click();

  // Each tab should render its own panel.
  const tabs = [
    { id: "compute", heading: "Providers" },
    { id: "model", heading: "Model" },
    { id: "translation", heading: "Translation Options" },
    { id: "prompts", heading: "Prompts" },
    { id: "output", heading: "Output & Files" },
    { id: "queue", heading: "Task Queue" },
    { id: "general", heading: "General" },
  ];

  for (const tab of tabs) {
    await page.getByTestId(`settings-tab-${tab.id}`).click();
    await expect(
      page.getByRole("heading", { name: tab.heading, exact: true })
    ).toBeVisible({ timeout: 10000 });
  }

  // Change max_input_tokens on the Model tab and wait for autosave debounce.
  await page.getByTestId("settings-tab-model").click();
  const maxInputTokens = page.locator('label:has-text("Max Input Tokens") input');
  await expect(maxInputTokens).toBeVisible();
  await maxInputTokens.fill("1234");
  await maxInputTokens.blur();
  await page.waitForTimeout(700);

  // Reload the webview and verify the persisted value.
  await page.reload();
  await expect(page.getByTestId("nav-translate")).toBeVisible({ timeout: 10000 });
  await page.getByTestId("nav-settings").click();
  await page.getByTestId("settings-tab-model").click();
  await expect(maxInputTokens).toHaveValue("1234");

  // Queue tab: invalid concurrency shows an inline error and clamps on blur.
  await page.getByTestId("settings-tab-queue").click();
  const concurrencyInput = page.locator('label:has-text("Concurrency") input');
  await expect(concurrencyInput).toBeVisible();
  await concurrencyInput.fill("0");
  await expect(page.locator("#error-concurrency")).toBeVisible();
  await concurrencyInput.blur();
  await expect(page.locator("#error-concurrency")).not.toBeVisible();
  await expect(concurrencyInput).toHaveValue("1");

  await browser.close();
});
