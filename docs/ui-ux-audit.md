# BabelEbook 桌面应用 UI/UX 审计报告

## 1. 审计范围与方法

- **审计对象**：`desktop/src` 前端页面与组件、`desktop/src-tauri/src/commands.rs` 后端命令、`desktop/e2e` 自动化测试用例。
- **审计维度**：信息架构、导航、表单交互、错误反馈、视觉层级、无障碍、文案一致性、首次使用体验。
- **审计方法**：代码静态审查 + 交互流程推演；结合 E2E 测试覆盖范围判断当前已验证与未验证的用户路径。
- **重要限制**：本次审计**未实际运行桌面应用进行视觉与动效验证**。所有结论均基于源码推导；涉及具体像素、动画、键盘焦点环、屏幕阅读器行为等结论，需在实际桌面环境中二次确认。

## 2. 执行摘要

BabelEbook 桌面应用的功能骨架已经相当完整：多供应商配置、任务队列、检查点恢复、日志、多语言、主题切换等核心能力均已实现。然而从专业 UI/UX 视角看，当前界面仍处于“功能可用”阶段，距离“产品级易用”还有明显差距：

1. **关键反馈缺失**：主流程中的必填校验错误（尤其是 API Key）未在前端可视化呈现，用户会在“开始翻译”按钮被禁用的情况下找不到原因。
2. **流程断裂**：点击“开始翻译”后界面立即跳转到任务队列页，导致翻译页上的进度条与日志面板被架空，用户需要在两个页面之间来回切换才能掌握状态。
3. **配置入口过载**：供应商管理、模型参数、翻译选项、输出选项被平铺在侧边栏的 6 个设置页中，新用户难以建立“我要先做什么”的心智模型。
4. **半成品痕迹明显**：CSS 中残留 `summary-card`、`dry-run-toggle`、
   `connection-status`、`next-step` 等未使用样式；`add_to_queue`、
   `use_recommended_model` 等多语言键也未在 UI 中落地。
5. **无障碍与专业感不足**：显示/隐藏 API Key 使用 emoji（🙈/👁），工具提示依赖 CSS 伪元素，数字输入框仅靠 `min/max` 原生属性约束，缺少即时校验反馈。

## 3. 详细发现

### 3.1 高严重度

#### H1. 主流程校验错误不可见，CTA 禁用原因不明

- **严重度**：高
- **位置**：`desktop/src/App.tsx:469-491`（校验逻辑）、`desktop/src/pages/TranslatePage.tsx:306-323`（错误渲染）
- **问题**：`validation` 对象同时计算了 `errors.source`、`errors.output`、`errors.api_key` 和 `reason`，
  但 `TranslatePage` 只渲染了 `source` 与 `output` 两个内联错误。`api_key` 缺失时“开始翻译”按钮会被禁用，
  但用户看不到任何提示说明为何按钮不可用；`reason` 字段完全未被消费。
- **影响**：首次使用必须手动去“Compute”设置里填写 API Key，但回到翻译页后没有任何引导；用户容易误以为应用崩溃。
- **优化建议**：
  1. 在翻译页顶部或 CTA 区域统一展示“当前无法开始的阻塞原因”。
  2. 当 `active_provider` 的 `api_key` 为空且非 Ollama 时，显示可点击的提示条，直接跳转到对应供应商配置。
  3. 禁用按钮时通过 `title` 或工具提示说明原因；理想情况下使用 `aria-disabled` 并绑定说明文本。
- **重构步骤**：
  1. 在 `TranslatePage` 顶部新增 `<ValidationBanner validation={validation} onFix={() => setPage("settings-compute")} />`。
  2. 在 `App.tsx` 的 `validation` 对象中增加 `field` 到 `route` 的映射（`api_key` → `settings-compute`，`source/output` → 本页）。
  3. 删除或消费 `validation.reason`，统一由 banner 组件渲染。
  4. E2E 补充用例：未填 API Key 时页面存在可见的阻塞提示。

#### H2. 点击“开始翻译”后页面跳转，进度与日志被架空

