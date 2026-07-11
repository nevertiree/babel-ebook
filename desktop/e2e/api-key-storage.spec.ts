import { chromium, test, expect } from "@playwright/test";
import { spawn, type ChildProcess } from "node:child_process";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
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

function seedSettingsWithPlaintextKey() {
  mkdirSync(SETTINGS_DIR, { recursive: true });
  const payload = {
    version: 5,
    translation: {
      providers: [
        {
          name: "deepseek",
          provider: "deepseek",
          api_key: "sk-plaintext-secret",
          base_url: "",
          use_custom_base_url: false,
        },
      ],
      active_provider: "deepseek",
      model: "deepseek-chat",
      source_lang: "en",
      target_lang: "zh-CN",
      output_mode: "bilingual",
      style: "default",
      concurrency: 3,
      max_input_tokens: 4000,
      max_output_tokens: 2000,
      temperature: 0.3,
      dry_run: true,
      refine: false,
      checkpoint_dir: "",
      resume: "",
      translate_body: true,
      translate_metadata: true,
      translate_toc: true,
      translate_alt_text: true,
      translate_image_captions: true,
      translate_tables: true,
      translate_footnotes: true,
      translate_code: false,
      output_font: "",
      output_filename_template: "{stem}_{target_lang}",
      system_prompt: "",
      prompts: {
        default: "",
        literary: "",
        technical: "",
        academic: "",
        refine: "",
      },
    },
    general: {
      ui_language: "en",
      theme: "dark",
      follow_system_language: false,
    },
  };
  writeFileSync(SETTINGS_PATH, JSON.stringify(payload, null, 2));
}

function getProviderApiKey(name: string): string | undefined {
  const text = readFileSync(SETTINGS_PATH, "utf-8");
  const parsed = JSON.parse(text) as {
    translation?: { providers?: Array<{ name?: string; api_key?: string }> };
  };
  return parsed.translation?.providers?.find((p) => p.name === name)?.api_key;
}

test.beforeAll(async () => {
  await cleanupBrowserProcesses();
  seedSettingsWithPlaintextKey();

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

test("keeps API keys in plaintext settings.json and reloads them on restart", async () => {
  test.setTimeout(60000);
  const browser = await chromium.connectOverCDP(cdpUrl);
  const context = browser.contexts()[0];
  const page = context.pages()[0];
  page.on("console", (msg) => {
    console.log(`[browser console] ${msg.type()}: ${msg.text()}`);
  });

  // The seeded plaintext key should be preserved on load.
  await expect(page.getByTestId("nav-translate")).toBeVisible({ timeout: 10000 });
  expect(getProviderApiKey("deepseek")).toBe("sk-plaintext-secret");

  // Open the Compute settings page and enter a new API key.
  await page.getByRole("button", { name: "Compute" }).click();
  const keyInput = page.locator(".compute-settings-page .provider-api-key input").first();
  await expect(keyInput).toBeVisible();
  await keyInput.fill("sk-ui-secret-key");

  // Wait for the debounced autosave (500 ms) to finish.
  await page.waitForTimeout(1000);

  // settings.json should now contain the updated API key in plaintext.
  expect(getProviderApiKey("deepseek")).toBe("sk-ui-secret-key");

  await browser.close();
});
