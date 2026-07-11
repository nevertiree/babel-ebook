import { chromium, test, expect } from "@playwright/test";
import { spawn, type ChildProcess } from "node:child_process";
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, resolve, join } from "node:path";
import { fileURLToPath } from "node:url";
import { homedir } from "node:os";
import { cleanupBrowserProcesses, forceKill, getFreePort, waitForCdp } from "./e2e-helpers";

const __dirname = fileURLToPath(new URL(".", import.meta.url));
const APP_PATH = resolve(__dirname, "../../target/release/babel-ebook-desktop.exe");
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
          api_key: "",
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
      checkpoint_dir: "",
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

  const port = await getFreePort();
  cdpUrl = `http://localhost:${port}`;

  appProcess = spawn(APP_PATH, [], {
    env: {
      ...process.env,
      BABEL_EBOOK_E2E_CDP_PORT: String(port),
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

test("Ollama model dropdown is populated from local API", async () => {
  test.setTimeout(60000);
  const browser = await chromium.connectOverCDP(cdpUrl);
  const context = browser.contexts()[0];
  const page = context.pages()[0];
  page.on("console", (msg) => {
    console.log(`[browser console] ${msg.type()}: ${msg.text()}`);
  });

  const modelSelect = page.locator('label:has-text("Model") select');
  await expect(modelSelect).toBeVisible({ timeout: 10000 });
  const options = modelSelect.locator("option");
  await expect(options).not.toHaveCount(0, { timeout: 10000 });
  const optionCount = await options.count();
  expect(optionCount).toBeGreaterThan(1);

  await browser.close();
});
