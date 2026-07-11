import { chromium, test, expect } from "@playwright/test";
import { spawn, type ChildProcess } from "node:child_process";
import { mkdirSync, rmSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { cleanupBrowserProcesses, forceKill, getFreePort, waitForCdp } from "./e2e-helpers";

const __dirname = fileURLToPath(new URL(".", import.meta.url));
const APP_PATH = resolve(__dirname, "../../target/release/babel-ebook-desktop.exe");
const TEST_SOURCE = resolve(__dirname, "../../tests/fixtures/corrupted.epub");
const TEST_OUTPUT = resolve(__dirname, "../../output/e2e_corrupted_output.epub");

let appProcess: ChildProcess | null = null;
let cdpUrl: string;

test.beforeAll(async () => {
  const apiKey = process.env.BABEL_EBOOK_E2E_API_KEY;
  if (!apiKey) {
    throw new Error("BABEL_EBOOK_E2E_API_KEY is required");
  }

  await cleanupBrowserProcesses();

  mkdirSync(dirname(TEST_OUTPUT), { recursive: true });
  rmSync(TEST_OUTPUT, { force: true });

  const port = await getFreePort();
  cdpUrl = `http://localhost:${port}`;

  appProcess = spawn(APP_PATH, [], {
    env: {
      ...process.env,
      BABEL_EBOOK_E2E_CDP_PORT: String(port),
      BABEL_EBOOK_E2E_SOURCE: TEST_SOURCE,
      BABEL_EBOOK_E2E_OUTPUT: TEST_OUTPUT,
      BABEL_EBOOK_E2E_API_KEY: apiKey,
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

test("failed task can be retried and does not crash the app", async () => {
  test.setTimeout(180000);
  const browser = await chromium.connectOverCDP(cdpUrl);
  const context = browser.contexts()[0];
  const page = context.pages()[0];
  page.on("console", (msg) => {
    console.log(`[browser console] ${msg.type()}: ${msg.text()}`);
  });

  await expect(page.getByTestId("source-path")).toContainText(TEST_SOURCE, {
    timeout: 10000,
  });

  const startButton = page.getByTestId("start-button");
  await expect(startButton).toBeEnabled();
  await startButton.click();

  // Navigate to the queue page to inspect the failed task.
  await page.getByTestId("nav-tasks").click();
  await expect(page.getByTestId("task-list")).toBeVisible({ timeout: 10000 });
  const firstTask = page.getByTestId("task-item").first();
  await expect(firstTask).toContainText(/failed|error|invalid|corrupt/i, {
    timeout: 120000,
  });

  await page.screenshot({ path: "output/e2e_corrupted_task_failed.png" });

  // Retry should be offered for a failed task.
  const retryButton = page.getByTestId("retry-task");
  await expect(retryButton).toBeVisible();
  await retryButton.click();

  // After retry, the task should return to pending/running and then fail again.
  await expect(firstTask).toContainText(/pending|running|failed/i, {
    timeout: 120000,
  });

  await page.screenshot({ path: "output/e2e_corrupted_task_retried.png" });

  await browser.close();
});
