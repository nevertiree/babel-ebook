# BabelEbook 测试覆盖缺口清单

> 范围：Rust core (`crates/babel-ebook`)、Tauri 后端 (`desktop/src-tauri/src`)、desktop 前端与 E2E (`desktop/src`、`desktop/e2e`)。
> 生成时间：2026-07-12

## 一、Rust core / crates 覆盖缺口

### 1.1 模块覆盖矩阵

| 模块 / 功能 | 当前覆盖 | 说明 |
|------------|---------|------|
| `core.rs` — `CancellationToken`、进度事件、错误类型 | 部分覆盖 | 仅在端到端中间接执行，缺少 `ensure_not_cancelled`、`report_failures`、`estimate_source_tokens`、`translatable_chapters`、dry-run token 估算等直接单元测试。 |
| `core.rs` — `translate_epub` 端到端 | 部分覆盖 | `test_core.rs` 覆盖双语输出、dry-run、进度事件；缺少失败/部分失败路径、标题翻译失败处理。 |
| `pipeline.rs` — 有序并发管线 | 基本覆盖 | 内部测试覆盖跳过已完成、调度前取消、顺序、并发限制、死锁、checkpoint 持久化。 |
| `pipeline.rs` — checkpoint 集成 | 部分覆盖 | 仅在 `integration_new_features.rs` 中覆盖成功恢复路径；缺少 hash 不匹配、损坏 checkpoint、全部失败恢复。 |
| `worker.rs` — 专用 `!Send` worker 线程 | 部分覆盖 | 覆盖 dry-run、进度流、取消传播；缺少多任务顺序执行、关闭中提交、缓存任务、关闭后再提交。 |
| `checkpoint.rs` | 基本覆盖 | 覆盖往返与 job-id 确定性；缺少损坏 JSON、源 hash 不匹配、目录创建失败。 |
| `cache.rs` | 基本覆盖 | 覆盖往返、provider 隔离、清空、损坏 JSON、并发异步访问；缺少失效、禁用缓存、不可写目录、key 碰撞。 |
| `chunking.rs` | 完全覆盖 | 公共路径与空/空白/回退拆分边界均已测试。 |
| `config.rs` — 加载与默认值 | 基本覆盖 | 覆盖默认值、自定义项、prompts、glossary；缺少加载失败、cache/checkpoint 目录校验。 |
| `config.rs` — `Config::validate()` | 基本覆盖 | `test_validation.rs` 已覆盖大部分校验分支。 |
| `epub.rs` | 基本覆盖 | 覆盖往返、资源保留、嵌套目录、zip 回退、href 规范化、manifest-id 安全。 |
| `input_formats.rs` / `docx.rs` | 部分覆盖 | TXT/SRT/DOCX 端到端已覆盖；MOBI、unsupported-format、`html_or_xhtml` 分支未覆盖；DOCX 仅 happy path。 |
| `input_formats/srt.rs` | 完全覆盖 | 解析、CRLF、多行、异常块、无效时间行、HTML 转义均已测。 |
| `html/mod.rs` — `process_document` | 基本覆盖 | 覆盖输出模式、跳过代码、排除选择器、属性、scope、字体、source-lang、refine、取消。 |
| `html/selection.rs`、`html/insertion.rs` | 部分覆盖 | 仅通过 `process_document` 间接执行；缺少 `<br>` 规范化、scope 分支、`preserve_classes`+`<li>` 等直接测试。 |
| `html/translation.rs` — `translate_text` | 基本覆盖 | 覆盖缓存命中、refine、chunk 取消；缺少多 chunk refine、空译文、chunk 循环中缓存命中。 |
| `html/progress.rs` | 部分覆盖 | 缺少 `ChapterChunkAdapter` 事件重写的直接单元测试。 |
| `translator/registry.rs` | 基本覆盖 | 覆盖 provider 查找、大小写不敏感、自定义 model/tokens、API key 优先级；缺少 `providers` HashMap 回退、`openai-compatible` 构造。 |
| `translator/{openai,deepseek,anthropic,ollama}.rs` | 基本覆盖 | 覆盖默认值、不可达 list-models、max-tokens 快速失败、响应解析；缺少 `translate` 成功/失败/重试的 HTTP mock 测试。 |
| `translator/http_common.rs` | 基本覆盖 | 覆盖重试、model list 解析、错误格式化；缺少超时与真实 HTTP 响应解析。 |
| `escape.rs` | **未覆盖** | `xml_escape` / `html_escape` 纯工具函数零测试。 |
| `lib.rs` / i18n | 部分覆盖 | 仅语言加载冒烟测试。 |

