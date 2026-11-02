# AGENTS.md — 常驻开发规范

> 面向 AI 智能体与新加入开发者：本文件是每次任务都适用的常驻规则层（自动加载），只放「红线、边界与门禁」。
> 技术栈：Tauri v2 + SvelteKit 2 + Svelte 5 + Tailwind CSS v4 + shadcn-svelte + Paraglide + tauri-specta 的桌面应用模板；人类向导读见 `docs/README_zh-CN.md`。
> 优先级：安全红线 > 架构不变量 > 测试验证 > 代码风格。参考细节与「为什么」在按需文档中，按下方索引读取。

## 文档索引（做对应任务前先读）

| 你要做的事                                                 | 先读文档                                            |
| ---------------------------------------------------------- | --------------------------------------------------- |
| 放新文件 / 判断分层归属 / 目录职责 / 路径别名              | `docs/ARCHITECTURE.md`                              |
| 改 Rust 命令 / 前后端契约 / 重生成绑定                     | `docs/WORKFLOW.md` 的「命令契约流程」               |
| 改前后端文案 / 语言切换                                    | `docs/WORKFLOW.md` 的「国际化流程」                 |
| 动配置项 / schema 迁移 / 崩溃兜底 / locale 数据流 / 初始化 | `docs/ARCHITECTURE.md` 的「设计不变量与原因」       |
| 改外观 / 配色主题 / 布局 / 新页面视觉                      | `DESIGN.md`（视觉系统）+ `PRODUCT.md`（产品上下文） |
| 跑命令 / 依赖升级 / 构建产物 / 验证级别                    | `docs/WORKFLOW.md`                                  |
| 发版 / CI / 提交钩子细节                                   | `docs/WORKFLOW.md` 的「CI」与「验证矩阵」           |
| 派生新项目（改名 / 换协议 / 换密钥 / 换图标 / 文档收尾）   | `docs/INITIAL.md`                                   |

> 只有仓库根的 `AGENTS.md` 会被自动加载；上表文档按需读取——做对应任务时必须先读，不要依赖记忆。

---

## 安全红线

> 触碰以下任一条直接回滚，禁止讨论例外。

1. **禁止提交密钥：** `.env`、`*secret*`、`*key.pem`、Token、AK/SK 永不进仓库。Updater 私钥等只走环境变量 / 密钥管理服务。
2. **禁止修改项（无书面批准不得触碰）：**
   - `capabilities/*.json` 权限配置与 `tauri.conf.json` 的 CSP / updater 配置（新增外部 URL 必须同步两处）
   - 已发布的 GitHub Release / `v*` 标签对应的版本号（三处联动，见「提交规范」）
   - `.github/workflows/release.yml` 发布流水线
   - `bindings.ts` 生成物（只允许 `cargo test` 重生成，禁止手改）
3. **输入安全：** 所有外部输入必须校验 + 转义；Svelte 渲染默认转义，禁止 `{@html ...}` 直渲用户输入；Rust 侧字符串拼接 shell / SQL 时必须参数化。
4. **越权与能力最小化：** Tauri capability 按需最小授权，禁止全开 `*`；前端隐藏按钮不算权限控制，涉及本地文件 / 系统能力时后端必须二次校验。前端 `openExternal` 仅放行 `http(s)`（先剥 `git+` 前缀再校验），其它 scheme 直接拒绝并上报。`store` / `autostart` / `window-state` / `single-instance` / `clipboard-manager` / `global-shortcut` 仅 Rust 侧注册使用（见 `lib.rs`），无前端 JS 直调，故不在 `plugins.json` 声明，属刻意最小授权；新增前端直调时再按需补声明。
5. **依赖安全：** 禁止引入未知来源依赖；新增依赖必须说明理由；依赖审计（`pnpm audit` / `pnpm audit:backend`）为可选本地自查，不纳入 CI；依赖升级走手动 `chore(deps:update)` 提交，入口一律 `pnpm update:frontend | update:backend | update:all`（禁止裸 `pnpm update` / `cargo update`，前者是 pnpm 内建命令，会绕过脚本门禁）。

---

## 架构不变量

