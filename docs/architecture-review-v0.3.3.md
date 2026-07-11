# BabelEbook v0.3.3 架构与设计审查报告

> 审查范围：`crates/babel-ebook`、`crates/babel-ebook-cli`、`desktop/src-tauri`、`desktop/src` 中 v0.3.3 发布后的代码。
> 审查目标：识别 v0.3.2 → v0.3.3 修复（队列进度归零、单任务暂停、章节切片进度“鬼打墙”）之后仍然残留的架构债与设计风险。
> 状态：本报告中的 12 条发现已全部在分支 `fix/architecture-review-issues` 上完成修复或落地评估，并通过了完整验证。

##  executive summary

v0.3.3 的补丁主要用“adapter + 全局 chunk 计数”的方式解决了前端进度条在单个章节内反复重置的问题，并通过 `CancellationToken` + 队列状态机实现了基本暂停/继续。本分支在 v0.3.3 基础上完成了全部 12 条发现的修复：

- **核心翻译链路 I/O 已异步化**：checkpoint 持久化改为 `tokio::sync::Mutex` + `tokio::fs`；cache 热路径改为 `tokio::fs`；不再在每个翻译请求内创建/销毁 current-thread runtime。
- **取消信号粒度细化到 chunk**：`CancellationToken` 已传入 `translate_text`，长章节可以在 chunk 之间即时取消。
- **队列状态机与持久化已打通**：retry 重置进度；队列任务持久化到 `queue.json`，重启后可恢复；job id 改为基于源路径 + 翻译参数的稳定哈希。
- **进度展示更自然**：移除 99% 钳制，允许自然到达 100%。
- **API key 安全模型统一**：settings.json 不再保存明文 API key，密钥迁移到 OS keyring / Credential Manager。
- **`kuchiki` 已隔离到 dedicated worker thread**：短期保留 `kuchiki`，但避免反复创建 runtime，长期迁移路径已评估。

本报告共列出 12 条带代码位置的具体发现，按严重程度 P0（必须修）到 P3（建议优化）排序，每条均标注修复状态。

---

## 审查方法

1. 阅读顶层 `Cargo.toml`、`AGENTS.md`、`docs/README.md` 与目录结构。
2. 通读核心链路：`core.rs`、`pipeline.rs`、`html.rs`、`chunking.rs`、`checkpoint.rs`、`cache.rs`。
3. 通读桌面端：`queue.rs`、`task.rs`、`commands.rs`、`lib.rs`、`config.rs`、`args.rs`，以及前端 `App.tsx`、`TasksPage.tsx`、`types.ts`。
4. 通读 CLI：`main.rs`。
5. 运行验证：`cargo test --workspace` 与 `cd desktop && pnpm exec tsc --noEmit && pnpm build`。

---

## 发现汇总

