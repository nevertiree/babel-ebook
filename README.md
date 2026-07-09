# 巴别塔 · BabelEbook

[![CI][ci-badge]][ci-url]
[![License: MIT][license-badge]][license-url]
[![Rust Version][rust-badge]][rust-url]
[![Release][release-badge]][release-url]

**BabelEbook** is an EPUB translator powered by large language models.
It produces bilingual e-books where each translated paragraph is followed by
 the original text.

**BabelEbook（巴别塔）** 是一款基于大语言模型的 EPUB 翻译工具，
能把外文电子书翻译成中英双语版本。

<p align="center">
  <img src="docs/assets/screenshots/01-translate.png" alt="BabelEbook main window" width="800">
</p>

<p align="center">
  <a href="https://github.com/nevertiree/babel-ebook/releases/latest">
    <img alt="Download for Windows" src="https://img.shields.io/badge/Windows-Download-blue?logo=windows&logoColor=white">
  </a>
  <a href="https://github.com/nevertiree/babel-ebook/releases/latest">
    <img alt="Download for Linux" src="https://img.shields.io/badge/Linux-Download-orange?logo=linux&logoColor=white">
  </a>
</p>

---

## Choose your language / 选择语言 / 言語を選択 / 언어 선택 / Выберите язык / Seleccione su idioma

- [简体中文](docs/README.md)
- [English](docs/README.en.md)
- [日本語](docs/README.ja.md)
- [한국어](docs/README.ko.md)
- [Русский](docs/README.ru.md)
- [Español](docs/README.es.md)

> Your EPUB content and API keys are processed only on your own computer
> and are never sent to the project maintainers' servers.

---

## 快速开始

### 环境要求

- [Rust](https://rustup.rs/) 1.88 或更新版本
- [Node.js](https://nodejs.org/) 20 或更新版本
- [pnpm](https://pnpm.io/) 9 或更新版本（或启用 corepack：`corepack enable`）
- **Windows 用户**：需要安装 [Visual Studio Build Tools 2022](https://visualstudio.microsoft.com/downloads/?q=build+tools) 的 **"C++ build tools"** 工作负载，以及 WebView2 Runtime

### 构建与运行

```bash
# 1. 克隆仓库
git clone https://github.com/nevertiree/babel-ebook.git
cd babel-ebook

# 2. 构建 Rust 工作空间
cargo build --workspace

# 3. 安装桌面前端依赖
cd desktop
pnpm install

# 4. 启动桌面端开发
pnpm tauri dev
```

### CLI 使用示例

```bash
cargo run --release -p babel-ebook-cli -- input.epub -o output.epub \
  --provider deepseek --model deepseek-chat
```

## 项目结构

```text
├── Cargo.toml              # Rust workspace 配置
├── crates/
│   ├── babel-ebook/        # 核心翻译库
│   └── babel-ebook-cli/    # 命令行工具
├── desktop/
│   ├── src/                # React 前端
│   └── src-tauri/          # Tauri Rust 后端
├── docs/                   # 多语言文档
└── tests/                  # 集成测试
```

## 常见问题

| 问题 | 解决方案 |
|------|----------|
| `cargo build` 报错 `link.exe` 找不到 | Windows 需要安装 VS Build Tools 的 C++ 工作负载 |
| `cargo build` 报错 `extra operand` | 同样是 MSVC linker 问题，安装 VS Build Tools 后重开终端 |
| `pnpm install` 因 corepack 网络失败 | 可临时改用 `npm install` 安装前端依赖 |
| `pnpm tauri dev` 无法启动窗口 | 检查 WebView2 Runtime 是否已安装 |

## 参与贡献

欢迎提交 Issue 和 Pull Request！请先阅读 [.github/CONTRIBUTING.md](.github/CONTRIBUTING.md)。

[ci-badge]: https://github.com/nevertiree/babel-ebook/actions/workflows/ci.yml/badge.svg
[ci-url]: https://github.com/nevertiree/babel-ebook/actions/workflows/ci.yml
[license-badge]: https://img.shields.io/badge/License-MIT-yellow.svg
[license-url]: LICENSE
[rust-badge]: https://img.shields.io/badge/rust-1.88%2B-blue.svg
[rust-url]: https://www.rust-lang.org/
[release-badge]: https://img.shields.io/github/v/release/nevertiree/babel-ebook
[release-url]: https://github.com/nevertiree/babel-ebook/releases