> 每条都是硬约束；机制细节与原因见 `docs/ARCHITECTURE.md` 的「设计不变量与原因」。

1. **分层单向依赖：** `Svelte 组件 → libs/commands（链式 API）→ commands/（Rust 薄封装）→ features/ · cores/`，禁止反向调用与跨层跳跃（如组件裸 `invoke`、命令层写业务逻辑）。纯前端 UI 偏好走 `localStorage` 是有意例外，不经 commands 链。
2. **前后端契约优先：** 改命令必须先改 `commands/`（有对应业务时先改 `features/` 对应业务文件）（`#[tauri::command]` + `#[specta::specta]` + `collect_commands!`），再 `cargo test` 重生成 `bindings.ts`，最后改前端调用。禁止手改 `bindings.ts`、禁止跳过重生成直接改前端。
3. **状态归属：** Svelte 5 runes（`$state` / `$props` / `$effect`）优先，禁用旧式 store；页面私有状态用局部 state，跨页面共享才考虑提升。主题经 `mode-watcher` 全局管理，语言归后端 `config.json`，禁止各页面自建。
4. **错误处理分层：** `features/` 返回业务错误 → `commands/` 转 `CommandResult`（错误统一 `CommandError::Internal(...)`，`anyhow::Error` 经已有 `From` 自动转换）→ 前端经链式 API 处理。关键取值用 `.result()` 按 `status` 分支（`.value()` 的回落会把失败与空数据压成同一个值）；回调内只做幂等赋值 / toast 且保证不抛错；多步分支 / 跨状态机流转必须 `result()` 后置分支。未注册 `.failed()` 的失败会自动经 plugin-log 上报，不会静默丢失；多个业务通用的逻辑抽取到 `cores/`，不散落在各业务文件中。
5. **配置唯一来源与迁移：** Paraglide 编译选项唯一来源 `src/libs/i18n/project.inlang/paraglide.config.ts`；路径别名唯一来源 `package.json` 的 `imports`；新增外部 URL 同步 `capabilities/*.json` 与 CSP；新增 / 变更配置项必须同步 `cores/config.rs` 的 `CURRENT_SCHEMA_VERSION` 与 `MIGRATIONS` 迁移链，配置写入只走 `update` / `save_locale` / `migrate` 三入口；新增 / 变更配置项时对照 `docs/ARCHITECTURE.md` 的「成对维护的配置」表。
6. **崩溃边界：** 前端 `ErrorBoundary` + 后端 panic 钩子（`cores/system.rs`，兜底落盘临时目录 `my_app_crash.log`，Win `%TEMP%` / Unix `$TMPDIR`）。两者互不替代，禁止另加并行机制。
7. **语言数据流：** 唯一持久化点是 `config.json` 的 `locale` 键（`Locale` 枚举，`"en" | "zh-CN"`，禁止裸 `string` 或类型断言）；切换先经 `commands.updateConfig` 落盘再改内存，按返回值分支（禁用状态相等代理推断）；前端渲染期文案经 `libs/i18n/locale.svelte.ts` 桥接响应式（`overwriteGetLocale` 读 `$state`），**不重载的语言变更只允许经 `alignLocale`**；`restart` 桌面端永不结算（勿 `await`）。完整时序见 `docs/ARCHITECTURE.md`。
8. **初始化与订阅：** `initAppearance` 由调用点包 `untrack` 后执行（`+layout.svelte` 的 `$effect.pre`；init 内读写同一批外观状态，被跟踪会形成「改动→重初始化」回路），init 本体保持幂等、可重复调用；`app.html` 首帧内联脚本与 init 的键名 / 分支必须同步改；`localStorage` 存量键（`layout-name` / `color-theme` / `font-family` / `font-weight` / `font-size` / `dashboard-collapsed` / mode-watcher 的 `mode-watcher-mode`）冻结豁免，新增键统一 `app.` 前缀。

---

## 代码放哪里

