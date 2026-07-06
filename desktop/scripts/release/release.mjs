#!/usr/bin/env node
/**
 * Full release build for BabelEbook desktop.
 *
 * Preconditions:
 *   - Working tree is clean.
 *   - HEAD points to an annotated/lightweight tag.
 *   - The tag name matches the workspace version in Cargo.toml.
 *
 * It then runs lint/tests, builds the Tauri app, and copies installers into
 * release/v<version>/.
 *
 * Usage:
 *   pnpm release:build
 */
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { spawnSync } from "node:child_process";

const desktopRoot = resolve(import.meta.dirname, "../..");
const workspaceRoot = resolve(desktopRoot, "..");
const cargoPath = resolve(workspaceRoot, "Cargo.toml");

function run(commandLine, cwd = workspaceRoot, env = {}) {
  console.log(`\n$ ${commandLine}`);
  const result = spawnSync(commandLine, {
    cwd,
    stdio: "inherit",
    env: { ...process.env, ...env },
    shell: true,
  });
  if (result.status !== 0) {
    throw new Error(`Command failed: ${commandLine}`);
  }
}

function runSilent(command, args, cwd = workspaceRoot) {
  const result = spawnSync(command, args, {
    cwd,
    stdio: ["ignore", "pipe", "pipe"],
    encoding: "utf-8",
  });
  if (result.status !== 0) {
    throw new Error(`Command failed: ${command} ${args.join(" ")}`);
  }
  return result.stdout.trim();
}

async function readCargoVersion() {
  const content = await readFile(cargoPath, "utf-8");
  const match = content.match(/^\s*version\s*=\s*"([^"]+)"/m);
  if (!match) {
    throw new Error(`Could not find version in ${cargoPath}`);
  }
  return match[1];
}

async function main() {
  // 1. Clean working tree.
  const status = runSilent("git", ["status", "--porcelain"]);
  if (status !== "") {
    console.error("Working tree is not clean:");
    console.error(status);
    process.exit(1);
  }

  // 2. HEAD must be on a tag.
  let tag;
  try {
    tag = runSilent("git", ["describe", "--tags", "--exact-match"]);
  } catch (err) {
    console.error("HEAD does not point to a tag. Run `pnpm version:bump` first.");
    process.exit(1);
  }

  // 3. Tag must match Cargo.toml version.
  const version = await readCargoVersion();
  const expectedTag = `v${version}`;
  if (tag !== expectedTag) {
    console.error(`Tag mismatch: tag is ${tag} but Cargo.toml version is ${version}`);
    process.exit(1);
  }

  console.log(`\nBuilding release ${tag}...`);

  // 4. Quality gates.
  run("cargo fmt -- --check");
  run("cargo clippy --workspace --all-targets -- -D warnings");
  run("cargo test --workspace");

  // 5. Frontend checks.
  run("pnpm exec tsc --noEmit", desktopRoot);
  run("pnpm build", desktopRoot);

  // 6. Tauri build.
  run("pnpm tauri build", desktopRoot);

  // 7. Copy artifacts into versioned release directory.
  run("node scripts/release/copy-release.mjs", desktopRoot, { RELEASE_VERSION: version });

  console.log(`\nRelease ${tag} built successfully.`);
  console.log(`Artifacts are in: ${resolve(workspaceRoot, "release", `v${version}`)}`);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