- **严重度**：高
- **位置**：`desktop/src/App.tsx:662-691`
- **问题**：`handleStart` 在入队并启动队列后执行 `setPage("tasks")`，但 `TranslatePage` 本身已经包含进度条与日志面板。结果是：用户启动任务后立刻被切走，翻译页的进度组件几乎永远不会被看到；用户需要自行切到“Tasks”页才能看进度，再切到“Logs”页看详细日志。
- **影响**：核心工作流被割裂为三页（Translate / Tasks / Logs），增加认知负担；进度条组件形同虚设。
- **优化建议**：
  1. **默认保持在翻译页**，让翻译页成为“控制中心”：文件选择、配置摘要、启动、实时进度、日志全部在同一页完成。
  2. “Tasks”页改为“历史/队列管理”视角，展示多任务列表、暂停/重试/删除等批量操作。
  3. “Logs”页保留为全屏日志审计视图，但翻译页日志面板应支持展开/收起或跳转。
- **重构步骤**：
  1. 移除 `handleStart` 中的 `setPage("tasks")`。
  2. 在 `TranslatePage` 底部将进度条与日志面板合并为可折叠的“运行面板”。
  3. 当存在运行中任务时，在侧边栏“Tasks”入口显示状态徽标（running count / completed count）。
  4. 调整 E2E `translate.spec.ts`，验证启动后仍停留在翻译页且进度可见。

#### H3. 供应商配置界面信息密度过高，心智模型混乱

- **严重度**：高
- **位置**：`desktop/src/pages/ComputeSettingsPage.tsx:145-263`
- **问题**：每个供应商卡片在一行内堆叠了：供应商类型、配置名称、API Key、自定义 Base URL 开关、
  测试连接按钮、设为当前按钮、删除按钮。所有元素挤在 `provider-row compact` 中，宽度被硬编码为
  `120px`、`180px`、`260px` 等，窗口缩小时会溢出或截断。用户需要理解“配置名称”与“供应商类型”的区别，
  还要手动点击“设为当前”才能生效。
- **影响**：配置成本高；用户可能填了 Key 但没点“设为当前”，导致翻译时使用的是旧供应商。
- **优化建议**：
  1. 将“当前使用”改为单选逻辑，默认最后一个被编辑/新增的供应商为当前，或在翻译页直接展示当前供应商卡片并提供快捷切换。
  2. 把测试连接、删除等操作收入卡片工具栏，API Key 与 Base URL 独占一行。
  3. 为未配置完成的供应商显示“待完成”状态，阻止其被设为当前。
- **重构步骤**：
  1. 拆分 `provider-row compact` 为至少三行：基础信息行（类型 + 名称）、凭证行（API Key + 显示切换）、操作行（测试连接 + 删除 + 当前状态）。
  2. 移除硬编码像素宽度，改用 `minmax` / `flex` 布局并增加 `@media` 断点。
  3. 在 `ComputeSettingsPage` 顶部增加“当前使用”摘要卡片，减少用户寻找成本。
  4. 新增 `ProviderCard` 组件，统一单供应商的编辑/展示状态。

#### H4. 首次启动缺少引导，空 Key 状态无明确行动号召

- **严重度**：高
- **位置**：`desktop/src/App.tsx:275-282`、`desktop/src/pages/TranslatePage.tsx:290-297`
- **问题**：全新安装时应用会自动 seed 一个 `deepseek` 供应商，但 API Key 为空。
  `TranslatePage` 只在 `providers.length === 0` 时显示空状态；seed 后 providers 非空，空状态不显示，
  用户面对一个禁用按钮和没有提示的页面。
- **影响**：首次使用留存率受损；用户不知道下一步该做什么。
- **优化建议**：
  1. 若检测到当前供应商缺少 API Key，在翻译页中央显示“配置 API Key”引导卡片，点击后直接打开 Compute 设置并聚焦到对应输入框。
  2. 首次启动可考虑一次性引导（Wizard）流程：选择源文件 → 选择/配置供应商 → 确认目标语言 → 开始。