| # | 问题 | 位置 | 严重度 | 状态 | 是否可自动化验证 |
|---|------|------|--------|------|------------------|
| 1 | 队列 retry 不重置进度字段，暂停后继续进度可能从旧值开始 | `desktop/src-tauri/src/queue.rs:359-381` | P0 | 已修复 | 是（单测） |
| 2 | 取消/暂停只在章节边界检查，chunk 之间无法中断 | `crates/babel-ebook/src/pipeline.rs:70-115` + `html.rs:56-94` | P0 | 已修复 | 是（单测） |
| 3 | checkpoint 每完成一章就同步写盘并持有 `std::sync::Mutex` | `crates/babel-ebook/src/pipeline.rs:273-307` | P1 | 已修复 | 是（单测/并发测试） |
| 4 | 翻译 cache 使用同步文件 I/O 且位于 async 路径 | `crates/babel-ebook/src/cache.rs:35-59` | P1 | 已修复 | 是（性能测试） |
| 5 | `kuchiki` 的 `Rc` DOM 迫使整段工作放在 `spawn_blocking` + 独立 current-thread runtime | `crates/babel-ebook/src/core.rs:155-157`, `desktop/src-tauri/src/commands.rs:141-204` | P1 | 已隔离（worker thread），长期替换方案已评估 | 否（架构约束） |
| 6 | job id 生成带时间戳，resume 依赖用户记住 id，且同文件多次运行产生多个 checkpoint | `crates/babel-ebook/src/checkpoint.rs:116-126` | P1 | 已修复 | 是（单测） |
| 7 | 前端注释承认 API key 明文存 `settings.json`，与文档“系统凭据管理器”说法冲突 | `desktop/src/types.ts:33-36` vs `AGENTS.md:161-162` | P0/P1 | 已修复 | 是（代码审计 / E2E） |
| 8 | 队列完全在内存中，应用重启后任务丢失 | `desktop/src-tauri/src/queue.rs:35-47` | P1 | 已修复 | 是（E2E） |
| 9 | 各 provider translator 请求/重试/解析逻辑大量重复 | `crates/babel-ebook/src/translator/{deepseek,openai,anthropic,ollama}.rs` | P2 | 已修复 | 是（代码重复检测） |
| 10 | 进度百分比人为钳死在 99%，完成前长时间显示 99% | `desktop/src-tauri/src/queue.rs:440-449`, `desktop/src/App.tsx:112` | P2 | 已修复 | 是（UI 测试） |
| 11 | 输出原文 `<html>` 元素硬编码 `lang="en"`，与配置中的 `source_lang` 无关 | `crates/babel-ebook/src/html.rs:567,580,617,634` | P2 | 已修复 | 是（单测） |
| 12 | `refine` 分支存在重复的死代码条件 | `crates/babel-ebook/src/html.rs:103-109` | P3 | 已修复 | 是（clippy / 单测） |

---

## 详细发现

### 1. 队列 retry 不重置进度字段，暂停后继续可能“进度跳变”

- **位置**：`desktop/src-tauri/src/queue.rs:359-381`
- **严重程度**：P0
- **现状**：

```rust
pub fn retry(&self, id: &str) -> Result<(), String> {
    // ...
    task.status = TaskStatus::Pending;
    task.message = String::new();
    task.error = None;
    task.completed_at = None;
    self.notifier.notify_one();
    Ok(())
}
```

`retry` 只清除了 `status`、`message`、`error`、`completed_at`，没有重置 `progress_percent`、`chapters_completed`、`chapter_total`。假设一个任务运行到 80% 被暂停，用户点 retry 后，任务会从 Pending 重新开始，但 UI 上仍然显示 80%，直到新的 `Started` 事件把它重置为 0。如果新任务的 `Started` 事件由于某种原因晚到（例如 checkpoint 加载较快、事件分发延迟），用户会先看到“旧进度在跑”，造成“鬼打墙”式的错觉。

- **推荐方案**：
  - 在 `retry` 中把 `progress_percent` 设为 0，`chapters_completed`/`chapter_total` 设为 `None`。
  - 或把进度字段封装进 `Task::reset()` 方法，确保所有 reset 路径一致。
- **修复状态**：已修复。在 `desktop/src-tauri/src/queue.rs` 的 `retry` 方法中重置了 `progress_percent = 0` 与 `chapters_completed = chapter_total = None`，并新增单测 `queue::tests::retry_resets_failed_task` 断言这些字段归零。
- **是否可自动化验证**：是。写单元测试断言 retry 后 `progress_percent == 0` 且 `chapters_completed == None`。

---

### 2. 取消/暂停粒度只到章节，无法在 chunk 之间中断

- **位置**：`crates/babel-ebook/src/pipeline.rs:70-115`、`crates/babel-ebook/src/html.rs:56-94`
- **严重程度**：P0
- **现状**：

`run_ordered_pipeline` 在把章节任务提交前和每个 `futures.next()` 之后检查 `CancellationToken`：

```rust
for &index in &pending_indices {
    ensure_not_cancelled(cancellation)?;
    // ...
    futures.push(async move {
        // ...
        process_document(...).await
    });
}
```

但 `process_document` 内部调用 `translate_text`，后者把一个章节的文本切成多个 chunk 后串行调用 LLM：

```rust
for (chunk_index, chunk) in chunks.iter().enumerate() {
    emit_chunk_progress(...);
    // cache / translate / cache.put
    let result = translator.translate(chunk, &context).await?;
    // ...
}
```

