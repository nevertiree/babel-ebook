import { chromium, test, expect } from "@playwright/test";
import { spawn, type ChildProcess } from "node:child_process";
import { mkdirSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { cleanupBrowserProcesses, forceKill, getFreePort, waitForCdp } from "./e2e-helpers";

const __dirname = fileURLToPath(new URL(".", import.meta.url));
const APP_PATH = resolve(__dirname, "../../target/release/babel-ebook-desktop.exe");
const TEST_SOURCE = resolve(__dirname, "../../tests/fixtures/sample.epub");
const TEST_OUTPUT = resolve(__dirname, "../../output/e2e_output.epub");
const TEST_CHECKPOINT_DIR = resolve(__dirname, "../../output/e2e_checkpoints");

let appProcess: ChildProcess | null = null;
let cdpUrl: string;

test.beforeAll(async () => {
  await cleanupBrowserProcesses();

  mkdirSync(dirname(TEST_OUTPUT), { recursive: true });
  mkdirSync(TEST_CHECKPOINT_DIR, { recursive: true });

  const port = await getFreePort();
  cdpUrl = `http://localhost:${port}`;

  appProcess = spawn(APP_PATH, [], {
    env: {
      ...process.env,
      BABEL_EBOOK_E2E_CDP_PORT: String(port),
      BABEL_EBOOK_E2E_SOURCE: TEST_SOURCE,
      BABEL_EBOOK_E2E_OUTPUT: TEST_OUTPUT,
      BABEL_EBOOK_E2E_CHECKPOINT_DIR: TEST_CHECKPOINT_DIR,
      BABEL_EBOOK_E2E_DRY_RUN: "true",
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

test("refine checkbox toggles the form state", async () => {
  const browser = await chromium.connectOverCDP(cdpUrl);
  const context = browser.contexts()[0];
  const page = context.pages()[0];
  page.on("console", (msg) => {
    console.log(`[browser console] ${msg.type()}: ${msg.text()}`);
  });

  const refineCheckbox = page.getByTestId("refine-checkbox");
  await expect(refineCheckbox).not.toBeChecked();

  await refineCheckbox.click();
  await expect(refineCheckbox).toBeChecked();

  await refineCheckbox.click();
  await expect(refineCheckbox).not.toBeChecked();

  await browser.close();
});

test("checkpoint list is shown when checkpoint directory is configured", async () => {
  const browser = await chromium.connectOverCDP(cdpUrl);
  const context = browser.contexts()[0];
  const page = context.pages()[0];
  page.on("console", (msg) => {
    console.log(`[browser console] ${msg.type()}: ${msg.text()}`);
  });

  const list = page.getByTestId("checkpoint-list");
  await expect(list).toBeVisible();

  await browser.close();
});
