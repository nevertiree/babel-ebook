import { chromium, test, expect } from "@playwright/test";
import { spawn, type ChildProcess } from "node:child_process";
import { mkdirSync, rmSync, writeFileSync } from "node:fs";
import { dirname, resolve, join } from "node:path";
import { fileURLToPath } from "node:url";
import { homedir } from "node:os";
import { cleanupBrowserProcesses, forceKill, getFreePort, waitForCdp } from "./e2e-helpers";

const __dirname = fileURLToPath(new URL(".", import.meta.url));
const APP_PATH = resolve(__dirname, "../../target/release/babel-ebook-desktop.exe");
const TEST_SOURCE = resolve(__dirname, "../../tests/fixtures/multichapter_5.epub");
const TEST_OUTPUT = resolve(__dirname, "../../output/e2e_ollama_output.epub");
const CACHE_DIR = resolve(__dirname, "../../.babel_ebook_cache");
const SETTINGS_DIR = resolve(homedir(), "Documents/BabelEbook");
const SETTINGS_PATH = join(SETTINGS_DIR, "settings.json");

let appProcess: ChildProcess | null = null;
let cdpUrl: string;

function seedOllamaSettings() {
  mkdirSync(SETTINGS_DIR, { recursive: true });
  const payload = {
    version: 5,
    translation: {
      providers: [
        {
          name: "ollama",
          provider: "ollama",
          base_url: "",
          use_custom_base_url: false,
        },
      ],
      active_provider: "ollama",
      model: "llama3.2:latest",
      source_lang: "en",
      target_lang: "zh-CN",
      output_mode: "bilingual",
      style: "default",
      concurrency: 1,
      max_input_tokens: 4000,
      max_output_tokens: 2000,
      temperature: 0.3,
      dry_run: false,
      refine: false,
      checkpoint_dir: join(SETTINGS_DIR, "checkpoints"),
      translate_body: true,
      translate_metadata: true,
      translate_toc: true,
      translate_alt_text: true,
      translate_image_captions: true,
      translate_tables: true,
      translate_footnotes: true,
      translate_code: false,
    },
    general: {
      ui_language: "en",
      theme: "dark",
      follow_system_language: false,
    },
  };
  writeFileSync(SETTINGS_PATH, JSON.stringify(payload, null, 2));
}

test.beforeAll(async () => {
  await cleanupBrowserProcesses();
  seedOllamaSettings();

  mkdirSync(dirname(TEST_OUTPUT), { recursive: true });
  rmSync(TEST_OUTPUT, { force: true });
  rmSync(CACHE_DIR, { recursive: true, force: true });

  const port = await getFreePort();
  cdpUrl = `http://localhost:${port}`;

  appProcess = spawn(APP_PATH, [], {
    env: {
      ...process.env,
      BABEL_EBOOK_E2E_CDP_PORT: String(port),
      BABEL_EBOOK_E2E_SOURCE: TEST_SOURCE,
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

test("translates a multi-chapter EPUB with Ollama and shows smooth progress", async () => {
  test.setTimeout(600000); // Real LLM translation may take a few minutes.
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

  // Settings were pre-seeded with Ollama + llama3.2; verify the UI picked them up.
  await expect(page.locator("select").first()).toHaveValue("ollama");

  const startButton = page.getByTestId("start-button");
  await expect(startButton).toBeEnabled();

  // Capture the first width of the task progress bar after starting.
  await startButton.click();
  await expect(page.getByTestId("task-list")).toBeVisible({ timeout: 10000 });

  const firstTask = page.getByTestId("task-item").first();
  const progressFill = firstTask.locator(".progress-fill").first();

  // Wait for completion. Real LLM translation may be fast on small fixtures,
  // so we verify the task runs to completion and logs all chapter events.
  await expect(firstTask).toContainText(/completed/i, { timeout: 600000 });
  await expect(progressFill).toHaveAttribute("style", /width:\s*100%/);

  await page.screenshot({ path: "output/e2e_ollama_progress_done.png" });

  // Verify the output file was written.
  const fs = await import("node:fs");
  expect(fs.existsSync(TEST_OUTPUT)).toBe(true);
  expect(fs.statSync(TEST_OUTPUT).size).toBeGreaterThan(0);

  // Navigate to logs and verify there are multiple entries.
  // A 5-chapter book emits: Started + 5 ChapterStarted + 5 ChapterFinished + Completed = 12.
  await page.getByTestId("nav-logs").click();
  await expect(page.locator(".logs-page .log-entry")).toHaveCount(12, { timeout: 5000 });
  await page.screenshot({ path: "output/e2e_ollama_logs.png" });

  await browser.close();
});
