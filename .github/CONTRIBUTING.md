# 为 BabelEbook 做贡献

感谢你对改进 BabelEbook 的兴趣！本文档提供了为本项目做贡献的指南。

## 开始

1. Fork 本仓库并克隆你的 Fork。
2. 安装 [Rust](https://rustup.rs/) 1.85 或更新版本。
3. 安装 [pnpm](https://pnpm.io/) 用于桌面前端开发。
4. 构建工作空间：

   ```bash
   cargo build --workspace
   cd desktop && pnpm install
   ```

## 开发流程

- 从 `develop` 分支创建功能分支。
- 保持提交聚焦，并尽量使用约定式提交风格（`feat:`、`fix:`、`chore:`）。
- 提交 Pull Request 前运行质量门：

  ```bash
  cargo fmt -- --check
  cargo clippy --workspace --all-targets -- -D warnings
  cargo test --workspace
  cd desktop && pnpm exec tsc --noEmit && pnpm build
  ```

## 报告问题

报告 Bug 时，请包含：

- BabelEbook 版本（或提交哈希）。
- 复现步骤。
- 预期行为与实际行为。
- 操作系统和 Rust 版本。

## Pull Request 流程

1. 确保所有测试和检查通过。
2. 如果改动影响用户可见的行为，更新 `README.md` 和 `CHANGELOG.md`。
3. 保持 diff 范围与当前功能或修复相关。

## 行为准则

请在所有项目互动中保持尊重和建设性。