### 1.2 高优先级未覆盖场景

1. **Queue / worker 并发与状态转换**（核心 crate 无 queue，queue 在 Tauri 层；worker 仍需补充）
   - worker 顺序提交多个 job、job 执行中 shutdown、shutdown 后再提交返回 `WorkerError::ShutDown`。
   - job 附带 `TranslationCache` 的缓存命中路径。
2. **Checkpoint resume / refine 边界**
   - 源文件 hash 不匹配时忽略旧 checkpoint。
   - `Completed` 但 `content=None` 的条目。
   - 首次运行全部章节失败后的 resume。
   - `refine=true` 时不重复运行已完成章节。
   - 损坏 checkpoint JSON 被当作全新任务处理。
3. **并发 chunk 翻译**
   - chunk N 与 N+1 之间的取消。
   - 多 chunk 文本中缓存命中。
   - refine 后文本超过 `max_refine_source_tokens` 的拆分。
4. **Cache 边界**
   - 目录无法创建时自动禁用缓存。
   - `put` 在不可写目录下不 panic。
   - 相同文本不同 provider 的 key 唯一性。
5. **HTML 转义与实体边界**
   - `escape.rs` 基本转义。
   - 译文包含 `<script>` 时按文本节点插入，不执行。
   - `translation_scope.tables=false` 跳过 `<td>`/`<th>`。
   - `translation_scope.metadata=false` 跳过 `title` 属性。
6. **Config / provider 边界**
   - `Config::load` 缺失文件、损坏 JSON。
   - `validate_cache_dir` / `validate_output_parent` 失败分支。
   - `provider_config` 覆盖 `base_url`、`model`、`max_tokens`、`temperature`。
7. **Pipeline 端到端错误处理**
   - 所有章节失败时 `translate_epub` 返回错误。
   - 部分章节失败时仍写出 EPUB，失败章节保留原文。
   - 不支持输入格式返回清晰错误。

### 1.3 建议新增 Rust 高优先级测试

| 目标文件 | 场景 | 优先级 |
|---------|------|--------|
| `tests/test_escape.rs`（新增） | `xml_escape`/`html_escape` 对 `& < > " '` | 高 |
| `tests/test_core.rs` | 全部章节失败返回错误；部分失败仍写出且失败章节原文保留 | 高 |
| `tests/test_core.rs` | 不支持格式 / 损坏 EPUB 返回清晰错误 | 高 |
| `tests/integration_new_features.rs` | hash 不匹配忽略旧 checkpoint；`refine=true` 跳过已完成；损坏 checkpoint 当作新任务 | 高 |
| `tests/test_html.rs` | 译文含 `<script>` 按文本插入；`scope.tables=false` 跳过表格；`scope.metadata=false` 跳过 title | 高 |
| `tests/test_worker.rs`（新增） | 顺序多个 job、shutdown 中提交、shutdown 后再提交、带缓存的 job | 中 |
| `tests/test_cache.rs` | 目录无法创建时禁用缓存；不可写目录 `put` 不 panic；provider+text key 唯一性 | 中 |
| `tests/test_config.rs` | `Config::load` 缺失/损坏；cache/output 父目录校验失败 | 中 |
| `tests/test_translator.rs` | `providers` HashMap 配置、`openai-compatible` 自定义 base URL、Ollama base URL 覆盖 | 中 |

---

## 二、Tauri 后端覆盖缺口

### 2.1 模块覆盖矩阵

