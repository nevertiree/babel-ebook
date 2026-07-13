# BabelEbook

[![CI][ci-badge]][ci-url]
[![License: MIT][license-badge]][license-url]
[![Rust Version][rust-badge]][rust-url]
[![Release][release-badge]][release-url]

**BabelEbook** is an EPUB translator powered by large language models.
It produces bilingual e-books (source language + target language)
where each translated paragraph is followed by the original text.

Read this in other languages: [简体中文](README.md)

[ci-badge]: https://github.com/nevertiree/babel-ebook/actions/workflows/ci.yml/badge.svg
[ci-url]: https://github.com/nevertiree/babel-ebook/actions/workflows/ci.yml
[license-badge]: https://img.shields.io/badge/License-MIT-yellow.svg
[license-url]: ../LICENSE
[rust-badge]: https://img.shields.io/badge/rust-1.88%2B-blue.svg
[rust-url]: https://www.rust-lang.org/
[release-badge]: https://img.shields.io/github/v/release/nevertiree/babel-ebook
[release-url]: https://github.com/nevertiree/babel-ebook/releases

> Your EPUB content and API keys are processed only on your own computer
> and are never sent to the project maintainers' servers.
>
> Read this in other languages:
> [中文](README.md) · [English](README.en.md) · [日本語](README.ja.md) · [한국어](README.ko.md) · [Русский](README.ru.md)
> [Español](README.es.md)

<p align="center">
  <img src="assets/screenshots/01-translate.png" alt="BabelEbook main window" width="800">
</p>

<p align="center">
  <a href="https://github.com/nevertiree/babel-ebook/releases/download/v0.4.0/BabelEbook_0.4.0_x64-setup.exe">
    <img alt="Download for Windows" src="https://img.shields.io/badge/Windows-Download-blue?logo=windows&logoColor=white">
  </a>
  <a href="https://github.com/nevertiree/babel-ebook/releases/download/v0.4.0/BabelEbook_0.4.0_amd64.AppImage">
    <img alt="Download for Linux" src="https://img.shields.io/badge/Linux-Download-orange?logo=linux&logoColor=white">
  </a>
</p>

---

## Why BabelEbook?

| Feature | BabelEbook | Online translators | Calibre plugins |
|---------|------------|--------------------|-----------------|
| Fully local: EPUB never uploaded | ✅ | ❌ | ✅ |
| Bilingual side-by-side layout | ✅ | Partial | Manual adjustment needed |
| One-click desktop installer | ✅ | No install needed | Requires Calibre |
| DeepSeek / OpenAI / Anthropic / Ollama | ✅ | Fixed vendor | Plugin dependent |
| Glossary, exclude selectors, concurrency | ✅ | Partial | Plugin dependent |

---

## Screenshots

| Main window | Providers | Translation options |
|-------------|------------------|---------------------|
| ![Main window](assets/screenshots/01-translate.png) | ![Compute settings](assets/screenshots/02-settings-compute.png) | ![Translation options](assets/screenshots/03-settings-translation.png) |

| Translation progress | Logs |
|----------------------|------|
| ![Translation progress](assets/screenshots/06-translate-progress.png) | ![Logs](assets/screenshots/07-logs-progress.png) |

---

## Platform Support

The desktop GUI is available on:

- **Windows** (recommended): `.exe` (NSIS) and `.msi` installers.
- **Linux**: `.AppImage` (portable, double-click to run) and `.deb` packages
  for Debian/Ubuntu-based distributions.

macOS currently does **not** have an official desktop installer.
macOS users can build and run the command-line version from source.

---

## User Guide

### Download and Install