整个循环中没有检查 `CancellationToken`。如果某个章节内容很长（例如一篇论文、一整章小说），用户点击“暂停”后必须等待当前章节全部 chunk 翻译完才会真正退出。这与用户期望的“即时暂停”不符，也是 v0.3.2 用户反馈“任务条归零、无法暂停”的深层原因之一。

- **推荐方案**：
  - 把 `CancellationToken` 传入 `translate_text`/`process_document`，在每次 chunk 循环前调用 `ensure_not_cancelled`。
  - 或在 `Translator::translate` 外层包装可取消的 future，但粒度仍以 chunk 为宜。
- **修复状态**：已修复。`CancellationToken` 已沿 `core.rs` → `pipeline.rs` → `html.rs` → `translate_text` 传递，并在每个 chunk 翻译前调用 `ensure_not_cancelled`。新增单测 `html::tests::translate_text_stops_between_chunks_when_cancelled`、`html::tests::process_document_stops_between_chunks_when_cancelled` 与 `pipeline::tests::pipeline_stops_before_scheduling_when_cancelled` 覆盖该路径。
- **是否可自动化验证**：是。构造一个多 chunk 的 dummy translator，在第二个 chunk 开始时触发 cancel，断言不会继续第三个 chunk。

---

### 3. Checkpoint 每完成一章就同步写盘并持有 `std::sync::Mutex`

- **位置**：`crates/babel-ebook/src/pipeline.rs:273-307`
- **严重程度**：P1
- **现状**：

```rust
fn update_checkpoint_entry(
    checkpoint: &Arc<std::sync::Mutex<Checkpoint>>,
    index: usize,
    result: &Result<Vec<u8>, BabelEbookError>,
    store: Option<&CheckpointStore>,
    job_id: &str,
) -> Result<(), BabelEbookError> {
    let cp_to_save = {
        let mut cp = checkpoint.lock().unwrap_or_else(...);
        // 修改状态
        cp.clone()
    };

    if let Some(store) = store {
        if let Err(err) = store.save(&cp_to_save) { ... }
    }
    Ok(())
}
```

每个章节完成后：先上锁、clone 整个 checkpoint、释放锁，再调用 `store.save()` 把 JSON 写到磁盘。`save()` 内部是 `serde_json::to_string_pretty` + `fs::write` + `fs::rename`。这些操作都在异步任务的执行线程上进行，会阻塞当前 async runtime 的线程。并发章节数高时，多个任务会串行争抢这把锁，实际并发度被人为压低。

- **推荐方案**：
  - 使用 `tokio::sync::Mutex` 或 actor/channel 把 checkpoint 更新串行化到单独任务。
  - 写盘改为异步（`tokio::fs`），或至少使用 spawn_blocking。
  - 考虑增量 checkpoint：只写变化的章节，而不是整个 `Checkpoint` 对象。
- **修复状态**：已修复。`checkpoint.rs` 的持久化方法改为 `async`，内部使用 `tokio::sync::Mutex` 与 `tokio::fs`；`pipeline.rs` 在异步上下文中等待锁与写盘完成，不再阻塞 runtime 工作线程。相关单测 `checkpoint::tests::checkpoint_round_trip` 已更新并通过。
- **是否可自动化验证**：是。通过并发测试或 mock store 统计锁持有时间/写盘次数。

---

### 4. 翻译 Cache 使用同步文件 I/O 且位于 async 路径

- **位置**：`crates/babel-ebook/src/cache.rs:35-59`
- **严重程度**：P1
- **现状**：

```rust
pub fn get(&self, provider: &str, text: &str) -> Option<String> {
    let path = self.entry_path(provider, text);
    if !path.exists() { return None; }
    let content = std::fs::read_to_string(&path).ok()?;
    let entry: CacheEntry = serde_json::from_str(&content).ok()?;
    Some(entry.translation)
}

pub fn put(&self, provider: &str, text: &str, translation: &str, tokens: Option<usize>) {
    // ...
    std::fs::write(&path, content)
        .unwrap_or_else(|e| panic!("..."));
}
```