| 模块 | 当前覆盖 | 说明 |
|------|---------|------|
| `queue.rs` (~690 行) | 部分覆盖 | 18 个单元测试覆盖入队/取消/重试/移除/重排/暂停/恢复、持久化往返、进度计算、暂停-恢复竞态；worker 循环、事件转发、大量错误路径未测。 |
| `commands.rs` (~525 行) | **几乎未覆盖** | 仅 `validate_connection_args` 的空 API key 分支有一个测试；所有 Tauri command 包装、`run_translation` 非 dry-run 路径、路径辅助函数、locale、checkpoint 列表未测。 |
| `task.rs` (~67 行) | **未覆盖** | `Task::new`、`TaskStatus` 序列化无测试。 |
| `config.rs` (~218 行) | 部分覆盖 | 仅系统 prompt 与 prompt 模板有测试；大部分字段映射、默认值、校验未测。 |
| `keyring.rs` (~45 行) | **未覆盖** | API key 存取删、`NoEntry` 回退无测试。 |
| `error.rs` (~65 行) | **未覆盖** | `Display` 格式化、`From<serde_json::Error>` 无测试。 |
| `lib.rs` (~247 行) | 部分覆盖 | 覆盖 dry-run 翻译、版本字符串、并发为 0 校验；`run()` 启动与参数 plumbing 未测。 |
| `args.rs` (~85 行) | **未覆盖** | 参数反序列化/默认值无测试。 |
| `main.rs` | 无需测试 | 入口点。 |

### 2.2 高优先级未覆盖场景

1. **Pause / resume / cancel 状态转换**
   - `finish_task` 的 `CompletedOrFailed` 分支（成功与失败）未测；仅 `Requeued` 分支被测。
   - 对非 pending/running 任务 `cancel` 返回 `CancelInvalidStatus`。
   - 对非 paused 任务 `resume_task` 返回 `ResumeInvalidStatus`。
   - 对非 terminal 任务 `retry` 返回 `RetryInvalidStatus`。
   - `pause_task` 错误路径：`NoRunningTask`、`NotCurrentTask`、`TaskNotRunning`。
2. **并发队列操作 / 事件转发**
   - `spawn_worker` 与完整 worker 循环无单元测试，依赖 E2E。
   - `TaskProgressCallback` 转发到异步进度通道。
   - `update_task_progress` 的 `ChunkFinished` 分支（章节内进度近似）。
3. **Command 错误向前端传播**
   - command wrapper `map_err(|e| e.to_string())` 未执行。
   - `run_translation` 的 `build_config` 失败、校验失败、translator 创建失败、worker drop 路径。
   - `check_file_exists`、`suggest_output_path`、`get_system_locale`、`list_checkpoints` 成功/失败路径。
4. **Config 映射与默认值**
   - `build_config` 对 `output_mode`、`style`、`translation_scope`、`provider_config`、token 限制、`cache_dir`、
     `checkpoint_dir`、`resume`、`translate_code`→`pre`/`code` 排除选择器的映射。
   - `build_test_config` 未测。
5. **Keyring 回退**
   - `load_api_key` 在 `keyring::Error::NoEntry` 时返回 `Ok(None)`。
   - 其他 keyring 错误传播。
   - `store_api_key` / `delete_api_key`。

### 2.3 建议新增 Tauri 后端高优先级测试

| 目标文件 | 场景 | 优先级 |
|---------|------|--------|
| `src-tauri/src/queue.rs` | `finish_task` 标记 Completed/Failed；`cancel`/`resume`/`retry` 非法状态错误；`pause_task` 错误路径；重排错误 | 高 |
| `src-tauri/src/queue.rs` | `ChunkFinished` 进度近似；`TaskProgressCallback::on_progress` 转发 | 高 |
| `src-tauri/src/commands.rs` | `suggest_output_path` 默认/自定义/空模板回退；`check_file_exists` true/false；`get_system_locale` | 高 |
| `src-tauri/src/commands.rs` | `validate_connection_args` 剩余分支；`get_default_prompts` 非空 | 中 |
| `src-tauri/src/commands.rs` | `list_checkpoints` 空目录/解析 JSON/忽略坏文件 | 中 |
| `src-tauri/src/config.rs` | `build_config` output_mode/style/scope/provider_config/cache/checkpoint/resume/translate_code 映射 | 高 |
| `src-tauri/src/config.rs` | `build_test_config` 默认值 | 中 |
| `src-tauri/src/task.rs` | `Task::new` 生成 Pending UUID；`TaskStatus` 序列化为 snake_case | 高 |
| `src-tauri/src/error.rs` | 各 `Display` 消息与 `serde_json::Error` 转换 | 中 |

