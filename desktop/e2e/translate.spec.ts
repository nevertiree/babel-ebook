import { chromium, test, expect } from "@playwright/test";
import { spawn, type ChildProcess } from "node:child_process";
import { mkdirSync } from "node:fs";
import { dirname, resolve } from "node:path";
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
  const apiKey = process.env.BABEL_EBOOK_E2E_API_KEY;
  if (!apiKey) {
    throw new Error("BABEL_EBOOK_E2E_API_KEY is required");
  }

  mkdirSync(dirname(TEST_OUTPUT), { recursive: true });

  appProcess = spawn(APP_PATH, [], {
    env: {
      ...process.env,
      BABEL_EBOOK_E2E_CDP_PORT: "9222",
      BABEL_EBOOK_E2E_SOURCE: TEST_SOURCE,
      BABEL_EBOOK_E2E_OUTPUT: TEST_OUTPUT,
      BABEL_EBOOK_E2E_API_KEY: apiKey,
      BABEL_EBOOK_E2E_DRY_RUN: "true",
      BABEL_EBOOK_E2E_UI_LANGUAGE: "en",
    },
    detached: false,
    shell: true,
    stdio: ["ignore", "pipe", "pipe"],
  });
  appProcess.stdout?.on("data", (data) => {
    console.log(`[app stdout] ${data.toString().trim()}`);
  });
  appProcess.stderr?.on("data", (data) => {
    console.error(`[app stderr] ${data.toString().trim()}`);
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

test("translates a small EPUB in dry-run mode via the desktop UI", async () => {
  test.setTimeout(180000);
  const browser = await chromium.connectOverCDP(CDP_URL);
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

  await startButton.click();

  // Wait for the dry-run pipeline to parse the EPUB and emit the token estimate.
  const progressSection = page.getByTestId("progress-section");
  await expect(progressSection).toBeVisible({ timeout: 120000 });

  const progressMessage = page.getByTestId("progress-message");
  await expect(progressMessage).toContainText(/token|estimated|completed/i, {
    timeout: 120000,
  });

  await page.screenshot({ path: "output/e2e_translate_dryrun.png" });

  await browser.close();
});
