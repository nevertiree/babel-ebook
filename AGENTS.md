# AGENTS.md — BabelEbook

## Project Overview

BabelEbook（巴别塔）is an EPUB translator powered by large language models. It produces
bilingual e-books (Chinese/English by default) and ships as both a Rust CLI and a Tauri + React
desktop GUI.

- **Repository root**: project root (e.g. `C:/Users/<you>/Documents/Codebase/babel-ebook` on Windows)
- **Language**: Rust (core + CLI) + TypeScript/React (desktop)
- **Package manager**: `pnpm` (desktop), `cargo` (Rust)
- **Current version**: see `Cargo.toml` / `git describe --tags`

## Quick Commands

```bash
# Rust workspace
cargo build --workspace --release
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt -- --check

# Desktop (run from desktop/)
cd desktop
pnpm install
pnpm tauri dev          # hot-reload GUI dev
pnpm tauri build        # production installer build
pnpm build              # frontend-only production build
pnpm e2e                # Playwright E2E tests against the release binary
pnpm version:bump patch # bump workspace version and create annotated tag
pnpm release:build      # full release build (must run on a tag)
```

## Project Layout

```text
├── Cargo.toml              # workspace manifest, single source of truth for version
├── Cargo.lock
├── CHANGELOG.md
├── docs/README.md
├── AGENTS.md               # this file
├── crates/
│   ├── babel-ebook/        # core translation library
│   └── babel-ebook-cli/    # CLI binary
├── desktop/
│   ├── package.json
│   ├── playwright.config.ts
│   ├── e2e/                # Playwright tests
│   ├── scripts/            # bump-version.mjs, release.mjs, copy-release.mjs, gui-full-translate.mjs
│   ├── src/                # React frontend
│   └── src-tauri/          # Tauri Rust backend
└── release/v<x.y.z>/       # final distributable installers (created by release:build)
```

## Architecture Notes

- Rust core (`crates/babel-ebook`) handles EPUB parsing, chunking, caching, LLM calls, and progress events.
- CLI (`crates/babel-ebook-cli`) is a thin wrapper around the core.
- Desktop app streams progress via Tauri events (`translation_progress`) to the React UI.
- The workspace version lives in `Cargo.toml` `[workspace.package].version` and is mirrored
  to `desktop/package.json` and `desktop/src-tauri/tauri.conf.json` by `pnpm version:bump`.

## Adding or Changing Features

- Create feature branches from `develop`.
- Keep commits focused; use conventional commit style (`feat(...)`, `fix(...)`, `chore(...)`).
- Run the full quality gates before opening a PR or merging:

  ```bash
  cargo fmt -- --check
  cargo clippy --workspace --all-targets -- -D warnings
  cargo test --workspace
  cd desktop && pnpm exec tsc --noEmit && pnpm build
  ```

- For desktop changes, add or update Playwright E2E tests in `desktop/e2e/`.
- Update `docs/README.md` and `CHANGELOG.md` if user-facing behavior changes.
- Update this `AGENTS.md` if build/test/release workflows change.

## Branch & Merge Workflow

`master` and `develop` are protected branches. **Never push commits directly to them.**
Always work in a feature branch and merge through a Pull Request.

Typical workflow using the GitHub CLI (`gh`):

```bash
# 1. Make sure you are on the latest develop
git checkout develop
git pull origin develop

# 2. Create a feature branch
git checkout -b feat/<short-description>

# 3. Make focused commits
git add ...
git commit -m "feat(scope): description"

# 4. Push the branch
git push origin feat/<short-description>

# 5. Open a Pull Request
gh pr create --base develop --head feat/<short-description> \
  --title "feat(scope): description" \
  --body "What this PR does and why."

# 6. Wait for CI to pass. Do not merge while checks are failing.
gh run watch <run-id> --exit-status

# 7. Merge via gh (squash is preferred for feature branches)
gh pr merge <pr-number> --squash --delete-branch

# 8. Pull the updated base branch locally
git checkout develop
git pull origin develop
```

For documentation-only or hot-fix changes that must land on `master`, branch from
`master` and set `--base master` when creating the PR.

## Running Translations

CLI:

```bash
cargo run --release -p babel-ebook-cli -- input.epub -o output.epub \
  --provider deepseek --model deepseek-chat --concurrency 3 \
  --max-input-tokens 4000 --max-output-tokens 2000
```

Desktop:

```bash
cd desktop
pnpm tauri dev
```

## E2E Testing

The Playwright suite launches the release binary and connects over WebView2 CDP.

```bash
cd desktop
pnpm tauri build   # ensure target/release/babel-ebook-desktop.exe exists
BABEL_EBOOK_E2E_API_KEY=sk-... pnpm e2e
```

Environment variables used for injection:

- `BABEL_EBOOK_E2E_CDP_PORT` — WebView2 remote-debugging port (default used by the test is `9222`)
- `BABEL_EBOOK_E2E_SOURCE` — pre-fill source EPUB path
- `BABEL_EBOOK_E2E_OUTPUT` — pre-fill output EPUB path
- `BABEL_EBOOK_E2E_API_KEY` — pre-fill API key
- `BABEL_EBOOK_E2E_DRY_RUN` — set `"true"` for dry-run tests
- `BABEL_EBOOK_E2E_UI_LANGUAGE` — force UI language, e.g. `zh-CN` or `en`

If the CDP port is not exposed, kill hung `babel-ebook-desktop.exe` and `msedgewebview2.exe` processes first.

## Release Workflow (Git Flow)

1. Merge feature branches into `develop`.
2. On `master` (or a `release/*` branch), run `pnpm version:bump patch|minor|major`.
3. The bump script updates version files, syncs `Cargo.lock`, updates `CHANGELOG.md`,
   commits, and creates an annotated tag `v<x.y.z>`.
4. Run `pnpm release:build` on the tag commit.
5. Merge `master` back into `develop` so both branches carry the version bump.

## Security & Secrets

- **Never commit API keys.** Use environment variables, keyring, or local config files ignored by `.gitignore`.
- `test_config.json` and `.env` are already ignored.
- The E2E script accepts `BABEL_EBOOK_E2E_API_KEY` from the environment only.

## Common Gotchas

- `pnpm release:build` must be run on a tag commit; otherwise it exits with
  `HEAD does not point to a tag`.
- Windows release builds may fail to overwrite `target/release/babel-ebook-desktop.exe` if old
  processes are still running. Kill them first.
- WebView2 CDP port `9222` may already be occupied by a system `msedgewebview2.exe`;
  use a different port or kill the existing process.
- The desktop app stores settings via Tauri Store and the OS keyring; clean these if tests behave unexpectedly.
