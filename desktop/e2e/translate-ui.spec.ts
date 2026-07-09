import { chromium, test, expect } from "@playwright/test";
import { spawn, type ChildProcess } from "node:child_process";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = fileURLToPath(new URL(".", import.meta.url));
const APP_PATH = resolve(__dirname, "../../target/release/babel-ebook-desktop.exe");
const CDP_URL = "http://localhost:9222";
const TEST_SOURCE = resolve(__dirname, "../../tests/fixtures/sample.epub");
const TEST_OUTPUT = resolve(__dirname, "../../output/e2e_output.epub");

async function waitForCdp(retries = 30): Promise<boolean> {
  for (let i = 0; i < retries; i += 1) {
    try {
      const res = await fetch(`${CDP_URL}/json/version`);
      if (res.ok) return true;
    } catch {
      // not ready yet
    }
    await new Promise((r) => setTimeout(r, 1000));
  }
  return false;
}

let appProcess: ChildProcess | null = null;

test.beforeAll(async () => {
  appProcess = spawn(APP_PATH, [], {
    env: {
      ...process.env,
      BABEL_EBOOK_E2E_CDP_PORT: "9222",
      BABEL_EBOOK_E2E_SOURCE: TEST_SOURCE,
      BABEL_EBOOK_E2E_OUTPUT: TEST_OUTPUT,
      BABEL_EBOOK_E2E_DRY_RUN: "true",
      BABEL_EBOOK_E2E_UI_LANGUAGE: "en",
    },
    detached: false,
    shell: true,
  });

  const ready = await waitForCdp();
  if (!ready) {
    appProcess.kill();
    throw new Error("Tauri app did not expose CDP port 9222 in time");
  }
});

test.afterAll(() => {
  if (appProcess && !appProcess.killed) {
    appProcess.kill();
  }
});

test("refine checkbox toggles the form state", async () => {
  const browser = await chromium.connectOverCDP(CDP_URL);
  const context = browser.contexts()[0];
  const page = context.pages()[0];

  const refineCheckbox = page.getByTestId("refine-checkbox");
  await expect(refineCheckbox).not.toBeChecked();

  await refineCheckbox.click();
  await expect(refineCheckbox).toBeChecked();

  await refineCheckbox.click();
  await expect(refineCheckbox).not.toBeChecked();

  await browser.close();
});

test("checkpoint toggle expands and collapses the checkpoint list", async () => {
  const browser = await chromium.connectOverCDP(CDP_URL);
  const context = browser.contexts()[0];
  const page = context.pages()[0];

  const toggle = page.getByTestId("toggle-checkpoints");
  await expect(toggle).toBeVisible();
  await expect(toggle).toBeEnabled();

  await toggle.click();
  const list = page.getByTestId("checkpoint-list");
  await expect(list).toBeVisible();

  await toggle.click();
  await expect(list).not.toBeVisible();

  await browser.close();
});
