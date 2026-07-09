# README.md 与项目入口页重设计

## 背景

当前根目录 `README.md` 同时承担了“项目首页”和“快速入口”两个角色，但存在以下问题：

- **卖点不突出**：标题下方直接是技术说明，缺少一句能让人立刻记住的 slogan。
- **视觉冲击力弱**：只有 4 个基础 badge，没有平台、下载量、技术栈等能增强信任感的标签。
- **行动路径分散**：下载按钮藏在截图下方，且 Linux 链接指向的是固定版本 `0.1.0` 的静态 URL。
- **信息层级模糊**：普通用户和开发者内容混排，根 README 应该像落地页一样先留住用户，再把深度读者引导到 `docs/README.md`。

`docs/README.md` 本身是合格的用户/开发者指南，但根入口没有起到“漏斗”作用。

## 目标

1. **第一眼就让人觉得项目很专业**：通过 hero 区、badge 矩阵、功能卡片和截图营造出“成熟产品”的感觉。
2. **30 秒内知道怎么用**：下载 / 安装 / CLI 命令一目了然。
3. **可信**：强调本地运行、隐私、开源、多供应商支持。
4. **维护简单**：不引入新的图片或 GIF 资源，复用现有截图；文案以事实为主，避免夸大。
5. **同步更新流程**：把“develop 上的最新 tag 要同步到 master”这条规则写进 `AGENTS.md`。

## 约束

- 不新增图片/视频素材（避免宣发素材缺失导致链接失效）。
- 保持根 `README.md` 作为仓库首屏（GitHub 渲染）。
- 保留多语言文档入口，但根 README 只使用一种主要语言，避免过度冗长。
- 所有 badge 链接必须真实可用（shields.io / GitHub native）。
- 符合项目 MIT 许可和现有分支保护规则。

## 候选方案

### 方案 A：中文主场 + 双语英雄区（推荐）

根 `README.md` 以中文为主，但在 hero 区保留英文副标题和多语言入口链接；`docs/README.en.md` 等继续维护英文完整文档。

- **优点**：与现有中文用户群和 `docs/README.md` 一致；hero 区的英文副标题仍能吸引国际用户。
- **缺点**：对纯英文 GitHub 访客来说，主要内容是中文。

### 方案 B：英文主场 + 中文入口

根 `README.md` 改为英文，hero 区附带一句中文 slogan 并链接到中文完整文档。

- **优点**：更符合 GitHub 全球流量习惯，显得“国际化”。
- **缺点**：与现有以中文为主的文档体系脱节，主要目标用户可能需要多点一次才能看到中文详情。

### 方案 C：极简高端

根 `README.md` 压缩到 60 行以内：居中大标题、一句 slogan、3 个 badge、一张截图、一组下载按钮、一个命令示例。

- **优点**：非常干净，维护成本最低。
- **缺点**：缺少功能矩阵和供应商展示，SEO 关键词和说服力都不足，难以满足“看起来很吊”的要求。

**推荐：方案 A。** 它在不破坏现有中文文档结构的前提下，用英文副标题、badge 矩阵和功能卡片把“高端感”拉满。

## 详细设计（方案 A）

### 1. Hero 区（居中）

```markdown
# 巴别塔 · BabelEbook

> 把你的 EPUB 电子书翻译成高质量双语对照版<br>
> Translate your EPUBs into high-quality bilingual editions with LLMs.

[Badge 行]
```

第一行必须同时出现中文名和英文名，第二行中文 slogan，第三行英文 slogan。

### 2. Badge 矩阵

分三行或一行密集排列（GitHub 渲染时会自动换行）：

- 左侧/核心：CI、License MIT、Latest Release、GitHub Stars、Total Downloads、Last Commit。
- 中间/平台：Windows、Linux、macOS CLI（静态 badge，标注 CLI only）。
- 右侧/技术栈：Rust 1.88+、Tauri、React、i18n、Privacy First、Local First。

使用 shields.io，颜色统一为品牌色（深蓝 / 紫色 / 黑色），避免彩虹色太花哨。

### 3. 行动按钮

居中显示：

- **Download for Windows** → 指向 `https://github.com/nevertiree/babel-ebook/releases/latest`，由 GitHub 自动定位最新 Windows 安装包。
- **Download for Linux** → 同样指向 latest release 页面，避免文件名版本号写死后失效。
- **CLI Quick Start** → 锚点到 CLI 示例。
- **中文完整文档** → `docs/README.md`。
- **English Docs** → `docs/README.en.md`。

### 4. 主截图

复用 `docs/assets/screenshots/01-translate.png`，宽度 `800`，加 `alt` 文本。

### 5. 功能卡片（Why BabelEbook）

用 2×2 或 5 列卡片展示：

| 图标 | 标题 | 一句话说明 |
|------|------|------------|
| 🛡️ | 完全本地 & 隐私优先 | EPUB 不上传，API Key 存系统 keyring |
| 📖 | 双语对照排版 | 每段译文后紧跟原文，适合学习 |
| 🤖 | 多模型支持 | DeepSeek / OpenAI / Anthropic / Ollama / 兼容接口 |
| 🖥️ | 桌面 + CLI | Tauri 图形界面 + Rust 命令行 |
| ⚡ | 高度可配置 | 并发、术语表、排除选择器、输出模式、自定义 prompt |

### 6. 30 秒上手

分两个 tab 式小标题（纯 markdown，非真实 tab）：

- **桌面用户**：下载安装包 → 填 API Key → 选 EPUB → 开始翻译。
- **CLI 用户**：给出最常用的一行命令，并链接到完整 CLI 用法。

### 7. 供应商与输出模式

用紧凑表格列出支持的 provider 和 `--provider` 值，以及三种输出模式说明。

### 8. 平台支持

简短说明 Windows / Linux 桌面包、macOS CLI only，并链接到平台限制详情。

### 9. 页脚

- Built with Rust, Tauri, React, i18next.
- License MIT。
- 贡献入口：`.github/CONTRIBUTING.md`。

### 10. AGENTS.md 更新

在“Release Workflow (Git Flow)”中增加一条：

> 发布 tag 生成并推送到远程后，必须将 `master` 分支 fast-forward 到该 tag：
>
> ```bash
> git checkout master
> git merge --ff-only v<x.y.z>
> git push origin master
> ```
>
> 确保 `master` 始终指向最新发布版本。如果 tag 是在 `develop`（或 `release/*`）分支上生成的，这一步同样适用。

## 改动文件

1. `README.md`：完全重写为方案 A 的落地页结构。
2. `AGENTS.md`：在发布流程里补充 develop tag → master 的同步规则。
3. `docs/README.md`（可选）：调整顶部的“其他语言版本”链接文字，使其与新的根 README 入口一致；不做大幅重写。

## 成功标准

- 根 `README.md` 在 GitHub 上首屏（不滚动）能看到：标题、slogan、badge 矩阵、下载按钮、主截图。
- 所有 badge 链接均有效（不返回 404）。
- 根 README 行数控制在 200 行以内，信息密度高但不冗长。
- `AGENTS.md` 中新增的规则与现有 Git Flow 不冲突。
- 不引入新的二进制图片资源。

## 明确排除

- 不重新设计 `docs/README.md` 的完整内容（只同步入口链接）。
- 不制作新的宣传视频或 GIF。
- 不新增项目的多语言根 README（仍通过 `docs/` 目录提供）。