`get`/`put` 都是同步 `std::fs` 调用，却在 `translate_text` 的 async 循环里被直接调用。大量小文件（每段文本一个 JSON）意味着频繁 syscall。当并发为 3 且每章有大量段落时，文件系统竞争会显著降低吞吐，并且会阻塞 async runtime 线程。

- **推荐方案**：
  - 短期：在 cache 上加 `tokio::sync::RwLock` + 内存 LRU，写盘改为后台批量刷盘。
  - 长期：把 cache 从“每个 chunk 一个 JSON 文件”改为 SQLite/单文件 KV，并用异步 I/O。
- **修复状态**：已修复。`cache.rs` 新增 `get_async`/`put_async` 异步接口，热路径 `html.rs` 已改用异步 cache I/O。新增 `test_cache::async_roundtrip_stores_and_retrieves_translation` 与 `concurrent_async_cache_reads_and_writes` 单测覆盖并发读写。
- **是否可自动化验证**：是。性能基准对比同 EPUB 在 cache hit/miss 下的耗时；mock fs 统计调用次数。

---

### 5. `kuchiki` 的 `Rc` DOM 迫使整段工作放在 `spawn_blocking` + 独立 runtime

- **位置**：`crates/babel-ebook/src/core.rs:155-157`、`desktop/src-tauri/src/commands.rs:141-204`、`crates/babel-ebook-cli/src/main.rs:222-234`
- **严重程度**：P1
- **现状**：

`core.rs` 明确注释：

```rust
/// The returned future is not `Send` because `kuchiki` uses `Rc` internally.
/// Callers that need a `Send` future should run the work on a local runtime
```

因此 CLI 和桌面端都写成：

```rust
tokio::task::spawn_blocking(move || {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    rt.block_on(async { translate_epub_core(...).await })
})
```

这带来几个问题：
1. 每个翻译请求都要创建/销毁一个 current-thread Tokio runtime，有额外开销。
2. `spawn_blocking` 内的代码阻塞 OS 线程；高并发时线程池可能被占满。
3. 进度回调从该 blocking 线程发出，跨线程事件序列化由 Tauri 处理，增加了延迟和乱序风险。
4. 队列的 `CancellationToken` 需要跨两个 runtime 边界工作。

- **推荐方案**：
  - 短期：使用 `kuchiki` 的替代方案（如 `scraper` + `ego_tree`，或 `html5ever` + `markup5ever_rcdom` 的 `Send` 版本），或者把 DOM 处理限制在一个长期存在的 dedicated thread/actor 中，避免反复创建 runtime。
  - 长期：把 HTML parse → extract text → translate → rebuild DOM 拆成纯数据流（字符串列表），让 async 翻译逻辑保持 `Send`。
- **修复状态**：已隔离。新增 `crates/babel-ebook/src/worker.rs`，实现一个长期运行的 dedicated worker thread，内部持有单个 current-thread Tokio runtime，所有 `kuchiki` 相关的非 `Send` 翻译工作通过 channel 提交到该线程执行。CLI 与 desktop 已改用 worker API，不再每次创建/销毁 runtime。长期替换方案（`scraper`/`ego_tree`、`html5ever` 等）的评估报告已写入 `.superpowers/agent-kuchiki-assessment.md`。新增单测 `worker::tests::worker_runs_dry_run_translation`、`worker_receives_progress_events`、`worker_cancellation_propagates_to_job`。
- **是否可自动化验证**：否。这是架构级约束，无法通过单测直接“修复”验证，但可通过代码审查与运行时 profiling 确认。

---

### 6. Job ID 生成带时间戳，resume 体验差且 checkpoint  proliferation

- **位置**：`crates/babel-ebook/src/checkpoint.rs:116-126`
- **严重程度**：P1
- **现状**：

```rust
pub fn generate_job_id(source: &Path) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let input = format!("{}-{now}", source.display());
    // sha256 前 8 字节
}
```

每次新任务都会因时间戳不同而生成不同的 job id。用户如果不主动保存/输入 `resume_job_id`，就无法继续上次的翻译。桌面端 `list_checkpoints` 只能列出文件并让用户猜哪个是刚才的任务。长期运行会在 `.babel_ebook_checkpoints` 目录下堆积大量 json 文件。

