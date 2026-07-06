import { chromium, expect } from "@playwright/test";
import { spawn, spawnSync } from "node:child_process";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { existsSync, unlinkSync } from "node:fs";

const __dirname = dirname(fileURLToPath(import.meta.url));
const APP_PATH = resolve(__dirname, "../../target/release/babel-ebook-desktop.exe");
const CDP_URL = "http://localhost:9224";

const SOURCE = resolve(__dirname, "../../tests/fixtures/sample.epub");
const OUTPUT = resolve(__dirname, "../../output/gui_full_translate.epub");

const API_KEY = process.env.BABEL_EBOOK_E2E_API_KEY;
if (!API_KEY) {
  console.error("Missing BABEL_EBOOK_E2E_API_KEY");
  process.exit(1);
}

// Ensure no previous hung desktop instances hold onto WebView2 resources.
spawnSync("powershell", [
  "-Command",
  "Get-Process babel-ebook-desktop -ErrorAction SilentlyContinue | Stop-Process -Force",
]);

async function waitForCdp(retries = 60) {
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

async function main() {
  if (existsSync(OUTPUT)) {
    unlinkSync(OUTPUT);
    console.log(`Removed existing output: ${OUTPUT}`);
  }

  console.log("Starting BabelEbook desktop app...");
  const app = spawn(APP_PATH, [], {
    env: {
      ...process.env,
      BABEL_EBOOK_E2E_CDP_PORT: "9224",
      BABEL_EBOOK_E2E_SOURCE: SOURCE,
      BABEL_EBOOK_E2E_OUTPUT: OUTPUT,
      BABEL_EBOOK_E2E_API_KEY: API_KEY,
      BABEL_EBOOK_E2E_DRY_RUN: "false",
      BABEL_EBOOK_E2E_UI_LANGUAGE: "zh-CN",
    },
    shell: true,
  });

  app.on("exit", (code) => {
    console.log(`App exited with code ${code ?? "unknown"}`);
  });

  const ready = await waitForCdp();
  if (!ready) {
    console.error("App did not expose CDP port 9224 in time");
    app.kill();
    process.exit(1);
  }

  console.log("Connected to CDP, navigating UI...");
  const browser = await chromium.connectOverCDP(CDP_URL);
  const context = browser.contexts()[0];
  const page = context.pages()[0];

  await expect(page.getByTestId("source-path")).toContainText(SOURCE, { timeout: 10000 });
  await expect(page.getByTestId("output-path")).toContainText(OUTPUT);

  const startButton = page.getByTestId("start-button");
  await expect(startButton).toBeEnabled();

  console.log("Starting full translation...");
  await startButton.click();

  // Wait for the translation to finish (success log entry from the command result).
  const successLog = page
    .locator('[data-testid="log-panel"] .log-entry.success')
    .filter({ hasText: /Translation completed|翻译完成:/ })
    .last();
  await successLog.waitFor({ timeout: 7200000 });

  const progress = page.getByTestId("progress-message");
  const finalMessage = await progress.textContent();
  const finalLog = await successLog.textContent();
  console.log(`Final progress: ${finalMessage}`);
  console.log(`Final log: ${finalLog}`);

  await page.screenshot({ path: "output/gui_full_translate.png" });

  await browser.close();
  app.kill();
  console.log("GUI full translation test finished.");
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