---

## 三、Desktop 前端与 E2E 覆盖缺口

### 3.1 页面 / 功能覆盖矩阵

| 功能 / 页面 | E2E 状态 | 说明 |
|------------|---------|------|
| TranslatePage 主表单 | 部分覆盖 | 所有 spec 通过 `BABEL_EBOOK_E2E_SOURCE`/`OUTPUT` 注入路径；没有 spec 真正点击原生文件选择器。 |
| Start 翻译按钮 | 已覆盖 | 在 `translate.spec.ts`、`translate-failure.spec.ts`、`ollama-real-translation.spec.ts` 中点击。 |
| Dry-run 按钮 | **未覆盖** | `data-testid="dry-run-button"` 存在但从未点击。 |
| Provider / model 快速选择 | 部分覆盖 | model 下拉框在 `model-selection.spec.ts` 中验证；provider 切换未执行。 |
| Refine 复选框 | 已覆盖 | `translate-ui.spec.ts` 会切换。 |
| Checkpoint 列表 / resume | 部分覆盖 | `translate-ui.spec.ts` 检查可见性；未测试从列表选择 checkpoint。 |
| Output path 自动建议 | **未覆盖** | 由 `suggest_output_path` Tauri command 驱动。 |
| TasksPage / 队列 | 部分覆盖 | 队列级 pause/start 在 `translate.spec.ts` 中覆盖；per-task 暂停/恢复/取消、批量操作、重排、移除、错误详情弹窗未覆盖。 |
| Retry failed task | 已覆盖 | `translate-failure.spec.ts` 点击 `data-testid="retry-task"`。 |
| LogsPage | 部分覆盖 | 仅验证翻译后的条目数；搜索、级别过滤、复制、清空、滚动到底部未覆盖。 |
| Compute settings (providers) | 部分覆盖 | `compute-focus.spec.ts` 输入 provider 名称；增删 provider、设置 active、测试连接、自定义 base URL、key 可见切换未覆盖。 |
| Model params settings | **未覆盖** | token 限制、temperature 输入未执行。 |
| Translation settings | **未覆盖** | 源/目标语言、output mode、style、scope 复选框、排除选择器、翻译属性。 |
| Prompts settings | **未覆盖** | system prompt、各 style prompt、重置。 |
| Output settings | 部分覆盖 | checkpoint 目录可见；字体、文件名模板、目录选择器未覆盖。 |
| Queue settings | **未覆盖** | concurrency 输入校验/保存未测试。 |
| General settings | **未覆盖** | UI 语言、主题、跟随系统语言、导入/导出设置。 |
| AboutPage / LegalPage | **未覆盖** | 版本、主页/法律链接、许可证获取与返回。 |
| API key storage / keyring | 已覆盖 | `api-key-storage.spec.ts` 测试明文 key 迁移与 UI 输入 key 不写入 `settings.json`。 |
| Toast / 校验 / 错误流 | **未覆盖** | `ToastContainer` 与校验对象从未断言。 |

### 3.2 缺失的点击模拟覆盖

1. **通过 UI 选择输入 EPUB 文件**
   - 当前所有 spec 都绕过 `selectSource()` 对话框；`file-row-source` 点击 / “Select file” 按钮路径未执行。
2. **从 UI 启动翻译**
   - Start 按钮被点击，但都是在 env 注入 source/output/key 之后；校验驱动的启用/禁用、覆盖确认弹窗未测试。