```text
src/          前端 SvelteKit SPA：routes/（页面与布局）· components/（common · layout · shadcn-svelte）· libs/（commands · i18n · hooks · navigation · shortcut · utils）· tests/
src-tauri/    后端 Rust：src/{commands, cores, features, plugins}/ · locales/ · capabilities/ · tauri.conf.json
docs/         文档（常驻规则 + 按需层）；static/ 原样拷贝资源；.github/workflows/ CI 与发布
```

（完整目录树与各目录职责见 `docs/ARCHITECTURE.md`。）

- 前端：页面专有组件放所属路由的 `_components/`（页面内相对导入）；仅跨页共享的手写组件放 `components/common/`；注册表类（路径、图标、顺序等应用知识）放 `libs/<领域>/`（如 `libs/navigation/`），`libs/utils/` 只放不含业务 / 应用知识的通用工具。
- 后端：`commands/` 只做薄封装（参数与结果转换 + 注册）；具体业务逻辑按业务类型拆文件进 `features/`（由 `commands/` 调用，不要求纯函数）；多业务通用逻辑抽取进 `cores/`；不另设 `utils/`。
- 布局子系统在 `components/layout/`（`layout-container` + tabs / sidebar / dashboard + `parts/`）；新增布局类型必须在 `libs/hooks/appearance.svelte.ts` 的注册表中登记。
- 生成物禁手改：`bindings.ts`（`cargo test` 重生成，需要提交）、`paraglide/`（`i18n:compile` 重生成，禁编辑、禁从外部导入其内部文件）。

---

## 工作流（SOP）

```text
读 AGENTS.md（+ 索引指向的文档）→ 定位目录 / 分层 → 小步修改 → 补生成步骤 →
pnpm format → pnpm validate → 自查安全红线 → 汇报修改结果 → 给出下一步建议
```

**任务开始前：**

1. 通读本文件，并按文档索引读取对应任务文档；
2. 用 `git status / git diff` 确认工作区干净，从最新 `main` 拉分支 `feat/* | fix/* | chore/*`。

**任务执行中：**

1. 严格按分层与边界写代码（`docs/ARCHITECTURE.md`）；
2. 按改动类型补齐生成步骤：改 `messages/` → `i18n:compile`；改后端命令或逻辑 → `cargo test`（重生成绑定）；
3. 每完成一个小步即 `pnpm format` + `pnpm validate`，红灯立即停下修复。

**任务收尾：**

1. 跑 `docs/WORKFLOW.md` 验证矩阵中对应级别的完整验证；
2. 自查安全红线；
3. 汇报修改结果并给出下一步建议。

**硬性规则：** 任何文件改动完成后必须先 `pnpm format`，再 `pnpm validate`；两者全绿前不得宣称完成、不得提交。钩子只覆盖暂存文件，是最后防线而非替代。

---

## 禁令速查

| 禁止                                                                     | 正确做法                                                                     |
| ------------------------------------------------------------------------ | ---------------------------------------------------------------------------- |
| 组件内裸 `invoke`                                                        | 走 `libs/commands` 链式 API（`.value() / .result() / .success()/.failed()`） |
| 手改 `bindings.ts` / `paraglide/`                                        | 改 Rust 后 `cargo test` 重生成；改文案后 `i18n:compile`                      |
| 改完文案不跑 `i18n:compile`                                              | 必跑，否则 `build` / CI 失败                                                 |
| 往 `svelte.config.ts` / `vite.config.ts` 复制别名，或复制 Paraglide 参数 | 各认唯一来源：`package.json` 的 `imports` / `paraglide.config.ts`            |
| TSDoc 里写代码示例 / 描述以英文小写开头                                  | 示例放 `//` 注释块；描述用中文开头                                           |
| 提交前不跑 `pnpm format` + `pnpm validate`                               | 全绿再推；钩子只看暂存文件                                                   |
| `shadcn-svelte add` 遇到覆写提示一路回车                                 | 逐项拒绝覆写，事后 `git diff` 回退无关文件                                   |
| 密钥 / Token 进仓库，硬编码 URL / 密钥 / 本地绝对路径                    | 只走环境变量；外部 URL 同步 capabilities + CSP                               |
| 裸 `pnpm update` / `cargo update`                                        | 走 `pnpm update:frontend \| update:backend \| update:all`                    |