- **重构步骤**：
  1. 新增 `FirstRunHint` 组件，判断条件：
     `providers.length === 0 || activeProvider?.api_key === "" && activeProvider.provider !== "ollama"`。
  2. 在 `TranslatePage` 的 `!hasProviders` 分支之后增加该提示。
  3. 在 E2E 中新增 `first-run.spec.ts`：删除设置文件后启动应用，验证引导可见。

#### H5. 进度与任务状态分散，缺乏统一状态中心

- **严重度**：高
- **位置**：`desktop/src/App.tsx:242-245`、`desktop/src/pages/TranslatePage.tsx:453-467`、`desktop/src/pages/TasksPage.tsx:32-102`
- **问题**：应用同时维护 `progress` 状态（单任务进度百分比与消息）和 `queue.tasks` 数组（多任务状态）。
  两者数据来源不同但语义重叠；`progress` 状态在 `TranslatePage` 展示，`queue` 在 `TasksPage` 展示。
  失败任务的错误信息只出现在 `TasksPage` 的 `inline-error`，而日志页又有一份日志。
- **影响**：用户需要同时关注三个地方才能确认“这次翻译到底怎么样了”。
- **优化建议**：
  1. 统一状态模型：当前运行任务由 `queue.current_task_id` 驱动，`progress` 派生自该任务的 `chapter_progress`。
  2. 翻译页只展示“当前任务”的实时进度与最近日志；历史任务全部归入 Tasks 页。
  3. 失败时弹出可关闭的错误详情卡片（或 drawer），而非仅依赖任务列表中的小红字。
- **重构步骤**：
  1. 在 `App.tsx` 中新增 `const currentTask = queue.current_task_id ? queue.tasks.find(...) : undefined`。
  2. 让 `TranslatePage` 接收 `currentTask` 而非独立的 `progress`，进度条宽度取自 `currentTask.progress_percent`。
  3. 移除 `progress` 相关 state 与 `translation_progress` 监听中的重复计算，仅保留日志追加逻辑。

---

### 3.2 中严重度

#### M1. 存在未落地的半成品 UI（死代码与未使用的多语言键）

- **严重度**：中
- **位置**：`desktop/src/App.css:357-405`（`.summary-card`、`.dry-run-toggle`）、
  `desktop/src/locales/*.json`（`add_to_queue`、`use_recommended_model`、`ready_to_start` 等）、
  `desktop/src/App.tsx:727-763`（`enqueueTask` 被 `void enqueueTask` 屏蔽）
- **问题**：CSS 中保留了 `.summary-card`、`.dry-run-toggle`、`.connection-status`、`.next-step` 等完整样式，
  但源码中没有任何组件使用。多语言文件里存在 `add_to_queue`、`use_recommended_model` 等键却未在 UI 落地。
  `App.tsx` 中声明了 `enqueueTask` 却刻意用 `void enqueueTask` 禁用。
- **影响**：代码库可信度下降，维护者难以判断哪些功能是“已完成但未启用”还是“已废弃”。
- **优化建议**：
  1. 决定“添加到队列”是否正式支持；若支持，在翻译页增加该按钮并移除 `void enqueueTask`。
  2. 决定“推荐模型”提示是否保留；若保留，在 `ModelSelect` 无匹配模型时展示。
  3. 清理所有未使用样式与多语言键，或将其明确标记为 TODO。
- **重构步骤**：
  1. 运行 `pnpm exec tsc --noEmit` 与 `pnpm build`，确保删除死代码不会破坏构建。
  2. 使用 `grep -R "summary-card\|dry-run-toggle\|connection-status\|next-step" desktop/src` 确认无引用后删除对应 CSS。
  3. 对多语言键进行引用扫描，删除未使用键或补充对应 UI。

#### M2. 工具提示不可访问且交互脆弱

- **严重度**：中
- **位置**：`desktop/src/App.css:700-729`
- **问题**：帮助图标 `ⓘ` 通过 CSS `::after` 伪元素和 `data-tooltip` 属性显示提示，依赖鼠标 hover/focus。没有 `role="tooltip"`、`aria-describedby`，键盘用户在移动焦点后可能看不到提示；屏幕阅读器无法朗读。
- **影响**：辅助功能不达标；非鼠标用户难以理解参数含义。
- **优化建议**：
  1. 将工具提示实现为独立 `Tooltip` 组件，使用 `aria-describedby` 关联触发元素。
  2. 支持键盘聚焦（`Tab`）与 `Esc` 关闭。
  3. 对关键帮助信息考虑常驻展示或折叠说明，而非全部依赖 hover。
