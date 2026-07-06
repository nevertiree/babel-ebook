#!/usr/bin/env node
/**
 * Bump the workspace version across Cargo.toml, desktop/package.json and
 * desktop/src-tauri/tauri.conf.json, then commit and create an annotated git tag.
 *
 * Usage:
 *   pnpm version:bump patch
 *   pnpm version:bump minor
 *   pnpm version:bump major
 */
import { readFile, writeFile } from "node:fs/promises";
import { resolve } from "node:path";
import { execSync } from "node:child_process";

const desktopRoot = resolve(import.meta.dirname, "../..");
const workspaceRoot = resolve(desktopRoot, "..");

const cargoPath = resolve(workspaceRoot, "Cargo.toml");
const cargoLockPath = resolve(workspaceRoot, "Cargo.lock");
const packageJsonPath = resolve(desktopRoot, "package.json");
const tauriConfPath = resolve(desktopRoot, "src-tauri", "tauri.conf.json");
const changelogPath = resolve(workspaceRoot, "CHANGELOG.md");

const SEMVER_REGEX = /^(\d+)\.(\d+)\.(\d+)$/;

function bumpVersion(current, level) {
  const match = current.match(SEMVER_REGEX);
  if (!match) {
    throw new Error(`Current version "${current}" is not a valid semver string`);
  }
  let [_, major, minor, patch] = match.map(Number);
  switch (level) {
    case "major":
      major += 1;
      minor = 0;
      patch = 0;
      break;
    case "minor":
      minor += 1;
      patch = 0;
      break;
    case "patch":
      patch += 1;
      break;
    default:
      throw new Error(`Unknown bump level "${level}". Use patch, minor or major.`);
  }
  return `${major}.${minor}.${patch}`;
}

async function readCargoVersion() {
  const content = await readFile(cargoPath, "utf-8");
  const match = content.match(/^\s*version\s*=\s*"([^"]+)"/m);
  if (!match) {
    throw new Error(`Could not find version in ${cargoPath}`);
  }
  return match[1];
}

async function updateCargoVersion(version) {
  const content = await readFile(cargoPath, "utf-8");
  const updated = content.replace(
    /^(\[workspace\.package\][\s\S]*?^\s*version\s*=\s*")([^"]+)(")/m,
    `$1${version}$3`
  );
  if (updated === content) {
    throw new Error(`Failed to update version in ${cargoPath}`);
  }
  await writeFile(cargoPath, updated);
}

async function updateJsonVersion(path, version) {
  const content = await readFile(path, "utf-8");
  const json = JSON.parse(content);
  json.version = version;
  await writeFile(path, JSON.stringify(json, null, 2) + "\n");
}

async function updateChangelog(version) {
  const date = new Date().toISOString().split("T")[0];
  const entry = `## [${version}] - ${date}\n\n- Release version ${version}.\n\n`;
  let content;
  try {
    content = await readFile(changelogPath, "utf-8");
  } catch (err) {
    if (err.code === "ENOENT") {
      content = "# Changelog\n\n";
    } else {
      throw err;
    }
  }
  content = content.replace(/^# Changelog\n\n/i, `# Changelog\n\n${entry}`);
  await writeFile(changelogPath, content);
}

function runGitSilent(args) {
  return execSync(`git ${args}`, {
    cwd: workspaceRoot,
    stdio: ["ignore", "pipe", "pipe"],
    encoding: "utf-8",
  }).trim();
}

function runGit(args) {
  execSync(`git ${args}`, {
    cwd: workspaceRoot,
    stdio: "inherit",
    encoding: "utf-8",
  });
}

async function main() {
  const level = process.argv[2];
  if (!level) {
    console.error("Usage: pnpm version:bump <patch|minor|major>");
    process.exit(1);
  }

  const current = await readCargoVersion();
  const next = bumpVersion(current, level);

  const status = runGitSilent("status --porcelain");
  if (status !== "") {
    console.error("Working tree is not clean. Commit or stash changes before bumping version.");
    console.error(status);
    process.exit(1);
  }

  console.log(`Bumping version: ${current} -> ${next}`);

  await updateCargoVersion(next);
  await updateJsonVersion(packageJsonPath, next);
  await updateJsonVersion(tauriConfPath, next);
  await updateChangelog(next);

  console.log("Updating Cargo.lock...");
  execSync("cargo update --workspace", { cwd: workspaceRoot, stdio: "inherit", encoding: "utf-8" });

  runGit(`add "${cargoPath}" "${cargoLockPath}" "${packageJsonPath}" "${tauriConfPath}" "${changelogPath}"`);
  runGit(`commit -m "chore(release): bump version to ${next}"`);
  runGit(`tag -a "v${next}" -m "Release v${next}"`);

  console.log(`\nVersion ${next} committed and tagged as v${next}.`);
  console.log(`Run "pnpm release:build" to produce release artifacts.`);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
