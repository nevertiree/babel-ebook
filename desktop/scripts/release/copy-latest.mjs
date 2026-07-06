#!/usr/bin/env node
/**
 * Copies the current versioned release artifacts into `release/latest/`,
 * renaming the version segment to `latest`.
 *
 * Example:
 *   release/v0.2.1/BabelEbook_0.2.1_x64-setup.exe
 *     -> release/latest/BabelEbook_latest_x64-setup.exe
 *
 * Usage:
 *   RELEASE_VERSION=0.2.1 node scripts/copy-latest.mjs
 */
import { copyFile, mkdir, readdir, rm } from "node:fs/promises";
import { join, resolve } from "node:path";

const version = process.env.RELEASE_VERSION;
if (!version) {
  console.error("RELEASE_VERSION environment variable is required.");
  process.exit(1);
}

const workspaceRoot = resolve(import.meta.dirname, "..", "..", "..");
const sourceDir = join(workspaceRoot, "release", `v${version}`);
const latestDir = join(workspaceRoot, "release", "latest");

async function main() {
  await rm(latestDir, { recursive: true, force: true });
  await mkdir(latestDir, { recursive: true });

  const versionedPrefix = `BabelEbook_${version}_`;
  let copied = 0;

  const entries = await readdir(sourceDir).catch(() => []);
  for (const name of entries) {
    if (!name.startsWith(versionedPrefix)) {
      continue;
    }
    const latestName = name.replace(versionedPrefix, "BabelEbook_latest_");
    const src = join(sourceDir, name);
    const dst = join(latestDir, latestName);
    await copyFile(src, dst);
    console.log(`Copied: ${src} -> ${dst}`);
    copied += 1;
  }

  if (copied === 0) {
    console.error(`No BabelEbook_${version}_ artifacts found in ${sourceDir}`);
    process.exit(1);
  }

  console.log(`\nAll ${copied} latest artifact(s) copied to ${latestDir}`);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