- **重构步骤**：
  1. 新增 `src/components/Tooltip.tsx`，使用 React state 控制显隐，监听 `focus/blur/mouseenter/mouseleave/escape`。
  2. 替换所有 `.field-info` 使用为 `<Tooltip content={...}><Icon /></Tooltip>`。
  3. 在 Playwright E2E 中增加键盘焦点测试：聚焦帮助图标后提示可见。

#### M3. API Key 显示/隐藏按钮使用 emoji

- **严重度**：中
- **位置**：`desktop/src/pages/ComputeSettingsPage.tsx:185-186`
- **问题**：按钮内容使用 `🙈` / `👁`，在不同平台、字体、屏幕阅读器下表现不一致；emoji 作为唯一语义载体不符合专业桌面应用标准。
- **影响**：视觉不统一，辅助功能差；可能在高对比度主题下难以辨认。
- **优化建议**：使用 SVG 图标（eye / eye-off）并配合 `aria-label={showKey ? t("hide") : t("show")}`。
- **重构步骤**：
  1. 在 `public/` 或内联 JSX 中引入 `EyeIcon` / `EyeOffIcon`。
  2. 替换 `input-toggle` 按钮内容为图标，保留 `title` 与 `aria-label`。
  3. 确保高对比度主题下图标有足够对比度。

#### M4. 数字输入框允许越界值且缺少即时反馈

- **严重度**：中
- **位置**：`desktop/src/pages/ModelParamsPage.tsx:24-79`
- **问题**：`concurrency` 虽然设置了 `min={1} max={10}`，但用户仍可通过键盘输入 `0`、`100` 等值；
  HTML `min/max` 仅在提交表单时校验，React 立即将其写入 state。`max_input_tokens`、
  `max_output_tokens`、`temperature` 同样没有范围校验。
- **影响**：可能导致翻译时触发后端校验失败，用户体验为“配置了很久但开始时报错”。
- **优化建议**：
  1. 在 `setForm` 前进行数值截断或即时显示越界提示。
  2. 对 `temperature` 使用 `0-2` 步进 `0.1` 的滑块或数字输入组合，并展示“稳定—创造性”语义标签。
  3. 在 `App.tsx` 的 `validation` 中增加模型参数越界检查。
- **重构步骤**：
  1. 新增 `clamp(value, min, max)` 工具函数。
  2. 在 `ModelParamsPage` 的 `onChange` 中调用 `clamp` 并保留用户原始输入以显示临时错误（或使用受控组件即时修正）。
  3. E2E 增加参数边界测试：输入 `concurrency=0` 后页面展示错误。

#### M5. “Compute” 与 “Clear custom prompts” 文案 misleading

- **严重度**：中
- **位置**：`desktop/src/locales/en.json:125`、`desktop/src/locales/zh-CN.json:125`（`settings_compute`）；`desktop/src/pages/PromptsPage.tsx:65-73`、`desktop/src/pages/PromptsPage.tsx:128-134`
- **问题**：
  - “Compute（算力）”对普通用户是技术黑话；该页面实际管理的是“供应商/API 配置”。
  - “Clear custom prompts（清空自定义提示词）”按钮实际调用 `get_default_prompts` 并将默认值回填，即“重置为默认”，而非清空为空字符串。
- **影响**：用户可能误解功能语义，导致误操作。
- **优化建议**：
  1. 将 `settings_compute` / `nav_settings` 子项改为“Providers”/“供应商”。
  2. 将按钮文案改为“Reset to defaults”/“恢复默认提示词”。
  3. 若确实需要“清空”，则提供“清空”与“恢复默认”两个明确选项。
- **重构步骤**：
  1. 更新所有 `locales/*.json` 中对应键。
  2. 将 `handleReset` 重命名为 `handleResetToDefaults`，按钮文案使用新键。
  3. 在 E2E 中验证按钮文案语义。