3. **任务队列暂停 / 恢复 / 取消**
   - 队列级 pause/start 已覆盖。
   - per-task `pause-task`/`resume-task`/`cancel-task` 的 `data-testid` 存在但从未点击。
   - 批量工具栏（全选、批量 cancel/retry/remove）未执行。
   - pending 任务的重排上下箭头未执行。
4. **设置页导航与保存**
   - 仅打开 Compute/Providers tab；其他 tab 未导航。
   - autosave（500 ms debounce 到 `saveSettings`）仅在 `api-key-storage.spec.ts` 中间接断言。
5. **错误 Toast / 校验流**
   - 未断言 source/output/API key 缺失时 Start 按钮禁用。
   - 未断言后端错误时 `ToastContainer` 出现错误 toast、`appendError` 写入日志。

### 3.3 缺失的前端单元测试

- `desktop` 下无 `*.test.ts`/`*.test.tsx` 文件。
- 高价值候选：
  - `desktop/src/progress.ts` — `parseProgressPayload` 完备性与异常 payload。
  - `desktop/src/config.ts` — settings 迁移、keyring 迁移、`normalizeTheme`。
  - `desktop/src/utils.ts` — `generateId`。
  - `desktop/src/hooks/useQueue.ts` — refresh 与 command dispatch（需 Tauri mock）。
  - `desktop/src/hooks/useLogState.ts` — 进度事件到日志映射。
  - `desktop/src/components/ToastContainer.tsx` — 自动关闭。

### 3.4 建议新增 E2E 高优先级测试

| 目标文件 | 场景 | 优先级 |
|---------|------|--------|
| `e2e/tasks-controls.spec.ts`（新增） | 启动 dry-run，导航到 TasksPage，per-task 暂停/恢复/取消，验证状态变化 | 高 |
| `e2e/settings-navigation.spec.ts`（新增） | 打开 Settings，依次点击每个 tab 断言面板渲染；Model tab 修改 token 限制并断言 600 ms 后持久化；Queue tab 非法 concurrency 校验 | 高 |
| `e2e/translate-validation.spec.ts`（新增） | 无 source/output/key 时 Start/Dry-run 禁用；损坏 EPUB 触发错误 toast 与 `running-panel-error` | 高 |
| `e2e/translate-file-picker.spec.ts`（新增） | 不注入 env，mock/触发 Tauri 对话框选择 fixture，断言 source/output 路径更新并启动翻译 | 中 |
| `e2e/logs-page.spec.ts`（新增） | dry-run 后进入 Logs，搜索/级别过滤/复制/清空 | 中 |

---

## 四、本轮目标（高优先级补齐计划）

本轮聚焦以下可验证、用户高频触发的场景：

1. **Rust core 边界测试**
   - `escape.rs` 基本转义。
   - `core.rs` 全部章节失败 / 部分失败路径。
   - `html/` 译文含脚本标签的实体安全插入、scope 分支。
   - `worker.rs` 顺序多 job / shutdown 后再提交 / 带缓存 job。
   - `integration_new_features.rs` checkpoint hash 不匹配 / refine resume / 损坏 checkpoint。

2. **Tauri 后端边界测试**
   - `queue.rs`：非法状态错误（cancel/resume/retry/pause）、`finish_task` Completed/Failed、`ChunkFinished` 进度近似。
   - `task.rs`：`Task::new` 与 `TaskStatus` 序列化。
   - `commands.rs`：`suggest_output_path`、`check_file_exists`、`get_system_locale`、`list_checkpoints`。
   - `config.rs`：`build_config` 关键字段映射（output_mode、style、scope、provider_config、cache/checkpoint/resume、translate_code）。

3. **Desktop E2E 点击模拟测试**
   - `tasks-controls.spec.ts`：per-task 暂停/恢复/取消。
   - `settings-navigation.spec.ts`：设置页 tab 导航与 autosave。
   - `translate-validation.spec.ts`：校验与错误 toast。

4. **验证门禁**
   - `cargo test --workspace`
   - `cd desktop && pnpm exec tsc --noEmit && pnpm build`
   - 当 `target/release/babel-ebook-desktop.exe` 存在时运行 `pnpm e2e`。
