# BabelEbook 架构优雅性 / 技术负债 / 交互心智负担问题清单

> 本清单随重构进度更新。状态说明：已修复 / 修复中 / 待修复。

---

## 一、架构优雅性

| # | 位置 | 问题 | 影响 | 修复方案 | 状态 |
|---|---|---|---|---|---|
| 1 | `desktop/src/App.tsx` | 上帝组件：同时管理 form / general / queue / logs / toasts / sidebar / 页面路由等 10+ 种状态 | 组件超过 600 行，任何改动都需理解全局；单文件变更冲突概率高 | 提取 `useQueue`、`useLogState` hook；将进度相关逻辑从 ref 迁移到派生状态 | 已修复 |
| 2 | `desktop/src/types.ts` `FormState` | 40+ 字段的上帝对象，所有设置页都接收整个 `FormState` | 页面与大量无关字段耦合；无关字段变化导致不必要重渲染 | 拆分为 `ModelParams` / `TranslationSettingsState` / `PromptSettingsState` / `OutputSettingsState` / `QueueSettingsState` / `TranslateInputs`；各页面仅接收所需 slice，并用 `memo` 包裹 | 已修复 |
| 3 | `desktop/src/App.css` | 全局单文件 CSS 涵盖主题、基础、多页面、多组件 | 样式归属不清；修改变更影响面难以评估 | 拆分为 `styles/theme.css`、`styles/base.css` 及 `pages/*.css`、`components/*.css`，组件按需 import | 已修复 |
| 4 | `desktop/src-tauri/src/queue.rs` `QueueManager` | 异步上下文中使用 `std::sync::Mutex` | 阻塞 tokio worker 线程，可能引发 latency spike | 替换为 `tokio::sync::Mutex`；用 `mpsc` 通道解耦进度回调 | 已修复 |
| 5 | `desktop/src-tauri/src` 命令层 | `Result<T, String>` 遍布命令层 | 错误语义丢失，调用方只能拿到字符串；keyring / IO / 业务错误无法区分 | 新增 `error.rs` 定义 `AppError`；队列/持久化层统一使用，命令边界再转字符串 | 已修复 |
| 6 | `crates/babel-ebook/src/config.rs` `Config` | 30+ 字段混合文件路径、provider、翻译选项、prompt、glossary 等 | 每个模块引入 `Config` 后被迫依赖整个序列化结构；HTML 处理只需翻译选项却拿到 API key / 路径 | 提取 `TranslationOptions` 子结构，将 prompt / token 方法下沉；`Config` 保留转发方法兼容现有调用 | 已修复 |
| 7 | `crates/babel-ebook/src/html.rs` | 779 行承担解析、选择、跳过、翻译、进度适配、双语插入 | 单一模块职责过多，单测与阅读成本高 | 拆分为 `html/selection.rs`、`html/progress.rs`、`html/insertion.rs`、`html/translation.rs`、`html/mod.rs` | 已修复 |
| 8 | `crates/babel-ebook/src/epub.rs` `EpubBook` | 抽象薄弱：`EpubBook` 只是数据袋，`read_epub` / `write_epub` 是自由函数 | 使用方需要知道自由函数存在；OO 抽象不清晰 | 添加 `EpubBook::read` / `EpubBook::write` 关联方法，`core` 调用方法 | 已修复 |
| 9 | `crates/babel-ebook/src/pipeline.rs` / `core.rs` | `run_ordered_pipeline` 接收 9 个独立参数；`core` 直接 orchestrate pipeline / checkpoint / title / output | pipeline 与 core 紧耦合，参数列表膨胀 | 引入 `PipelineContext` 聚合 translator / config / cache / progress / cancellation；减少裸参数 | 已修复 |

---

## 二、技术负债

| # | 位置 | 问题 | 影响 | 修复方案 | 状态 |
|---|---|---|---|---|---|
| 1 | `desktop/src/App.tsx` 进度 refs | 进度信息累积在 ref 中，与 UI 状态不同步 | 日志/进度显示可能滞后或丢失 | 进度派生自 task 状态，统一走 `LogState` | 已修复 |
| 2 | `desktop/src/App.tsx` queue 刷新 | `task_progress` 事件直接本地 patch queue，与 `get_queue_state` 来源不一致 | 状态竞争，UI 可能回跳 | `task_progress` 只触发 `get_queue_state` 刷新，queue 单一来源 | 已修复 |
| 3 | `desktop/src/App.css` | 全局 CSS 单文件 | 难以维护，删除样式风险高 | 按页面/组件拆分 | 已修复 |
| 4 | `desktop/src-tauri/src` | `String` 错误 | 无法结构化处理错误 | `AppError` 枚举 | 已修复 |
| 5 | `desktop/src-tauri/src/queue.rs` | `std` Mutex | 异步阻塞 | `tokio` Mutex | 已修复 |
| 6 | `crates/babel-ebook/src/config.rs` | prompt / token 计算方法与 `Config` 绑定 | 测试与复用困难 | 下移至 `TranslationOptions` | 已修复 |
| 7 | `crates/babel-ebook/src/html.rs` | 模块过大（779 行） | 修改风险高 | 拆分子模块 | 已修复 |
| 8 | `crates/babel-ebook/src/epub.rs` | 读写自由函数 | 抽象泄漏 | 封装为 `EpubBook` 方法 | 已修复 |
| 9 | `crates/babel-ebook/src/pipeline.rs` | 参数爆炸 | 调用方负担重 | `PipelineContext` 聚合 | 已修复 |

---

## 三、交互心智负担

| # | 位置 | 问题 | 影响 | 修复方案 | 状态 |
|---|---|---|---|---|---|
| 1 | `desktop/src/pages/*.tsx` | 每个设置页都要理解整个 `FormState` | 新增字段时需要检查所有页面 | 页面只接收相关 slice | 已修复 |
| 2 | `desktop/src/App.tsx` | 一个文件承载所有交互逻辑 | 新功能难以定位插入点 | 按关注点拆分 hooks | 已修复 |
| 3 | `desktop/src/App.css` | 全局样式查找困难 | 改样式不知影响哪些页面 | 组件/页面级 CSS | 已修复 |
| 4 | `crates/babel-ebook/src/config.rs` | 30+ 字段让配置使用者难以判断需要哪些 | 误用/传递错误字段 | 按用途分组为子结构 | 已修复 |
| 5 | `crates/babel-ebook/src/html.rs` | 单文件混合 DOM 处理与翻译流程 | 定位 bug 时需在大文件中跳转 | 按职责拆分子模块 | 已修复 |