#### M6. 源/目标语言在多处重复出现，配置所有权模糊

- **严重度**：中
- **位置**：`desktop/src/pages/TranslatePage.tsx:246-287`、`desktop/src/pages/TranslationSettingsPage.tsx:34-92`
- **问题**：源语言、目标语言、输出模式在翻译页的快速设置和 TranslationSettings 页同时存在。用户不确定哪一处是“权威”设置；修改一处后另一处同步，但心理模型是“两处都在管同一件事”。
- **影响**：设置页显得冗余；翻译页快速设置区被不重要选项占据。
- **优化建议**：
  1. 翻译页只保留**与本次翻译强相关**的快捷项：源文件、目标文件、目标语言、输出模式、当前供应商/模型。
  2. 源语言、翻译风格、排除选择器等高级选项全部集中到 TranslationSettings 页。
  3. 在翻译页提供“配置摘要”卡片，点击可快速跳转到对应设置页。
- **重构步骤**：
  1. 移除 `TranslatePage` 中的 `source_lang` 选择器，仅保留 `target_lang` 与 `output_mode`。
  2. 启用 `App.css` 中已有的 `.summary-card` 样式，在 `TranslatePage` 渲染当前关键配置摘要。
  3. 摘要卡片中每个字段均可点击跳转对应设置页。

#### M7. 日志页自动滚动干扰阅读，且缺少级别过滤

- **严重度**：中
- **位置**：`desktop/src/pages/LogsPage.tsx:28-30`、`desktop/src/components/LogPanel.tsx:14-16`
- **问题**：每次日志更新都会自动 smooth scroll 到底部。若用户在查看历史日志，新消息会强行将其拉回底部。日志页仅支持搜索，不支持按 `info/chapter/success/error` 过滤。
- **影响**：长任务运行时无法安心查阅日志；排查错误效率低。
- **优化建议**：
  1. 自动滚动只在用户已经位于底部时触发；若用户手动向上滚动，则暂停自动滚动并显示“新消息”提示。
  2. 增加日志级别过滤按钮组（全部 / 信息 / 章节 / 成功 / 错误）。
  3. 提供“跳转到最新”快捷按钮。
- **重构步骤**：
  1. 在 `LogsPage` 增加 `isScrolledToBottom` ref，通过 `scroll` 事件监听。
  2. 修改 `useEffect` 仅在 `isScrolledToBottom` 为 true 时调用 `scrollIntoView`。
  3. 新增 `LogLevelFilter` 组件，过滤 `filtered` 数组。

#### M8. 任务队列缺少排序、批量操作与空状态优化

- **严重度**：中
- **位置**：`desktop/src/pages/TasksPage.tsx:36-99`、`desktop/src-tauri/src/commands.rs:409-417`（`reorder_tasks` 命令存在但无 UI）
- **问题**：后端已支持 `reorder_tasks`，但前端没有拖拽或上下移动控件。任务操作只有单个按钮，无批量暂停/删除/重试。队列无任务时仅显示一行文字，没有引导去添加任务。
- **影响**：多文件翻译场景效率低；用户无法调整执行顺序。
- **优化建议**：
  1. 为 pending 任务提供拖拽排序或“上移/下移”按钮。
  2. 增加任务复选框与批量工具栏（暂停/删除/重试）。
  3. 空队列时展示“拖拽 EPUB 到这里”或“前往翻译页添加任务”的引导。
- **重构步骤**：
  1. 引入轻量级拖拽库（如 `@dnd-kit/sortable`，需先确认是否允许新增依赖）或手写拖拽实现。
  2. 在 `TasksPage` 中调用 `invoke("reorder_tasks", { ids })`。
  3. 新增批量选中状态与工具栏。

#### M9. 许可证页面使用 `<pre>` 展示，可读性差

