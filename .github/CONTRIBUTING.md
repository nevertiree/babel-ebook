# 为 BabelEbook 做贡献

感谢你对改进 BabelEbook 的兴趣！本文档提供了为本项目做贡献的指南。

## 开始

1. Fork 本仓库并克隆你的 Fork。
2. 安装 [Rust](https://rustup.rs/) 1.88 或更新版本（Windows 请选择 `x86_64-pc-windows-msvc` toolchain）。
3. **Windows 额外要求**：安装 [Visual Studio Build Tools 2022](https://visualstudio.microsoft.com/downloads/?q=build+tools) 的 **"C++ build tools"** 工作负载，并确保 WebView2 Runtime 已安装。

   ```powershell
   winget install Microsoft.VisualStudio.2022.BuildTools --override "--wait --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
   ```
4. 安装 [Node.js](https://nodejs.org/) 20 或更新版本。
5. 安装 [pnpm](https://pnpm.io/) 9 或更新版本（或执行 `corepack enable`）。
6. 构建工作空间：

   ```bash
   cargo build --workspace
   cd desktop && pnpm install
   ```

   > 如果 `pnpm install` 因 corepack 网络问题失败，可临时改用 `npm install` 安装前端依赖，但提交前仍建议使用 pnpm 重新生成 `pnpm-lock.yaml`。

## 开发流程

- **所有功能开发必须在 `develop` 分支上进行**；禁止从 `master` 切功能分支。
- 从 `develop` 创建功能分支，例如 `feat/<short-description>` 或 `docs/<short-description>`。
- `master` 和 `develop` 是受保护分支，**禁止直接推送和删除**。
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
2. 功能分支的 PR **必须以 `develop` 为 base**，禁止直接合并到 `master`。
3. 如果改动影响用户可见的行为，更新 `docs/README.md` 和 `CHANGELOG.md`。
4. 保持 diff 范围与当前功能或修复相关。
5. `master` 仅用于保存带 tag 的文档/发布序列，不接受功能代码直接合并。

## 行为准则

请在所有项目互动中保持尊重和建设性。
