import { chromium, test, expect } from "@playwright/test";
import { spawn, spawnSync, type ChildProcess } from "node:child_process";
import { copyFileSync, mkdirSync, rmSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { cleanupBrowserProcesses, clearAppData, forceKill, getFreePort, waitForCdp } from "./e2e-helpers";

const __dirname = fileURLToPath(new URL(".", import.meta.url));
const APP_PATH = resolve(__dirname, "../../target/release/babel-ebook-desktop.exe");
const FIXTURE_SCRIPT = resolve(__dirname, "../../tests/fixtures/generate_multichapter_epub.py");
const TEST_SOURCE = resolve(__dirname, "../../tests/fixtures/e2e_tasks_controls.epub");
const TEST_OUTPUT = resolve(__dirname, "../../output/e2e_tasks_output.epub");

let appProcess: ChildProcess | null = null;
let cdpUrl: string;

function generateFixture() {
  mkdirSync(dirname(TEST_SOURCE), { recursive: true });
  rmSync(TEST_SOURCE, { force: true });
  const result = spawnSync("python", [FIXTURE_SCRIPT, "5"], {
    cwd: resolve(__dirname, "../../tests/fixtures"),
    encoding: "utf-8",
  });
  if (result.status !== 0) {
    console.error(`fixture generator exited with code ${result.status}: ${result.stderr}`);
    throw new Error("failed to generate test fixture");
  }
  const generated = resolve(__dirname, "../../tests/fixtures/multichapter_5.epub");
  copyFileSync(generated, TEST_SOURCE);
}

test.beforeAll(async () => {
  const apiKey = process.env.BABEL_EBOOK_E2E_API_KEY;
  if (!apiKey) {
    throw new Error("BABEL_EBOOK_E2E_API_KEY is required");
  }

  generateFixture();

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
      BABEL_EBOOK_E2E_SOURCE: TEST_SOURCE,
      BABEL_EBOOK_E2E_OUTPUT: TEST_OUTPUT,
      BABEL_EBOOK_E2E_API_KEY: apiKey,
      BABEL_EBOOK_E2E_DRY_RUN: "true",
      BABEL_EBOOK_E2E_SLOW_DRY_RUN_MS: "5000",
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

test("exercises per-task pause, resume and cancel controls", async () => {
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
  await expect(page.getByTestId("output-path")).toContainText(TEST_OUTPUT);

  const startButton = page.getByTestId("start-button");
  await expect(startButton).toBeEnabled();

  // Start the first task and wait for it to be running.
  await startButton.click();
  await expect(page.getByTestId("running-panel")).toBeVisible({ timeout: 10000 });

  // Enqueue a second task while the first one is running; it stays pending.
  await startButton.click();

  // Navigate to the queue page.
  await page.getByTestId("nav-tasks").click();
  await expect(page.getByTestId("task-list")).toBeVisible({ timeout: 10000 });

  const firstTask = page.getByTestId("task-item").first();
  const secondTask = page.getByTestId("task-item").nth(1);
  await expect(secondTask).toBeVisible();

  // Pause the whole queue; the running task becomes paused.
  const pauseButton = page.getByTestId("pause-queue");
  await pauseButton.click();
  const resumeTaskButton = firstTask.getByTestId("resume-task");
  await expect(resumeTaskButton).toBeVisible({ timeout: 30000 });

  // Resume the paused task; it returns to pending while the queue is paused.
  await resumeTaskButton.click();
  const cancelTaskButton = firstTask.getByTestId("cancel-task");
  await expect(cancelTaskButton).toBeVisible({ timeout: 10000 });

  // Cancel the second (pending) task.
  await secondTask.getByTestId("cancel-task").click();
  const retryTaskButton = secondTask.getByTestId("retry-task");
  await expect(retryTaskButton).toBeVisible({ timeout: 10000 });

  // Restart the queue; the first task should run to completion.
  const startQueueButton = page.getByTestId("start-queue");
  await startQueueButton.click();
  await expect(pauseButton).toBeVisible({ timeout: 10000 });

  await expect(firstTask).toContainText("completed", {
    timeout: 120000,
    ignoreCase: true,
  });

  await page.screenshot({ path: "output/e2e_tasks_controls.png" });

  await browser.close();
});