- **严重度**：中
- **位置**：`desktop/src/pages/LegalPage.tsx:45-47`
- **问题**：许可证文本使用 `<pre>` 标签原样展示，没有合适的行宽、字体、标题层级；长段落难以阅读。
- **影响**：法律文本阅读体验差，显得不专业。
- **优化建议**：
  1. 将许可证文本解析为 Markdown/HTML 后渲染，使用合适的段落、标题与列表样式。
  2. 若保持纯文本，也应使用 `<article>` + 段落拆分，而非 `<pre>`。
- **重构步骤**：
  1. 将 `LICENSE` 文件复制为 `public/legal/LICENSE.md`（build 脚本已做类似工作）。
  2. 使用轻量 Markdown 解析器渲染，或按空行分段渲染为 `<p>`。

---

### 3.3 低严重度

#### L1. 异步操作缺少 loading / skeleton 状态

- **严重度**：低
- **位置**：`desktop/src/pages/ComputeSettingsPage.tsx:89-119`、`desktop/src/pages/TranslatePage.tsx:22-122`
- **问题**：测试连接时按钮文案从“Test Connection”变为“Testing...”，但没有 spinner；`list_models` 调用期间模型下拉框仅显示 `disabled`，无加载提示。
- **影响**：网络较慢时用户不确定操作是否生效。
- **优化建议**：
  1. 为按钮增加 `aria-busy` 与旋转图标。
  2. 模型下拉框 loading 时显示“Loading models...”占位选项。
- **重构步骤**：
  1. 新增 `LoadingSpinner` 组件。
  2. 在 `ComputeSettingsPage` 与 `ModelSelect` 中替换纯文案 loading 状态。

#### L2. 设置页平铺于侧边栏，导航层级扁平化过度

- **严重度**：低
- **位置**：`desktop/src/App.tsx:47-54`、`desktop/src/App.tsx:880-892`
- **问题**：6 个设置页直接作为导航项平铺，导致侧边栏较长；设置之间没有分组标签页，用户难以快速定位。
- **影响**：随着设置项增多，侧边栏会越来越长；新用户需要逐个点开才能理解结构。
- **优化建议**：
  1. 侧边栏只保留“Settings”一级入口，内部使用 Tab 或锚点分组（Provider、Model、Translation、Output、Prompts、General）。
  2. 保留直接路由能力（如 `settings/compute`），便于外部跳转。
- **重构步骤**：
  1. 新增 `SettingsLayout` 组件，内含子导航。
  2. 修改 `App.tsx` 的 `settingsPages` 渲染逻辑。

#### L3. 翻译前缺少“预览/估算”入口

- **严重度**：低
- **位置**：`desktop/src/pages/OutputSettingsPage.tsx:33-39`、`desktop/src/App.tsx:157-165`（后端 dry_run 已支持）
- **问题**：`dry_run` 能力已存在，但 UI 把它作为一个普通 checkbox 藏在 Output Settings 中。用户可能不知道它能做什么。
- **影响**：用户无法在开始真实翻译前快速估算 token/章节数，增加试错成本。
- **优化建议**：
  1. 在翻译页 CTA 区域提供“估算（Dry Run）”按钮，点击后仅运行 dry run 并展示结果。
  2. 将 `dry_run` 从 OutputSettings 中移除或改为“默认开启估算模式”的高级选项。
- **重构步骤**：
  1. 在 `TranslatePage` 的 `start-row` 中增加 `onDryRun` 按钮。
  2. 复用 `buildTranslateArgs` 并将 `dry_run: true` 传给 `invoke("enqueue_task")` / `start_queue`。
  3. 在结果返回后展示估算卡片（章节数、预计 token）。

#### L4. 关于页返回按钮使用未定义样式

- **严重度**：低
- **位置**：`desktop/src/pages/LegalPage.tsx:36`
- **问题**：按钮使用 `className="btn-secondary"`，但 `App.css` 中只定义了 `.legal-header button`，没有 `.btn-secondary`。
- **影响**：样式回退到全局 `button` 样式，视觉上可能与其他按钮不一致；代码维护性差。
- **优化建议**：统一按钮变体类名（`button-secondary` 或 `btn-secondary`），并全局定义。
- **重构步骤**：
  1. 在 `App.css` 中定义 `.btn-secondary` / `.button-secondary`。
  2. 统一所有文件中的类名引用。

