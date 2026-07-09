# README.md 重设计实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将根 `README.md` 重设计为以中文为主、视觉冲击力强、信息层级清晰的落地页，同时把发布 tag 同步到 `master` 的规则写入 `AGENTS.md`。

**Architecture:** 纯文档变更：从 `develop` 切出 `feat/readme-redesign` 分支，重写 `README.md`，追加 `AGENTS.md` 规则；通过 `markdownlint-cli2` 验证格式，最终推送分支并创建 PR。

**Tech Stack:** Markdown, shields.io badges, GitHub Actions CI badge, Tauri v2, React 19, Rust 1.88+.

## Global Constraints

- 不新增图片/视频素材，复用 `docs/assets/screenshots/01-translate.png`。
- 根 `README.md` 控制在 200 行以内。
- 所有 badge 链接必须真实可用；优先使用引用式定义以控制行长度。
- Markdown 行长度限制 120 字符（`.markdownlint-cli2.yaml`）。
- 符合项目 Git Flow：从 `develop` 切 feature 分支，禁止直接向 `master`/`develop` 推送。
- 提交信息使用 Conventional Commits（`docs:`）。

---

### Task 1: 创建功能分支

**Files:** 仅 git 操作

**Interfaces:** N/A

- [ ] **Step 1: 切回 develop 并同步远程**

Run:
```bash
git checkout develop
git pull origin develop
```
Expected: 当前分支为 `develop`，`git status` 显示 `Your branch is up to date with 'origin/develop'`。

- [ ] **Step 2: 创建功能分支**

Run:
```bash
git checkout -b feat/readme-redesign
```
Expected: 当前分支切换为 `feat/readme-redesign`。

- [ ] **Step 3: 提交已存在的设计文档**

Run:
```bash
git add docs/superpowers/specs/2026-07-09-readme-redesign-design.md
git commit -m "docs: add README redesign spec"
```

- [ ] **Step 4: 提交实现计划**

Run:
```bash
git add docs/superpowers/plans/2026-07-09-readme-redesign.md
git commit -m "docs: add README redesign implementation plan"
```

---

### Task 2: 重写根 README.md

**Files:**
- Modify: `README.md`

**Interfaces:** N/A

- [ ] **Step 1: 清空旧 README.md**

Use Write tool to overwrite `README.md` with an empty string (will be replaced in next step).

- [ ] **Step 2: 写入新 README.md 内容**

Use Write tool to overwrite `README.md` with exactly the following content:

````markdown
<div align="center">

# 巴别塔 · BabelEbook

> 把你的 EPUB 电子书翻译成高质量双语对照版  
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
````

- [ ] **Step 3: 验证格式与行数**

Run:
```bash
wc -l README.md
```
Expected: 行数 ≤ 200。

Run:
```bash
npx markdownlint-cli2 README.md
```
Expected: 无错误（`0 errors`）。

- [ ] **Step 4: Commit**

Run:
```bash
git add README.md
git commit -m "docs: redesign root README as landing page"
```

---

### Task 3: 更新 AGENTS.md 发布流程

**Files:**
- Modify: `AGENTS.md`

**Interfaces:** N/A

- [ ] **Step 1: 读取 AGENTS.md 的 Release Workflow 段落**

Use Read tool to inspect `AGENTS.md` around `## Release Workflow (Git Flow)`。

- [ ] **Step 2: 插入 tag → master 同步规则**

Use Edit tool to insert以下规则到第 5 步之后、第 6 步之前（或作为独立步骤放在流程末尾）：

```markdown
5. **The latest release tag must always be reachable from `master`.**  
   After `pnpm version:bump` creates and pushes the tag, fast-forward `master`
   to that tag so that `master` points to the latest release:

   ```bash
   git checkout master
   git merge --ff-only v<x.y.z>
   git push origin master
   ```
```

（若原有步骤编号已是 5/6/7，请顺延编号，保持列表连续。）

- [ ] **Step 3: 验证 AGENTS.md 格式**

Run:
```bash
npx markdownlint-cli2 AGENTS.md
```
Expected: 无错误。

- [ ] **Step 4: Commit**

Run:
```bash
git add AGENTS.md
git commit -m "docs: enforce fast-forward master to latest release tag"
```

---

### Task 4: 质量门验证

**Files:** `README.md`, `AGENTS.md`

**Interfaces:** N/A

- [ ] **Step 1: 运行 markdownlint**

Run:
```bash
npx markdownlint-cli2 README.md AGENTS.md
```
Expected: 无错误。

- [ ] **Step 2: 检查相对链接**

Run:
```bash
grep -nE "\]\([^h][^/]|\]\(docs/|\]\(\.github/|\]\(LICENSE" README.md
```
Expected: 所有相对链接均指向仓库内实际存在的文件。

- [ ] **Step 3: 查看整体 diff**

Run:
```bash
git diff develop --stat
```
Expected: 只修改 `README.md`、`AGENTS.md`，以及已提交的 `docs/superpowers/specs/` 和 `docs/superpowers/plans/` 文件。

---

### Task 5: 推送分支并创建 Pull Request

**Files:** git/PR

**Interfaces:** N/A

- [ ] **Step 1: 推送分支**

Run:
```bash
git push origin feat/readme-redesign
```
Expected: 分支推送到 origin。

- [ ] **Step 2: 创建 PR（target 为 develop）**

Run:
```bash
gh pr create --base develop --head feat/readme-redesign \
  --title "docs: redesign root README and add tag-to-master rule" \
  --body "- 重写根 README.md 为中文主场落地页，强化 hero、badge、功能卡片与下载入口。\n- 在 AGENTS.md 发布流程中增加 master 必须 fast-forward 到最新 release tag 的规则。"
```
Expected: PR 创建成功，返回 PR URL。

---

## Spec Coverage Check

| Spec 要求 | 对应 Task |
|---|---|
| 中文主场 + 双语 hero | Task 2 |
| Badge 矩阵（CI / license / release / stars / downloads / platforms / tech stack / privacy） | Task 2 |
| 首屏下载入口 + 功能卡片 | Task 2 |
| 30 秒上手（桌面 + CLI） | Task 2 |
| 供应商表、输出模式、平台支持 | Task 2 |
| `AGENTS.md` tag → master 规则 | Task 3 |
| 不新增图片资源 | Global Constraint |
| 行数 ≤ 200、markdownlint 通过 | Task 2/4 |
| Git Flow feature 分支 + PR | Task 1/5 |

## Placeholder Scan

- 无 `TBD`、`TODO`、`implement later`。
- 无未定义的函数/类型引用（纯文档变更）。
- 所有文件路径均为绝对或相对仓库根目录的精确路径。
