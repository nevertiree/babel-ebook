import { chromium, test, expect } from "@playwright/test";
import { spawn, type ChildProcess } from "node:child_process";
import { mkdirSync, rmSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { cleanupBrowserProcesses, clearAppData, forceKill, getFreePort, waitForCdp } from "./e2e-helpers";

const __dirname = fileURLToPath(new URL(".", import.meta.url));
const APP_PATH = resolve(__dirname, "../../target/release/babel-ebook-desktop.exe");
const TEST_OUTPUT = resolve(__dirname, "../../output/e2e_compute_focus.epub");

let appProcess: ChildProcess | null = null;
let cdpUrl: string;

test.beforeAll(async () => {
  await cleanupBrowserProcesses();
  clearAppData();

  mkdirSync(dirname(TEST_OUTPUT), { recursive: true });
  rmSync(TEST_OUTPUT, { force: true });

  const port = await getFreePort();
  cdpUrl = `http://localhost:${port}`;

  appProcess = spawn(APP_PATH, [], {
    env: {
      ...process.env,
      BABEL_EBOOK_E2E_CDP_PORT: String(port),
      BABEL_EBOOK_E2E_OUTPUT: TEST_OUTPUT,
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

test("provider config name input keeps focus while typing", async () => {
  test.setTimeout(60000);
  const browser = await chromium.connectOverCDP(cdpUrl);
  const context = browser.contexts()[0];
  const page = context.pages()[0];
  page.on("console", (msg) => {
    console.log(`[browser console] ${msg.type()}: ${msg.text()}`);
  });

  // Navigate to settings and then the Providers tab.
  await page.getByTestId("nav-settings").click();
  const providersTab = page.locator('.settings-tab:has-text("Providers")');
  await expect(providersTab).toBeVisible({ timeout: 10000 });
  await providersTab.click();

  // Find the config name input.
  const nameInput = page.locator('.provider-name-label input').first();
  await expect(nameInput).toBeVisible();

  // Focus, type, and verify focus remains.
  await nameInput.click();
  await nameInput.fill('TestName');
  await expect(nameInput).toBeFocused();
  await expect(nameInput).toHaveValue('TestName');

  await browser.close();
});