1. Open the [Releases](https://github.com/nevertiree/babel-ebook/releases) page.
2. Download the installer for your system:

   **Windows**
   - **Recommended for most users**: `BabelEbook_<version>_x64-setup.exe`
     (NSIS installer, automatically matches the system language).
   - **IT admins or silent deployment**: `BabelEbook_<version>_x64_en-US.msi` (MSI installer).

   **Linux**
   - **Recommended for most distributions**: `BabelEbook_<version>_amd64.AppImage`
     (no installation needed; run `chmod +x` then double-click).
   - **Debian / Ubuntu**: `BabelEbook_<version>_amd64.deb`
     (double-click to install, or run `sudo dpkg -i BabelEbook_<version>_amd64.deb`).

3. Double-click the installer and follow the prompts.

> **Linux Chinese font display:** If your Linux system does not have a Chinese font installed,
> Chinese characters in the UI may appear as squares.
> Install a system-recommended Chinese font package, such as `fonts-noto-cjk` on Debian/Ubuntu:
>
> ```bash
> sudo apt-get install fonts-noto-cjk
> ```

### First Use

#### 1. Prepare an API key

BabelEbook needs to call a third-party large language model API.
It currently supports DeepSeek, OpenAI, Anthropic, and locally hosted Ollama.

Using DeepSeek as an example:

1. Visit the [DeepSeek platform](https://platform.deepseek.com/), sign up, and create an API key.
2. Open BabelEbook and go to **Settings** → **Providers**.
3. Select Provider `DeepSeek` and enter your API key.
4. Click **Test Connection** to verify the connection.

> If you use local Ollama, no API key is needed; just fill in the Base URL (for example `http://localhost:11434`).

### Translate a Book

1. On the main screen click **Select EPUB** to choose the e-book you want to translate.
2. Select the target language (default `zh-CN` for Simplified Chinese).
3. Click **Start Translation**.
4. The output file will be saved to the location you specified.

### Common Settings

| Setting | Description |
|---------|-------------|
| Providers | Select the LLM vendor and enter the API key. |
| Target Language | Target translation language, e.g. `zh-CN`, `en`, `ja`, etc. |
| Output Mode | `bilingual` (source + target), `translation_only` (target only), `interleaved` (alternating paragraphs). |
| Concurrency | Number of chapters translated in parallel. Higher is faster but costs more. |
| Max Input/Output Tokens | Maximum tokens per request. Defaults are usually fine. |
| Exclude Selectors | Elements to skip, e.g. `.code`, `pre`. |
| Glossary | Terminology table to fix translations of proper nouns. |

### Output Modes

- **Bilingual**: Each translated paragraph is followed by the original text. Great for language learning.
- **Translation only**: Only the translated content is kept.
- **Interleaved**: Source and target paragraphs alternate.

### UI Language

The desktop app supports English, Español, 日本語, 한국어, Русский, and 简体中文.
The UI language is selected automatically on first launch based on the system language and can be changed in Settings.

### FAQ

**Q: Why is the translation output empty or missing chapters?**
A: Check whether the EPUB content is a scanned image; if so, run OCR first.
You can also adjust `Exclude Selectors` to skip elements that should not be translated.

**Q: How many tokens will translation consume?**
A: Use **Dry Run** mode in the main screen or CLI to count tokens without actually calling the API.

**Q: Is my API key safe?**
A: Yes. API keys are stored in the Windows Credential Manager by default and are not saved in plain-text configuration files.

---

## Developer Guide

### Project Introduction

BabelEbook uses a Rust + TypeScript architecture:

- **Rust core** (`crates/babel-ebook`): EPUB parsing, chunking, caching, LLM calls.
- **Rust CLI** (`crates/babel-ebook-cli`): Command-line entry point.
- **Tauri desktop app** (`desktop/`): Rust backend + React/TypeScript frontend.

### Requirements

- [Rust](https://rustup.rs/) 1.88 or later
- [pnpm](https://pnpm.io/) 9+ (desktop development)
- Windows 10/11 (for desktop GUI development)
- An API key for the chosen vendor

### Quick Start

```bash
# Clone the repository
git clone https://github.com/nevertiree/babel-ebook.git
cd babel-ebook

# Build and test the Rust workspace
cargo build --workspace
cargo test --workspace

# Install desktop frontend dependencies
cd desktop
pnpm install

# Start the desktop dev server
pnpm tauri dev
```

### Project Layout

```text
├── Cargo.toml              # workspace version (single source of truth)
├── crates/
│   ├── babel-ebook/        # core translation library (Rust)
│   └── babel-ebook-cli/    # command-line interface (Rust)
├── desktop/
│   ├── src/                # React + i18next frontend (TypeScript)
│   ├── src-tauri/          # Tauri Rust backend
│   ├── e2e/                # Playwright GUI tests
│   └── scripts/            # build & release helpers
└── release/v<x.y.z>/       # final distributable installers (generated)
```

### Build Commands

#### CLI

```bash
cargo build --release -p babel-ebook-cli
# Output: target/release/babel-ebook
```

#### Windows Desktop Installer

```bash
cd desktop
pnpm install
pnpm tauri build
```

Outputs:

- MSI: `target/release/bundle/msi/BabelEbook_<version>_x64_en-US.msi`
- NSIS: `target/release/bundle/nsis/BabelEbook_<version>_x64-setup.exe`

#### Linux Desktop Installer

On Debian/Ubuntu or compatible distributions, first install Tauri dependencies:

```bash
sudo apt-get update
sudo apt-get install -y libwebkit2gtk-4.1-dev build-essential curl wget file \
  libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev xdg-utils
```

Then build:

```bash
cd desktop
pnpm install
pnpm tauri build
```

Outputs:

- AppImage: `target/release/bundle/appimage/BabelEbook_<version>_amd64.AppImage`
- deb: `target/release/bundle/deb/BabelEbook_<version>_amd64.deb`

> **Chinese UI font on Linux:** If your Linux system does not have a Chinese font installed,
> Chinese characters in the UI may appear as squares.
> Install `fonts-noto-cjk` (Debian/Ubuntu: `sudo apt-get install fonts-noto-cjk`)
> or another system Chinese font.

### Quality Gates

Before opening a PR, make sure the following pass:

```bash
cargo fmt -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

cd desktop
pnpm exec tsc --noEmit
pnpm build
```

### Contribution Guidelines

Contributions are welcome!
Please read [.github/CONTRIBUTING.md](.github/CONTRIBUTING.md),
[.github/CODE_OF_CONDUCT.md](.github/CODE_OF_CONDUCT.md), and
[.github/SECURITY.md](.github/SECURITY.md) first.

#### Branch Model

This project follows **Git Flow**:

- `master`: released production code.
- `develop`: daily integration baseline.
- `release/<version>`: release stabilization branch.
- `feature/<name>`: feature branch.

#### Commit Style

- Use [Conventional Commits](https://www.conventionalcommits.org/):
  - `feat:` new feature
  - `fix:` bug fix
  - `docs:` documentation update
  - `refactor:` refactoring
  - `chore:` build/tools/misc
- Keep commits small and focused.
- Do not commit API keys, personal paths, or internal planning documents.

#### PR Requirements

1. All CI checks pass.
2. Update `docs/README.md` and `CHANGELOG.md` if user-facing behavior changes.
3. Keep the diff scoped to the feature or fix.
4. Desktop changes should include or update Playwright E2E tests.

### Release Workflow

```bash
cd desktop

# 1. Bump version (patch / minor / major), sync Cargo.toml/package.json/tauri.conf.json, and create a tag
pnpm version:bump minor

# 2. Run the full build on the tag commit
pnpm release:build
```

Final artifacts are copied to `release/v<version>/`.

### Advanced CLI Usage

```bash
export DEEPSEEK_API_KEY=sk-...

cargo run --release -p babel-ebook-cli -- input.epub -o output.epub \
  --provider deepseek \
  --model deepseek-chat \
  --concurrency 3 \
  --max-input-tokens 4000 \
  --max-output-tokens 2000

# Estimate tokens only, without calling the API
cargo run --release -p babel-ebook-cli -- input.epub -o output.epub --dry-run

# Use a JSON configuration file
cargo run --release -p babel-ebook-cli -- input.epub -o output.epub --config config.json
```

Run `babel-ebook --help` for the full list of CLI arguments.

### Supported LLM Providers

| Provider | `--provider` | Default model | Base URL | Notes |
|----------|--------------|---------------|----------|-------|
| DeepSeek | `deepseek` | `deepseek-chat` | `https://api.deepseek.com` | Recommended default |
| OpenAI | `openai` | — | `https://api.openai.com/v1` | Requires explicit `--model` |
| Anthropic | `anthropic` | `claude-3-5-sonnet-20241022` | `https://api.anthropic.com` | — |
| Ollama | `ollama` | `llama3` | local | No API key needed |
| OpenAI-compatible | `openai-compatible` | — | Set via `base_url` | For self-hosted or proxy endpoints |

### Security

- **Never commit API keys:**
  - Use environment variables, the OS keyring, or local config files ignored by `.gitignore`.
  - Do not write API keys into code or commit them to Git.
- Report security vulnerabilities privately via [.github/SECURITY.md](.github/SECURITY.md).

### Acknowledgments

Built with Rust, Tauri, React, and i18next.

## License

MIT
