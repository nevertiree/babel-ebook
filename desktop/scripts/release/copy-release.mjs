#!/usr/bin/env node
/**
 * Copies the latest Windows installer artifacts from the Cargo/Tauri output
 * directory into the workspace `release/v<version>/` folder.
 *
 * The version is read from the `RELEASE_VERSION` environment variable (set by
 * `release.mjs`). When run standalone after `pnpm tauri build`, you can provide
 * it explicitly:
 *
 *   RELEASE_VERSION=0.2.0 node scripts/copy-release.mjs
 */
import { copyFile, mkdir, readdir, rm } from "node:fs/promises";
import { join, resolve } from "node:path";

const version = process.env.RELEASE_VERSION;
if (!version) {
  console.error(
    "RELEASE_VERSION environment variable is required. " +
      "Run this script via `pnpm release:build` or set it manually."
  );
  process.exit(1);
}

const desktopRoot = resolve(import.meta.dirname, "../..");
const workspaceRoot = resolve(desktopRoot, "..");
const bundleDir = join(workspaceRoot, "target", "release", "bundle");
const releaseDir = join(workspaceRoot, "release", `v${version}`);

const sources = [join(bundleDir, "msi"), join(bundleDir, "nsis")];

async function main() {
  // Remove any stale artifacts from a previous run of the same version.
  await rm(releaseDir, { recursive: true, force: true });
  await mkdir(releaseDir, { recursive: true });

  const expectedPrefix = `BabelEbook_${version}_`;
  let copied = 0;
  for (const dir of sources) {
    const entries = await readdir(dir).catch(() => []);
    for (const name of entries) {
      if (name.startsWith(expectedPrefix)) {
        const src = join(dir, name);
        const dst = join(releaseDir, name);
        await copyFile(src, dst);
        console.log(`Copied: ${src} -> ${dst}`);
        copied += 1;
      }
    }
  }

  if (copied === 0) {
    console.error(
      `No BabelEbook_${version}_ installer artifacts found in target/release/bundle/`
    );
    process.exit(1);
  }

  console.log(`\nAll ${copied} installer artifact(s) copied to ${releaseDir}`);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
