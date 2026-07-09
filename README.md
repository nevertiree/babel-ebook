<!-- markdownlint-disable MD041 -->
<div align="center">

# 巴别塔 · BabelEbook

> 把你的 EPUB 电子书翻译成高质量双语对照版<br>
> Translate your EPUBs into high-quality bilingual editions with LLMs.

<p>
  <a href="https://github.com/nevertiree/babel-ebook/actions/workflows/ci.yml">
    <img src="https://github.com/nevertiree/babel-ebook/actions/workflows/ci.yml/badge.svg" alt="CI">
  </a>
  <a href="LICENSE">
    <img src="https://img.shields.io/badge/License-MIT-yellow.svg" alt="License: MIT">
  </a>
  <a href="https://github.com/nevertiree/babel-ebook/releases">
    <img src="https://img.shields.io/github/v/release/nevertiree/babel-ebook" alt="Release">
  </a>
  <a href="https://github.com/nevertiree/babel-ebook/stargazers">
    <img src="https://img.shields.io/github/stars/nevertiree/babel-ebook?style=flat" alt="Stars">
  </a>
  <a href="https://github.com/nevertiree/babel-ebook/releases">
    <img src="https://img.shields.io/github/downloads/nevertiree/babel-ebook/total" alt="Downloads">
  </a>
  <a href="https://github.com/nevertiree/babel-ebook/commits/master">
    <img src="https://img.shields.io/github/last-commit/nevertiree/babel-ebook" alt="Last Commit">
  </a>
</p>

<p>
  <a href="https://github.com/nevertiree/babel-ebook/releases/latest">
    <img src="https://img.shields.io/badge/Windows-Download-0078D4?logo=windows&logoColor=white" alt="Download for Windows">
  </a>
  <a href="https://github.com/nevertiree/babel-ebook/releases/latest">
    <img src="https://img.shields.io/badge/Linux-Download-E95420?logo=linux&logoColor=white" alt="Download for Linux">
  </a>
  <img src="https://img.shields.io/badge/macOS-CLI%20only-silver?logo=apple&logoColor=white" alt="macOS CLI only">
</p>

<p>
  <img src="https://img.shields.io/badge/Rust-1.88%2B-000000?logo=rust&logoColor=white" alt="Rust">
  <img src="https://img.shields.io/badge/Tauri-v2-24C8D8?logo=tauri&logoColor=white" alt="Tauri">
  <img src="https://img.shields.io/badge/React-19-61DAFB?logo=react&logoColor=white" alt="React">
  <img src="https://img.shields.io/badge/i18n-6%20languages-26A17B?logo=i18next&logoColor=white" alt="i18n">
</p>

<p>
  <img src="https://img.shields.io/badge/Privacy%20First-local%20processing-6f42c1" alt="Privacy First">
  <img src="https://img.shields.io/badge/Local%20First-no%20cloud-20c997" alt="Local First">
</p>

<p>
  <a href="docs/README.md">中文文档</a> ·
  <a href="docs/README.en.md">English Docs</a> ·
  <a href="docs/README.ja.md">日本語</a> ·
  <a href="docs/README.ko.md">한국어</a> ·
  <a href="docs/README.ru.md">Русский</a> ·
  <a href="docs/README.es.md">Español</a>
</p>

<img src="docs/assets/screenshots/01-translate.png" alt="BabelEbook 主界面" width="800">

</div>

---

## 核心亮点

| | |
|:---|:---|
| 🛡️ **完全本地 & 隐私优先** | EPUB 内容和 API Key 只在你电脑上处理，Key 由系统凭据管理器安全保存。 |
| 📖 **双语对照排版** | 每段译文后紧跟原文，支持 `bilingual` / `translation_only` / `interleaved` 三种模式。 |
| 🤖 **多模型支持** | DeepSeek、OpenAI、Anthropic、Ollama 以及任意 OpenAI-compatible 接口。 |
| 🖥️ **桌面 + CLI** | Tauri 图形界面开箱即用；Rust CLI 适合批量、自动化和服务器场景。 |
| ⚡ **高度可配置** | 并发、术语表、排除选择器、输出文件名模板、自定义 prompt 模板。 |

## 30 秒上手

### 桌面用户

1. 从 [Releases](https://github.com/nevertiree/babel-ebook/releases/latest) 下载 Windows `.exe` 或 Linux `.AppImage`。
2. 安装后打开应用，进入 **Settings → Provider / API**，填入你的 API Key 并测试连接。
3. 选择源 EPUB、目标语言、输出模式，点击 **Start Translation**。

### CLI 快速开始

```bash
# 安装/克隆后
cargo run --release -p babel-ebook-cli -- input.epub -o output.epub \
  --provider deepseek --model deepseek-chat --concurrency 3
```

更多参数请运行 `babel-ebook --help` 或查看 [docs/README.md](docs/README.md)。

## 支持的 LLM 供应商

| Provider | `--provider` | 默认模型 | 备注 |
|----------|--------------|----------|------|
| DeepSeek | `deepseek` | `deepseek-chat` | 默认推荐 |
| OpenAI | `openai` | — | 需显式指定 `--model` |
| Anthropic | `anthropic` | `claude-3-5-sonnet-20241022` | — |
| Ollama | `ollama` | `llama3` | 本地运行，无需 API Key |
| OpenAI-compatible | `openai-compatible` | — | 通过 `base_url` 指定 |

## 输出模式

- **Bilingual（双语对照）**：译文在前，原文紧跟其后，适合学习。
- **Translation Only（仅译文）**：只保留翻译后的内容。
- **Interleaved（交错排列）**：原文与译文段落交替出现。

## 平台支持

- **Windows 10/11**：推荐 `.exe`（NSIS）安装包，另提供 `.msi`。
- **Linux**：提供 `.AppImage`（免安装）和 `.deb`（Debian/Ubuntu）。
- **macOS**：暂无官方桌面安装包，可通过 CLI 从源码编译使用。

详细平台说明见 [docs/README.md](docs/README.md)。

## 安全与隐私

- EPUB 不上传到项目维护者的服务器。
- API Key 默认存入操作系统 keyring，不以明文保存。
- 发现安全漏洞请通过 [.github/SECURITY.md](.github/SECURITY.md) 私下报告。

## 参与贡献

欢迎 Issue 和 PR！请先阅读 [.github/CONTRIBUTING.md](.github/CONTRIBUTING.md)。

## License

[MIT](LICENSE)

---

<p align="center">Built with Rust, Tauri, React, and i18next.</p>
