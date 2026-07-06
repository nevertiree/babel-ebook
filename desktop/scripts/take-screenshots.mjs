import { chromium } from "@playwright/test";
import { spawn } from "node:child_process";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { mkdirSync } from "node:fs";

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(__dirname, "../..");
const APP_PATH = resolve(ROOT, "target/release/babel-ebook-desktop.exe");
const CDP_PORT = "9223";
const CDP_URL = `http://localhost:${CDP_PORT}`;
const SOURCE = resolve(ROOT, "tests/fixtures/sample.epub");
const OUTPUT = resolve(ROOT, "output/screenshot_output.epub");
const SHOT_DIR = resolve(ROOT, "docs/assets/screenshots");

mkdirSync(SHOT_DIR, { recursive: true });

async function waitForCdp(retries = 30) {
  for (let i = 0; i < retries; i += 1) {
    try {
      const res = await fetch(`${CDP_URL}/json/version`);
      if (res.ok) return true;
    } catch {
      // not ready
    }
    await new Promise((r) => setTimeout(r, 1000));
  }
  return false;
}

function takeShot(page, name) {
  return page.screenshot({
    path: resolve(SHOT_DIR, name),
    fullPage: false,
  });
}

const app = spawn(APP_PATH, [], {
  env: {
    ...process.env,
    WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${CDP_PORT} --remote-allow-origins=*`,
    BABEL_EBOOK_E2E_CDP_PORT: CDP_PORT,
    BABEL_EBOOK_E2E_SOURCE: SOURCE,
    BABEL_EBOOK_E2E_OUTPUT: OUTPUT,
    BABEL_EBOOK_E2E_API_KEY: "sk-dummy-screenshot-key",
    BABEL_EBOOK_E2E_DRY_RUN: "true",
    BABEL_EBOOK_E2E_UI_LANGUAGE: "zh-CN",
  },
  detached: false,
  shell: true,
});

try {
  const ready = await waitForCdp();
  if (!ready) {
    throw new Error("Tauri app did not expose CDP port in time");
  }

  const browser = await chromium.connectOverCDP(CDP_URL);
  const context = browser.contexts()[0];
  const page = context.pages()[0];

  // 1. Main translate page
  await page.waitForSelector('[data-testid="source-path"]', { timeout: 10000 });
  await takeShot(page, "01-translate.png");

  // 2. Settings -> Compute
  await page.getByText("算力", { exact: true }).click();
  await page.waitForTimeout(500);
  await takeShot(page, "02-settings-compute.png");

  // 3. Settings -> Translation options
  await page.getByText("翻译选项", { exact: true }).click();
  await page.waitForTimeout(500);
  await takeShot(page, "03-settings-translation.png");

  // 4. Logs page
  await page.getByText("日志", { exact: true }).click();
  await page.waitForTimeout(500);
  await takeShot(page, "04-logs.png");

  // 5. About page
  await page.getByText("关于", { exact: true }).click();
  await page.waitForTimeout(500);
  await takeShot(page, "05-about.png");

  // 6. Start a dry-run and capture progress + logs
  await page.getByText("翻译", { exact: true }).click();
  await page.getByTestId("start-button").click();
  await page.waitForSelector('[data-testid="progress-message"]', {
    timeout: 120000,
  });
  await page.waitForTimeout(1000);
  await takeShot(page, "06-translate-progress.png");

  await page.getByText("日志", { exact: true }).click();
  await page.waitForTimeout(500);
  await takeShot(page, "07-logs-progress.png");

  await browser.close();
  console.log(`Screenshots saved to ${SHOT_DIR}`);
} finally {
  if (app && !app.killed) {
    app.kill();
  }
}
