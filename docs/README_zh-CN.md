[English](../README.md) | 简体中文

# tauri-desktop-template

基于 Tauri v2 + SvelteKit 2 + Svelte 5 的桌面应用程序开发模板。

生产级起点：类型安全的 Tauri 命令链、主题、前后端双轨国际化、崩溃兜底，以及完整的桌面
外壳（托盘、自启、窗口状态、单实例、深链、更新）开箱即用。

> 本模板曾带演示页（`/demo`）与 SQLite 笔记示例（`/notes`），为保持主分支干净已整体移除；
> 完整接线示例（沙盒文件、系统对话框、通知、全局快捷键、网络请求、文件拖放）见上游
> [`v0.3.0`](https://github.com/AlfredClark/tauri-desktop-template/tree/v0.3.0) 标签。

## 应用截图

| 首页 (`/`)               | 设置页 (`/settings`)           | 关于页 (`/about`)           |
| ------------------------ | ------------------------------ | --------------------------- |
| ![首页](images/home.png) | ![设置页](images/settings.png) | ![关于页](images/about.png) |

> 截图存放于 `docs/images/`，以路由名命名（`home`、`settings`、`about`）。

## 功能特性

1. **类型安全命令链（tauri-specta）** — Rust 侧 `#[tauri::command]` +
   `#[specta::specta]` 导出类型到 `src/libs/commands/bindings.ts`（生成物，禁止手改）。
   前端一律走 `src/libs/commands` 链式 API，组件内禁止裸 `invoke`：
   - 带回落取值：`commands.getConfig().value(fallback)`
   - 按状态分支：`commands.getConfig().result()`
   - 事务写法：`commands.updateConfig({ locale }).success(onOk).failed(onFail)`
   - 未注册 `.failed()` 的失败会自动经 `plugin-log` 上报，不会静默丢失。
2. **明暗主题 + 配色主题** — `mode-watcher` 跟随系统（`light` / `dark` / `system`），Tailwind
   v4 `.dark` 变体；十种配色（鲜艳系 `ocean` / `teal` / `emerald` / `amber` / `tangerine` /
   `rose` / `amethyst` + 暗色美学 `nord` / `dracula` / `tokyo-night`，`neutral` 回落默认值）
   经 `data-color-theme` 属性切换（见 `themes.css`，刻意避开 mode-watcher 挂载即覆盖的
   `data-theme`）。纯前端偏好，持久化于 `localStorage`，首帧前生效。
3. **前后端双轨国际化（Paraglide + rust-i18n）** — 前端 `paraglide-js`（`en` / `zh-CN`），后端
   `rust-i18n`（`src-tauri/locales/*.yml`，`fallback = "en"`）。唯一持久化点是后端
   `config.json` 的 `locale` 键：后端先读持久化值、缺失则经 `tauri-plugin-os` 探测系统语言并
   落库；前端在 `+layout.svelte` 挂载期后台水合（刻意不在 `+layout.ts` 的 `load()` 里做——
   Tauri `invoke` 接不进 SvelteKit 传给 `load` 的 `fetch`），再用
   `libs/i18n/locale.svelte.ts` 的 `alignLocale` 对齐（写 localStorage 缓存 + 响应式重渲染，
   不整页重载）。
4. **前后端崩溃兜底** — 前端 `ErrorBoundary`（经 `@tauri-apps/plugin-log` 上报，堆栈仅 dev
   显示，2s 节流）；后端 `cores/system.rs` panic 钩子（先走常规日志，日志不可用时落盘到
   临时目录 `my_app_crash.log`（Win `%TEMP%` / Unix `$TMPDIR`），超 512KB 轮转保留一份）。
5. **设置页（`/settings`）** — 通用分组（语言、开机自启、记住窗口、自动检查更新、托盘、
   关闭行为 `prompt` / `exit` / `minimize_to_tray`）经 `updateConfig` 落盘后端
   `config.json`；外观分组（主题、配色、`tabs` / `sidebar` 布局、系统字体选择器、字重、
   字号）纯前端；快捷键分组（快捷显示：徽章加号展示 + 按键录制，注册失败弹必须手动关闭的提示）；一键恢复默认。
6. **关于页（`/about`）** — 应用信息、项目链接（`opener`，仅放行 `http(s)`）、平台信息、
   诊断分组（打开日志 / 配置目录、复制系统信息）与更新面板（检查 → 带进度下载 → 重启，
   仅桌面端，检查 120s 超时兜底）。
7. **桌面外壳** — 无边框窗口 + 自绘标题栏（置顶 / 最小化 / 最大化 / 关闭）、带本地化菜单
   （显示/隐藏、退出）的系统托盘、关闭行为拦截、开机自启、窗口状态记忆（窗口初始
   `visible: false`，恢复后统一 `show`）、单实例聚焦 + 深链参数转发（`tdt://` 协议）、显示应用全局快捷键
   （`shortcut_show` 配置驱动，注册失败 sticky 提示）、更新对接 GitHub Releases `latest.json`。

## 技术选型

> 版本唯一来源是 `package.json`（前端）与 `src-tauri/Cargo.toml`（后端），按其中声明的版本
> 使用 API，禁止自行升级 / 降级。

| 层级        | 技术选型                                                                                                                                                                  | 说明                                                                                                                                                |
| ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| 前端框架    | SvelteKit 2 + Svelte 5                                                                                                                                                    | SPA 模式（`adapter-static` + `fallback: index.html`，`ssr = false`），Vite 端口 1420                                                                |
| UI / 样式   | shadcn-svelte（nova / neutral）+ Tailwind CSS v4，Lucide 图标，Geist 字体                                                                                                 | `cn()` 合并类名，`.dark` 变体 + `[data-color-theme]` 配色                                                                                           |
| 前端插件    | `@tauri-apps/api`、`plugin-log` / `opener`、`system-fonts`                                                                                                                | 日志经 `plugin-log` 上报，字体列表经 `system-fonts-api`                                                                                             |
| 前后端契约  | tauri-specta + `specta` / `specta-typescript`                                                                                                                             | 生成物 `src/libs/commands/bindings.ts`，禁止手改                                                                                                    |
| 国际化      | Paraglide（前端）+ rust-i18n（后端）                                                                                                                                      | `src/libs/i18n/messages/*.json` + `src-tauri/locales/*.yml`                                                                                         |
| 后端框架    | Tauri v2 + Rust（edition 2024，工具链由 `rust-toolchain.toml` 锁定）                                                                                                      | `commands/` 薄封装（按调用域），`features/` 按业务类型组装完整业务，`cores/` 跨层共享能力                                                           |
| 后端插件    | `opener` / `store` / `log` / `os` / `updater` / `autostart` / `window-state` / `system-fonts` / `single-instance` / `deep-link` / `clipboard-manager` / `global-shortcut` | `updater` / `autostart` / `window-state` / `single-instance` / `deep-link` / `global-shortcut` 仅桌面端（`global-shortcut` 仅 Rust 侧，无前端直调） |
| 构建 / 质量 | TypeScript 严格模式 + `svelte-check`，ESLint + Prettier，vitest，pnpm                                                                                                     | Node `>= 24`，`pnpm-lock.yaml` 与 `Cargo.lock` 均需提交                                                                                             |
| 部署        | GitHub Actions（`ci.yml` / `release.yml`）+ Tauri bundler                                                                                                                 | 三处版本号联动（见下文）                                                                                                                            |

## 快速开始

环境要求：Node `>= 24`、`pnpm@12.15.0`、Rust 工具链以 `rust-toolchain.toml` 为准。

```bash
pnpm install --frozen-lockfile   # 按锁文件安装（不要用 npm / yarn）

pnpm tauri:dev                   # 完整桌面联调：Vite（端口 1420）+ Tauri 窗口
pnpm dev                         # 仅启动前端（Tauri 命令不可用时自动降级）

pnpm format                      # 自动修复两端：Prettier + ESLint + cargo fmt
pnpm validate                    # test + lint + check（每次修改后必跑）

pnpm build                       # 仅构建前端（adapter-static 输出到 build/）
pnpm tauri:build                 # 完整打包；tauri:build:local 仅编译不打包
```

常用脚本：

```bash
pnpm lint            # 前端 + 后端全量检查
pnpm check           # svelte-kit sync + svelte-check（严格 TS 检查）
pnpm test            # vitest + cargo test 单次运行
pnpm test:cargo      # 后端测试 + 重生成 bindings.ts
pnpm i18n:compile    # 改完 messages/ 后必须执行
pnpm tauri:icon      # 由 src-tauri/app-icon.svg 生成各平台图标
pnpm release         # bumpp 联动三处版本：package.json + tauri.conf.json + Cargo.toml
```

## 页面一览

| 路由        | 内容说明                                                                 |
| ----------- | ------------------------------------------------------------------------ |
| `/`         | 首页英雄区：技术栈图标 + 应用名 + 简介；布局骨架验证起点，业务在此开发。 |
| `/settings` | 通用（后端持久化）+ 外观（纯前端）+ 快捷键（全局注册）+ 恢复默认。       |
| `/about`    | 应用 / 项目 / 平台 / 诊断四分组 + 更新面板。                             |

导航标签在 `src/libs/navigation/nav-tabs.ts` 一处注册，供 `tabs` 与 `sidebar` 两套布局
（`src/components/layout/`）同源使用；新增页面只需加一项。外观（布局注册表 + 字体偏好）
位于 `src/libs/hooks/appearance.svelte.ts`。

## 配置说明

后端 `config.json`（经 `tauri-plugin-store`，逻辑在 `cores/config.rs`）保存 durable 配置：

- `locale`（specta 导出的 `"en" | "zh-CN"` 联合类型，禁止裸 `string`）、`auto_start`、
  `remember_window`、`auto_check_update`、`tray_enabled`、
  `close_behavior`（`prompt` / `exit` / `minimize_to_tray`）、`shortcut_show`（显示应用全局快捷键，空即未设置）、`schema_version`。
- 写路径只走 `update` / `save_locale` / `migrate` 三入口，`update` 内幂等 `migrate`。
  新增 / 变更配置项时，若改动已有字段语义须同步 `CURRENT_SCHEMA_VERSION` 与 `MIGRATIONS`
  迁移链。
- 纯前端外观偏好（主题、配色、布局、字体 / 字重 / 字号）只存 `localStorage`，不走命令链，
  是有意例外。

## 特殊流程

**新增 / 修改命令：** 有对应业务时 `features/<name>.rs` 实现业务（在 `features/mod.rs` 声明
`pub mod <name>;`），无对应业务直接调 `cores/` → `commands/<name>.rs` 薄封装（`CommandResult` + 双注解 +
`collect_commands!`）→ `cargo test` 重生成 `bindings.ts` → 前端链式 API 调用 → 联调。
禁止手改 `bindings.ts`、禁止跳过重生成直接改前端。

**改文案：** 前端改 `src/libs/i18n/messages/*.json` → `pnpm i18n:compile` → 用 `m.<键>()` 取值、
`setLocale()` 切换；后端改 `src-tauri/locales/*.yml` → `rust_i18n::t!(...)` 取值。

**模板派生新项目：** 改名换标、深链协议与更新密钥轮换、图标替换、文档收尾与验收矩阵的完整
清单见 `docs/INITIAL.md`。脚手架（演示页与笔记示例）已由模板主分支移除；需要这些接线示例时见
上游 `v0.3.0` 标签。

## 项目结构（精简）

```text
src/routes/(main)/         # /、/settings、/about 页面；页面组件在各路由的 `_components/`
src/components/layout/    # tabs / sidebar / dashboard 布局壳 + 标题栏 / 导航部件
src/components/common/    # 跨页共享的手写组件（card-section / card-row / confirm-dialog）
src/libs/commands/        # bindings.ts（生成物）+ 链式 API 封装
src/libs/i18n/            # messages/ 文案 + project.inlang/ 配置 + locale.svelte.ts 语言响应式桥
src/libs/navigation/      # nav-tabs.ts 导航单源注册表
src-tauri/src/commands/   # 薄封装 + collect_commands! 注册
src-tauri/src/features/   # 纯业务逻辑（不依赖 Tauri 运行时）
src-tauri/src/cores/      # config / locale / system / tray / updater 等共享能力
src-tauri/src/plugins/    # 各插件初始化
src-tauri/locales/         # tray.yml（`tray.*`），一文件内含 en + zh-CN 双写
```

分层、边界与完整结构见 `AGENTS.md`、`docs/ARCHITECTURE.md`、`docs/WORKFLOW.md`。

## 验证与贡献

- 任何改动完成后先 `pnpm format` 再 `pnpm validate`，全绿前不得宣称完成、不得提交。
  改后端命令需另确认 `bindings.ts` 有同步 diff；改文案需补跑 `pnpm i18n:compile`。
- 提交遵循 Conventional Commits、英文小写（例：`feat(i18n): persist locale in backend`），
  一次提交只做一件事；由开发者手动提交，不自动提交。
- 发版：提 PR → CI 全绿 → Code Review → Squash 合并 → `pnpm release` 联动三处版本号
  （`package.json` + `src-tauri/tauri.conf.json` + `src-tauri/Cargo.toml`）→ 打 tag `vX.Y.Z`
  → `release.yml` 自动打包 → `pnpm changelog` 生成日志。

## 开源协议

GPL-3.0-only，见 [LICENSE](../LICENSE)。