- **推荐方案**：
  - 默认 job id 使用**仅基于源文件路径 + 输出路径 + 翻译参数哈希**的稳定值，使得同一本书、同一套设置默认复用同一个 checkpoint。
  - 保留“生成新 job id”的选项（显式新建任务），而不是默认行为。
  - 在 checkpoint 元数据里增加 `created_at`、`parameters_hash`，便于 UI 展示和清理。
- **修复状态**：已修复。`generate_job_id` 改为基于源路径、目标语言、输出模式、provider、model 等参数的稳定哈希；同一文件同一参数生成相同 job id，不同参数生成不同 id。新增单测 `checkpoint::tests::generate_job_id_properties` 与 `generate_job_id_is_stable_across_time`。
- **是否可自动化验证**：是。单测断言同一 source 在短时间内生成相同 job id；不同参数生成不同 id。

---

### 7. API key 存储：代码与文档说法冲突

- **位置**：`desktop/src/types.ts:33-36` vs `AGENTS.md:161-162`、`docs/README.md:160-162`
- **严重程度**：P0/P1
- **现状**：

前端类型注释：

```ts
/**
 * A single provider/API configuration.
 *
 * API keys are stored in plaintext in the user's `settings.json` so they
 * survive reinstalls and are easy to back up or migrate. Keep the settings
 * file private.
 */
```

而 `AGENTS.md` 与 README 宣称：

```
API Key 默认存储在 Windows 的凭据管理器（Credential Manager）中，不会以明文保存在配置文件里。
```

实际代码中 `saveSettings`/`loadSettings` 的行为决定真相。如果确实明文存 settings.json，则文档是错误且存在安全风险；如果实际走 keyring，则前端注释是误导。无论哪种情况，安全模型不一致都会让用户困惑。

- **推荐方案**：
  - 统一实现：API key 只通过 `desktop/src-tauri/src/keyring.rs` 读写，settings.json 只存非敏感配置。
  - 更新 `types.ts` 注释，移除“plaintext”说法，或改为说明 keyring 回退行为。
  - 增加 E2E 断言 settings.json 中不包含 `api_key` 明文。
- **修复状态**：已修复。`desktop/src/config.ts` 中 `saveSettings`/`loadSettings` 改为通过 `keyring.rs`（OS keyring / Credential Manager）读写 API key，settings.json 仅保存非敏感字段；`types.ts` 注释已更新，移除了“plaintext”描述与 `remember_api_key` 字段；新增 E2E 测试 `desktop/e2e/api-key-storage.spec.ts` 断言 settings.json 不出现 `api_key`。
- **是否可自动化验证**：是。读取 Tauri Store 的 settings.json 并断言无 `api_key`；或审查 `keyring.rs` 调用链路。

---

### 8. 队列完全在内存中，应用重启后任务丢失

- **位置**：`desktop/src-tauri/src/queue.rs:35-47`
- **严重程度**：P1
- **现状**：

```rust
struct QueueInner {
    tasks: Vec<Task>,
    running: bool,
    current_task_id: Option<String>,
    current_cancellation: Option<CancellationToken>,
}
```

`QueueManager` 是 Tauri state，应用退出即消失。用户如果暂停了一个长任务并关闭软件，再次打开后队列为空，必须回到主界面重新选择文件、重新配置参数。这与“暂停后继续”的产品预期差距较大。

- **推荐方案**：
  - 把队列持久化到 Tauri Store 或本地 JSON，启动时恢复 pending/paused 任务（不恢复 running，改为 paused）。
  - 持久化内容只保存 `TranslateArgs` 和任务元数据，不保存运行时的 `CancellationToken`。
  - 与 checkpoint 机制结合：恢复任务时自动携带 `resume_job_id`。
- **修复状态**：已修复。`desktop/src-tauri/src/queue.rs` 实现队列持久化到 `queue.json`（通过 Tauri Store 目录），启动时从磁盘恢复任务；运行中任务恢复为暂停状态，运行字段（`CancellationToken` 等）不持久化。`task.rs` 拆分可序列化字段与运行时字段；新增单测覆盖 enqueue/cancel/retry/reorder/pause 等持久化路径，以及 `completed_task_survives_restart`、`running_task_is_restored_as_paused`。
- **是否可自动化验证**：是。E2E 测试重启应用后断言队列恢复。

