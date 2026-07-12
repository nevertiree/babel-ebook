import { chromium, test, expect } from "@playwright/test";
import { spawn, type ChildProcess } from "node:child_process";
import { mkdirSync, rmSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { cleanupBrowserProcesses, clearAppData, forceKill, getFreePort, waitForCdp } from "./e2e-helpers";

const __dirname = fileURLToPath(new URL(".", import.meta.url));
const APP_PATH = resolve(__dirname, "../../target/release/babel-ebook-desktop.exe");
const TEST_SOURCE = resolve(__dirname, "../../tests/fixtures/corrupted.epub");
const TEST_OUTPUT = resolve(__dirname, "../../output/e2e_validation_output.epub");

let appProcess: ChildProcess | null = null;
let cdpUrl: string;

test.beforeAll(async () => {
  const apiKey = process.env.BABEL_EBOOK_E2E_API_KEY;
  if (!apiKey) {
    throw new Error("BABEL_EBOOK_E2E_API_KEY is required");
  }

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

test("disables start when required fields are missing and surfaces translation errors", async () => {
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
  const dryRunButton = page.getByTestId("dry-run-button");
  await expect(startButton).toBeEnabled();
  await expect(dryRunButton).toBeEnabled();

  // Clear the source path and verify the buttons become disabled.
  const clearSourceButton = page
    .locator(".file-row-source")
    .getByRole("button", { name: "Clear" });
  await clearSourceButton.click();
  await expect(page.getByTestId("source-path")).not.toContainText(TEST_SOURCE);
  await expect(startButton).toBeDisabled();
  await expect(dryRunButton).toBeDisabled();

  // Restore the source path by reloading the app (env injection will re-apply it).
  await page.reload();
  await expect(page.getByTestId("source-path")).toContainText(TEST_SOURCE, { timeout: 10000 });
  await expect(startButton).toBeEnabled();

  // Start a translation with a corrupted input file; it should fail visibly.
  await startButton.click();

  // Navigate to the queue page and wait for the task to fail.
  await page.getByTestId("nav-tasks").click();
  await expect(page.getByTestId("task-list")).toBeVisible({ timeout: 10000 });
  const task = page.getByTestId("task-item").first();
  await expect(task).toContainText("failed", { timeout: 120000, ignoreCase: true });

  // The error details button should be available for the failed task.
  await expect(task.getByTestId("view-error-details")).toBeVisible();

  await page.screenshot({ path: "output/e2e_validation_error.png" });

  await browser.close();
});