---

## 代码风格（仅非机器强制部分）

> Prettier / ESLint / clippy 规则会被 `pnpm validate` 机器拦截，以配置文件为准，本文件不复述（格式化与导入顺序见 `.prettierrc`）。

- 命名：文件 `kebab-case`，类 / 组件 `PascalCase`，函数 / 变量 `camelCase`，常量 `UPPER_SNAKE_CASE`，Rust 模块 `snake_case`。
- 语言：注释用中文（专有名词与包名保留英文）；变量 / 函数名与提交信息一律英文。
- 注释只解释 Why，不复述 What；Rust 文档注释（`//!` / `///`）写模块职责与契约，标识符加反引号。
- 日志：关键分支打 `info`，异常打 `error`，崩溃走 panic 钩子（见「架构不变量」6）；禁止打印密钥 / Token / 完整 PII。
- 枚举落盘值用语义名，禁用 `default` / `other` 之类只表达「被选中」的占位名。
- 函数尽可能 ≤ 50 行、圈复杂度 < 10，超长按逻辑拆分。
- TSDoc（`/** */`，短句单行）只写契约与非直观处；不写代码示例；描述别用英文小写标识符开头。
- shadcn-svelte 组件尽可能全名引用（如 `AlertDialogTrigger`），仅命名空间导出的原语（如 `Tooltip`）豁免。
- Svelte 组件只用 runes；Props 显式定义类型，禁止 `any` 透传。
- 时间：业务全链路 UTC，展示层转本地时区；禁止无时区信息的本地时间与 UTC 解析混用。

---

## 提交规范

- **格式：** Conventional Commits，由 commitlint（`commit-msg` 钩子）强制：类型小写、描述非空。
  - `type`：`feat | fix | refactor | doc | docs | test | chore | perf | build | ci | style | revert`，需与 `cliff.toml` 解析器对齐
  - 示例：`feat(i18n): persist locale in backend` / `fix(auth): avoid reload loop on startup`
- **语言：** 提交信息一律英文（`type`、`scope`、描述与正文），即使对话使用中文。
- **粒度：** 一次提交只做一件事；功能 + 修格式分开提交；禁止 `fix bug` / `update` 等无意义信息。
- **时机：** 由开发者手动提交，不允许自动提交；提交前必跑 `pnpm format` + `pnpm validate`。
- **版本：** 三处版本号经 `pnpm release`（bumpp）联动升级：`package.json` + `src-tauri/tauri.conf.json` + `src-tauri/Cargo.toml`。

CI 流程与提交钩子细节见 `docs/WORKFLOW.md`。

---

## 效力分级与定制

> 面向派生项目：本规范哪些条目由机器强制、哪些只是默认约定，以及想改时的落点（完整定制清单见 `docs/INITIAL.md`）。

1. **机器强制（违反即红）：** CI 步骤（prettier / ESLint / svelte-check / vitest / build，`cargo fmt --check` / clippy `-D warnings` / `cargo test` / `bindings.ts` 同步校验）、提交钩子（commitlint 与 lint-staged）、`i18n:compile` 生成物一致性、`check:sync` 三项对齐（TypeScript 版本 / 插件前后端版本 / 路径别名）。
2. **默认约定（评审与智能体默认遵循，可改）：** 「架构不变量」中的分层与边界、代码风格中的命名与函数规模、提交流程、测试范围。
3. **不降级：** 安全红线；`bindings.ts` / `paraglide/` 生成物禁手改；命令契约流程。

**想改默认约定时：** 直接编辑对应文档，并在同一提交里同步受影响的代码与门禁配置；放宽机器门禁需改 `.github/workflows/ci.yml`、`.husky/`、`commitlint.config.ts`、`lint-staged.config.ts`、`.prettierrc`、`eslint.config.ts`，改完跑 `pnpm validate` 确认本地与 CI 一致。

---

<!-- 维护说明：本文件是常驻规则层，细节写进 docs/ARCHITECTURE.md 与 docs/WORKFLOW.md；改动本文件时同步检查 docs/INITIAL.md 的落点表 -->