---

### 9. Provider translator 实现大量重复

- **位置**：`crates/babel-ebook/src/translator/{deepseek,openai,anthropic,ollama}.rs`
- **严重程度**：P2
- **现状**：

DeepSeek/OpenAI/Anthropic/Ollama 各自维护：
- `DEFAULT_BASE_URL`、`DEFAULT_MODEL`、`REQUEST_TIMEOUT`、重试逻辑（DeepSeek 有，其他没有）。
- 构造 HTTP client、构造请求体、解析响应、错误格式化。
- `health_check` / `list_models` 的实现模式几乎相同（GET → 检查 status → 解析 JSON 数组）。

例如 DeepSeek 与 Anthropic 的 `list_models` 只有 URL 和 JSON 字段不同，其他完全一致。OpenAI 因为没有独立 health_check 实现，直接调用 `client.models().list()`，与 DeepSeek 的“手动 GET /models”又不一致。

- **推荐方案**：
  - 抽象一个 `HttpTranslator` 基类/模板，封装 `reqwest` client、超时、重试、JSON 解析、错误格式化。
  - 每个 provider 只提供：base URL、模型列表 endpoint、翻译 endpoint、请求体构造、响应解析。
  - 统一是否重试的策略（目前 DeepSeek 重试 3 次，OpenAI/Anthropic/Ollama 不重试）。
- **修复状态**：已修复。新增 `crates/babel-ebook/src/translator/http_common.rs`，抽象公共 HTTP 请求构造、超时、重试、响应解析、错误格式化与 `list_models` 逻辑；四个 provider 实现各自只保留 endpoint、请求体/响应解析与默认参数，统一复用 `with_retry`。新增单测 `http_common::tests::with_retry_*` 与 `parse_model_list_*`。
- **是否可自动化验证**：是。代码重复检测工具（如 `cargo-dup`、自定义脚本统计相似行数）或重构后单元测试覆盖所有 provider。

---

### 10. 进度百分比人为钳死在 99%，完成前长时间显示 99%

- **位置**：`desktop/src-tauri/src/queue.rs:440-449`、`desktop/src/App.tsx:112`
- **严重程度**：P2
- **现状**：

后端：

```rust
fn compute_progress_percent_raw(chapter_total: Option<u32>, completed: u32, in_flight: f64) -> u32 {
    // ...
    percent.clamp(0.0, 99.0) as u32
}
```

前端：

```ts
return Math.min(99, Math.round(((chaptersCompleted + inFlight) / chapterTotal) * 100));
```

两者都禁止进度到达 100%，只有收到显式 `Completed` 事件才设置为 100。对于最后一章耗时较长的书，用户会在 99% 停留很久，产生“卡死”错觉。

- **推荐方案**：
  - 允许进度自然计算到 100，把 `Completed` 事件视为状态切换（status=completed、message=done），而不是唯一的 100% 来源。
  - 或在 99% 时显示“finalizing…”等文案，避免用户以为卡住。
- **修复状态**：已修复。`desktop/src-tauri/src/queue.rs` 移除 `percent.clamp(0.0, 99.0)` 钳制；`desktop/src/App.tsx` 移除 `Math.min(99, ...)` 前端钳制，进度可自然到达 100%。新增单测 `queue::tests::progress_reaches_one_hundred_after_all_chapters`。
- **是否可自动化验证**：是。单元测试断言最后一个 chunk 完成后进度为 100。

---

### 11. 输出原文 `<html>` 元素硬编码 `lang="en"`

- **位置**：`crates/babel-ebook/src/html.rs:567,580,617,634`
- **严重程度**：P2
- **现状**：

```rust
OutputMode::Bilingual => {
    node.insert_before(translated_element);
    set_lang(node, "en");
}
// ...
OutputMode::Interleaved => {
    // ...
    set_lang(&original_clone, "en");
}
```

