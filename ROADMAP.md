# BabelEbook Roadmap

> 本路线图记录已确认的方向和欢迎社区贡献的领域。
> 如果你想认领某一项，请先开 Discussion 或在对应 issue 下留言，避免重复工作。

## 近期（Next 1–2 releases）

- [ ] **macOS 桌面安装包**：让 CI 在 `desktop-macos-bundle` job 中构建 `.dmg` / `.app`。
- [ ] **Linux 发布产物补全**：确保 Release 包含 `.AppImage` 和 `.deb`。
- [ ] **Linux 中文字体检测/提示**：如果系统无中文字体，启动时提示安装 `fonts-noto-cjk`。
- [ ] **翻译缓存可视化**：在桌面端显示已缓存章节数、预计节省的 token 数。

## 中期（3–6 个月）

- [ ] **更多 LLM 供应商**：Azure OpenAI、Google Gemini、本地 llama.cpp 等。
- [ ] **插件/扩展机制**：允许用户用 WASM 或脚本自定义分块、后处理规则。
- [ ] **翻译记忆库**：长期保存术语与片段翻译，提高一致性和复用率。
- [ ] **EPUB 内嵌字体**：为生成的双语 EPUB 注入目标语言字体，改善跨设备显示。

## 长期（6 个月以上）

- [ ] **移动端支持**：评估 Android / iOS 的可行性（可能基于 Tauri Mobile）。
- [ ] **协作式术语表**：导入/导出共享术语表，支持社区维护。
- [ ] **云端同步（可选自托管）**：翻译记忆、设置跨设备同步，但默认仍保持本地优先。

## 如何参与

1. 查看 [Issues](https://github.com/nevertiree/babel-ebook/issues) 中带有 `good first issue` 或 `help wanted` 标签的任务。
2. 阅读 [CONTRIBUTING.md](.github/CONTRIBUTING.md) 和 [CODE_OF_CONDUCT.md](.github/CODE_OF_CONDUCT.md)。
3. 在 Discussion 中提出你的想法或认领任务。