#### L5. 翻译页文件选择器允许多种输入格式但后端能力需确认

- **严重度**：低
- **位置**：`desktop/src/pages/TranslatePage.tsx:180-196`
- **问题**：源文件选择器过滤了 `epub, mobi, azw3, txt, srt, docx`，但核心库名称是 `babel-ebook`，主要宣传为 EPUB 翻译。其他格式的支持程度与错误处理需要确认。
- **影响**：若后端对某些格式支持不完整，用户会在选择文件后翻译失败。
- **优化建议**：
  1. 在 UI 上明确标注“主要支持 EPUB；其他格式为实验性”。
  2. 选择非 EPUB 文件时给出警告提示。
  3. 或在后端返回格式不支持时提供清晰的错误信息。
- **重构步骤**：
  1. 在 `selectSource` 中根据扩展名弹出水印提示。
  2. 更新 `locales/*.json` 中 `ebook_files` 与错误提示。

## 4. 优先级路线图

### P0 — 立即修复（影响核心可用性）

1. **H1**：在翻译页展示完整的校验错误，特别是 API Key 缺失提示。
2. **H2**：移除“开始翻译”后的自动跳转，让翻译页成为控制中心。
3. **H4**：新增首次使用/空 Key 引导卡片。
4. **H3**：重构供应商配置卡片，降低信息密度并明确“当前使用”逻辑。

### P1 — 近期优化（显著提升体验）

1. **H5**：统一进度与任务状态模型，减少信息分散。
2. **M6**：重新划分翻译页与 TranslationSettings 页的职责。
3. **M5**：修正“Compute”与“Clear custom prompts”文案。
4. **M7**：改进日志自动滚动与级别过滤。
5. **M4**：为数字输入增加范围校验与反馈。
6. **M2/M3**：替换 emoji 与不可访问的工具提示实现。

### P2 — 体验打磨

1. **M1**：清理死代码与未使用的多语言键。
2. **M8**：任务队列排序与批量操作。
3. **M9**：许可证页面可读性优化。
4. **L1/L2/L3/L4/L5**：loading 状态、设置页导航、dry run 入口、按钮样式统一、输入格式提示。

## 5. 推荐的最小可行重构顺序

若资源有限，建议按以下顺序执行，每步都可独立验证：

1. **修复校验反馈**（H1）：只需修改 `TranslatePage`，增加 `ValidationBanner`。
2. **修复页面跳转**（H2）：删除 `handleStart` 中的 `setPage("tasks")`。
3. **合并状态模型**（H5）：让 `TranslatePage` 使用 `currentTask` 替换 `progress`。
4. **重构供应商 UI**（H3）：提取 `ProviderCard` 组件，分三行布局。
5. **清理死代码**（M1）：删除未使用样式与多语言键，明确 TODO。
6. **文案修正**（M5）：更新键名与按钮行为。
7. **可访问性改进**（M2/M3）：引入 `Tooltip` 与 SVG 图标。
8. **日志体验**（M7）：可控自动滚动 + 级别过滤。

## 6. 待验证假设

以下结论基于源码，建议在实际运行中确认：

- 侧边栏在最小窗口尺寸下是否出现截断或滚动条。
- 高对比度主题下 emoji 与工具提示的实际可见性。
- 在较低分辨率屏幕上，`provider-row compact` 是否会导致水平滚动。
- 屏幕阅读器（NVDA/JAWS）是否能正确朗读当前供应商、进度百分比与日志错误。
- 新用户首次启动时，是否能独立找到 API Key 配置入口。

## 7. 结论

BabelEbook 的功能实现已经具备产品雏形，但当前 UI/UX 仍处于“开发者友好、普通用户困惑”的状态。
最紧迫的问题是**核心流程中的反馈缺失与页面断裂**：用户不知道按钮为何被禁用、开始翻译后被跳转到别处、
配置分散在多个设置页中。优先解决 H1-H5 后，应用的首次使用留存与日常操作效率将会有质的飞跃。
后续再逐步完成可访问性、文案、死代码清理等打磨工作，即可达到可公开推荐的产品水准。