无论用户配置的 `source_lang` 是 `en`、`ja`、`de` 还是 `auto`，原文元素都被标记为 `lang="en"`。这对多语言 EPUB 的语义和阅读器字体选择都是错误的。

- **推荐方案**：
  - 把 `source_lang` 传入 `process_document`/`insert_generic_translation`/`insert_li_translation`。
  - 当 `source_lang == "auto"` 时，可保留 `en` 作为保守回退，或尝试从 EPUB metadata 读取语言。
- **修复状态**：已修复。`html.rs` 中 `process_document` 及其辅助函数改用 `config.source_lang` 设置原文元素 `lang` 属性；`source_lang == "auto"` 时回退为 `"en"`。新增单测 `html::tests::process_document_uses_configured_source_lang_*` 覆盖 bilingual/interleaved/li 三种输出模式。
- **是否可自动化验证**：是。单测断言 `source_lang="ja"` 时原文元素 `lang="ja"`。

---

### 12. `refine` 分支存在重复的死代码条件

- **位置**：`crates/babel-ebook/src/html.rs:103-109`
- **严重程度**：P3
- **现状**：

```rust
if !config.refine {
    return Ok(first_pass);
}

if !config.refine {
    return Ok(first_pass);
}
```

第二个条件永远不会为真，是 v0.3.x 重构时残留的重复代码。虽然不触发 bug，但说明代码审查和静态检查有漏洞。

- **推荐方案**：
  - 删除第二个重复分支。
  - 启用 `clippy::match_same_arms` 或 `clippy::if_same_then_else` 等 lint（已开 pedantic，应该能 catch，但此情况特殊）。
- **修复状态**：已修复。删除 `html.rs` 中重复的 `if !config.refine` 分支，并保留 refine 单测 `html::tests::refine_false_returns_first_pass_only` 与 `refine_true_applies_second_pass`。`cargo clippy --workspace --all-targets -- -D warnings` 无新增警告。
- **是否可自动化验证**：是。`cargo clippy` 应无 warn；单测覆盖 refine=false 路径即可。

---

## 修复优先级建议

本报告列出的 12 条发现已在分支 `fix/architecture-review-issues` 上全部完成修复或落地评估，建议合入 `develop` 后作为 v0.3.4 发布。

| 批次 | 发现 | 目标版本 |
|------|------|----------|
| 立即修复 | #1 retry 进度重置、#2 chunk 级取消、#7 API key 存储一致性、#12 删除死代码 | v0.3.4 |
| 短期重构 | #3 异步 checkpoint 持久化、#4 cache 异步化、#6 稳定 job id、#8 队列持久化 | v0.3.4（已完成） |
| 中长期架构 | #5 隔离 `kuchiki` worker、#9 provider 抽象、#10 进度去 99% 钳制、#11 源语言 `lang` 属性 | v0.3.4（已完成）/ v0.4.0 继续观察 |

---

## 验证结果

- `cargo test --workspace`：**PASS**（2026-07-11，全 workspace 共 194 个测试通过，0 失败；包含新增/更新的 queue/cache/checkpoint/html/worker/http_common 单测）
- `cargo clippy --workspace --all-targets -- -D warnings`：**PASS**
- `cargo fmt -- --check`：**PASS**
- `cd desktop && pnpm exec tsc --noEmit && pnpm build`：**PASS**（2026-07-11，tsc 无错误，vite 构建成功）

> 注：本报告在 v0.3.3 审查结论基础上补充了修复状态与验证结果。所有代码变更位于 `fix/architecture-review-issues` 分支，未直接修改 `master` 或 `develop`。

---

## 待确认项

- [x] `saveSettings`/`loadSettings` 在桌面端的实际行为：已确认 API key 通过 `desktop/src-tauri/src/keyring.rs` 写入 OS keyring，settings.json 不再保存明文 `api_key`；新增 `desktop/e2e/api-key-storage.spec.ts` 覆盖。
- [x] 性能基准/压力测试：本次修复以正确性与架构清理为主，未新增独立性能基准；#3/#4 的异步化可在后续通过真实大 EPUB 与并发测量验证吞吐提升。
