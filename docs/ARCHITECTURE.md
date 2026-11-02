# ARCHITECTURE.md — 架构、结构与设计不变量

> 按需文档：放新文件、判断分层归属、动配置 / 崩溃兜底 / 国际化数据流 / 初始化逻辑之前读本文件。
> `AGENTS.md` 只保留常驻规则结论，本文件承载结构细节与「为什么」。
> 版本唯一来源是 `package.json`（前端）与 `src-tauri/Cargo.toml`（后端）；生成代码必须使用这两个文件实际声明版本对应的 API，禁止自行升级 / 降级依赖。

---

## 技术栈与关键约定

- **前端框架：** SvelteKit + Svelte，SPA 模式（`adapter-static` + `fallback: index.html`，`ssr = false`）。
- **UI / 样式：** shadcn-svelte（nova / neutral）+ Tailwind CSS v4（`@tailwindcss/vite` + `tailwind-merge` / `tailwind-variants` / `tw-animate-css`）；图标 `@lucide/svelte`，字体 `@fontsource-variable/geist`；主题经 `mode-watcher` + `layout.css` 的 `.dark` 变体；配色经 `themes.css` 的 `[data-color-theme]` 11 套配色主题（10 命名加默认 `neutral` 回落，暗色须用 `:root.dark[data-color-theme]` 显式组合）；`cn()` 合并类名。
- **前端插件：** `@tauri-apps/api` + `@tauri-apps/plugin-log`（日志上报）/ `@tauri-apps/plugin-opener` / `@tauri-apps/plugin-deep-link` / `tauri-plugin-system-fonts-api`（系统字体探测）。
- **前后端契约：** tauri-specta + `specta` / `specta-typescript`；Rust 侧 `#[tauri::command]` + `#[specta::specta]` 导出类型，生成物 `src/libs/commands/bindings.ts`（禁手改，但需要提交）。
- **国际化：** 前端 Paraglide（策略 `localStorage` + `baseLocale`，CLI 为 `@inlang/cli`）；后端 rust-i18n（`src-tauri/locales/*.yml`，`fallback = "en"`）。
- **后端：** Tauri v2 + Rust（`edition = "2024"`，工具链由 `rust-toolchain.toml` 锁定，`tauri-build` 构建脚本）。
- **后端插件：** `opener` / `store` / `log` / `os` / `updater` / `autostart` / `window-state` / `system-fonts` / `single-instance` / `deep-link` / `clipboard-manager` / `global-shortcut`——依次对应外部打开、持久化（`config.json` 存 `locale`）、日志、系统语言与平台信息采集（`tauri-plugin-os` 仅作 Rust 库使用）、自动更新 / 开机自启 / 窗口状态（仅非移动端启用）、系统字体探测（`plugins/system_fonts.rs`）、单实例（二次启动聚焦 + 深链参数转发，仅桌面端）、深链协议（`plugins.deep-link.desktop.schemes`）、剪贴板纯文本、显示应用全局快捷键（配置驱动动态注册，仅桌面端、无前端直调）。
- **后端支撑：** `serde` / `serde_json` + `chrono`（UTC 时间）+ `log` 门面 + `thiserror` / `anyhow`（统一错误 `CommandError` / `CommandResult`）。
- **构建 / 质量：** Vite（固定端口 `1420`）+ `svelte-check` 严格 TS 检查 + ESLint / Prettier + vitest；包管理器 `pnpm`（`engines.node >= 24`）；`pnpm-lock.yaml` 与 `Cargo.lock` 均提交。
- **部署：** GitHub Actions + Tauri bundler；入口 `.github/workflows/ci.yml`（质量门禁）、`release.yml`（打包发布）。
- **上游示例索引：** 沙盒文件 / 对话框 / 通知 / 网络请求的完整接线示例见上游仓库 `v0.3.0` 标签（全局快捷键已是主线真实功能，见设置页快捷键分组，不再只算示例）。

**版本查询命令：**

```bash
node -v && pnpm -v && cat package.json | grep -A 30 '"dependencies"'
rustc --version && cat rust-toolchain.toml && cat src-tauri/Cargo.toml | grep -A 20 '\[dependencies\]'
```

---

## 目录结构（全量）

> 目录职责边界：只在对应目录做对应事，禁止跨层乱放。

```text
tauri-desktop-template/
├── src/                            # 前端：SvelteKit 单页应用（adapter-static + fallback: index.html）
│   ├── app.html                    # HTML 外壳，SvelteKit 在此注入脚本与样式
│   ├── assets/                     # 需要经构建处理的资源（别名 #assets，见 package.json 的 imports 字段），icons/ 下为内联图标（app-icon / github-icon / svelte-icon / tauri-icon / vite-icon）
│   ├── routes/                     # 页面与全局样式
│   │   ├── +layout.svelte          # 根布局：ModeWatcher（主题）+ Toaster + ErrorBoundary（渲染异常）包裹全部分组，首帧前对齐外观变量（字体/字重/字号）
│   │   ├── +layout.ts              # ssr = false（SPA 模式）；配置水合不在 load()（invoke 接不进 event.fetch），见 +layout.svelte 挂载期后台水合
│   │   ├── (main)/                 # 分组路由（括号不进 URL）：常规页面分组，独享布局容器
│   │   │   ├── +layout.svelte      # 分组布局：LayoutContainer 包裹，特殊页另起分组即可绕开布局
│   │   │   ├── +page.svelte        # 占位首页：布局骨架验证起点，业务在此开发
│   │   │   ├── about/              # 关于页（应用 / 项目 / 平台分组 + 更新面板；页面组件在 `_components/`）
│   │   │   └── settings/           # 完整设置页（通用 + 外观 + 快捷键 + 重置；页面组件在 `_components/`，超过 ~8 个分组时按分组建子目录）
│   │   └── layout.css              # Tailwind v4 入口与 shadcn-svelte 主题令牌（含 .dark 暗色变体与 --app-font-* 外观变量）
│   │   └── themes.css              # 配色主题令牌（鲜艳系 7 色 + 暗色美学 3 色的浅色 + :root.dark 暗色块，neutral 回落 layout.css）
│   ├── components/
│   │   ├── common/                 # 跨页共享的手写组件（error-boundary.svelte 全局渲染兜底，card-section/row.svelte 通用卡片分组与行，confirm-dialog.svelte 通用确认弹窗）
│   │   ├── layout/                 # 布局子系统：layout-container.svelte 容器 + tabs（标题栏/标签栏/内容/底边）、sidebar（侧边栏 + 标题栏/内容/底边）与 dashboard（可收起左侧导航 + 标题栏/内容/底边）实现，新增布局需在 libs/hooks/appearance.svelte.ts 中映射
│   │   │   └── parts/              # 布局配套部件（title-bar / nav-tabs-bar / side-nav-bar / dashboard-nav-bar / copyright）
│   │   └── shadcn-svelte/          # CLI 生成的 UI 组件（nova / neutral / lucide），新增走 CLI 添加，勿手动重组
│   ├── tests/                      # 前端测试集中目录，按源路径镜像：unit/ 纯逻辑（node）+ component/ 组件（jsdom）
│   └── libs/
│       ├── commands/               # tauri-specta 契约链（bindings.ts 生成物 + 链式 API 封装）
│       ├── i18n/                   # Paraglide：messages/ 文案、project.inlang/ 配置、paraglide/ 生成物、locale.svelte.ts 语言响应式桥
│       ├── navigation/             # 导航等应用级注册表（nav-tabs.ts：路径 / 图标 / 顺序，顶部标签栏与侧边栏导航同源）
│       ├── shortcut/                 # 快捷键领域模块（展示切分 / 录制归一化，与后端 `Shortcut::from_str` 可解析子集对齐）
│       ├── utils/                  # 前端通用工具（shadcn-svelte.ts 的 cn()、window-controls.ts、toast.ts、opener.ts）
│       └── hooks/                  # 前端共享状态（统一经 #libs/hooks 引用），如 appearance.svelte.ts（布局注册表 + 字体偏好）、config.svelte.ts（后端配置内存态）、updater.svelte.ts（更新状态）、is-mobile.svelte.ts（响应式断点）、deep-link.svelte.ts（三路深链入口）、dashboard.svelte.ts（仪表盘侧栏折叠偏好）
├── src-tauri/                      # 后端：Rust（edition 2024）
│   ├── src/
│   │   ├── main.rs                 # 二进制入口，仅转发到 lib::run()
│   │   ├── lib.rs                  # 串联插件注册 → 命令处理器 → 核心初始化
│   │   ├── commands/               # 命令封装层：薄封装 + collect_commands! 注册
│   │   ├── cores/                  # 通用能力与跨层共享类型（types / locale / config / deep_link / shortcut / specta / system / tray / updater）
│   │   ├── features/               # 具体业务逻辑：按业务类型拆文件组装完整业务，供 commands/ 调用（通用逻辑抽到 cores/；当前为空）
│   │   └── plugins/                # 各 Tauri 插件的初始化（log / store / opener / updater / autostart / window-state / system-fonts / single-instance / deep-link / clipboard / global-shortcut）
│   ├── locales/                    # rust-i18n 后端文案（按功能分文件，一文件一命名空间：当前 tray.yml 的 tray.*，单文件可内含多 locale 的 en/zh-CN 双写）
│   ├── app-icon.svg                # 应用图标源（推荐正方形透明底 SVG，标称尺寸任意；PNG 备选 ≥1024；`pnpm tauri:icon` 的输入，换图标时替换本文件）
│   ├── capabilities/               # 权限配置（default.json / plugins.json），外部 URL 需同步更新
│   ├── icons/                      # 应用图标（由 pnpm tauri:icon 生成）
│   ├── build.rs                    # tauri-build 构建脚本
│   ├── tauri.conf.json             # 窗口 / CSP / 打包目标 / updater 配置
│   └── Cargo.toml                  # 后端依赖与 crate 元信息
├── docs/                           # ARCHITECTURE.md（本文件）、WORKFLOW.md（命令与流程）、INITIAL.md（派生项目定制与发布清单）、README_zh-CN.md、images/（模板截图）
├── static/                         # 原样拷贝到产物根目录的资源（favicon.png）
├── .github/workflows/              # ci.yml（质量门禁）、release.yml（打包发布）
├── Cargo.toml                      # 虚拟工作区 + 全局 lints / profile
├── package.json                    # 脚本、前端依赖与路径别名（imports 字段）
├── svelte.config.ts                # adapter-static 与 Svelte 预处理器配置
├── vite.config.ts                  # Vite + SvelteKit + Tailwind + Paraglide 插件，固定端口 1420
├── vitest.config.ts                # 单元测试配置（复用 sveltekit() 插件；别名由 package.json imports 提供）
├── tsconfig.json                   # 继承 .svelte-kit 生成配置与 tsconfig.aliases.json，TS 严格检查选项
├── tsconfig.aliases.json           # # 别名的 TS 侧镜像（权威在 package.json imports；check:sync 校验一致）
└── AGENTS.md                       # 常驻开发规范（自动加载入口）
```

---

## 分层与依赖方向

```text
Svelte 组件 → libs/commands（链式 API）→ commands/（Rust 薄封装）→ features/ · cores/
```

- 禁止反向调用与跨层跳跃：组件裸 `invoke`、命令层写业务逻辑都属违规。
- 后端业务归属：具体业务逻辑按业务类型拆分为 `features/<业务>.rs`（`features/mod.rs` 声明 `pub mod <业务>;`），在其中组装完成该业务所需的完整逻辑，由 `commands/` 调用；不要求纯函数，也允许依赖 Tauri 运行时。多个业务通用或共享的逻辑抽取到 `cores/`，不散落在各业务文件中。`commands/` 按前端调用域分组，与 `features/` 的业务分组允许 N:M。
- 纯前端 UI 偏好（布局 / 字体等外观偏好走 `localStorage`）是有意例外，不经 commands 链。
- `libs/utils/` 只放不含业务 / 应用知识的通用工具（`cn()` / `toast` / `window-controls`）；注册表类（路径、图标、顺序等应用知识）放 `libs/<领域>/`，如导航注册表在 `libs/navigation/`。后端通用能力一律进 `cores/`，不另设 `utils/`。
- 页面专有组件放所属路由的 `_components/`（页面内相对导入）；仅跨页共享的手写组件放 `components/common/`。
- 布局子系统（`components/layout/`）新增布局类型时，必须在 `libs/hooks/appearance.svelte.ts` 的注册表映射中登记。

---

## 路径别名与配置唯一来源

- 路径别名（`#assets` / `#components` / `#libs`）唯一来源是 `package.json` 的 `imports` 字段（Node subpath imports，Vite / Vitest 原生解析）；TS 检查器（tsc / svelte-check）读 `tsconfig.aliases.json` 的只读镜像 `paths`（TS 对 imports 映射目标按 Node 语义不做隐式扩展名补全），由 `scripts/check-version-sync.ts` 在 `pnpm check` 中校验一致；不写入 `svelte.config.ts` / `vite.config.ts`（SvelteKit 已弃用 `kit.alias`，勿再启用）。
- `components.json` 的 `aliases.*` 必须与别名表写法一致（`aliases.hooks` 即 `#libs/hooks`）——CLI 生成的组件按其值发 import，改别名时两处同步，避免漂移。
- Paraglide 编译选项唯一来源是 `src/libs/i18n/project.inlang/paraglide.config.ts`（CLI 与 vite 插件都读它），禁止往 `package.json` / `vite.config.ts` 各写一份。
- 新增外部 URL 必须同步更新 `capabilities/*.json` 与 `tauri.conf.json` 的 CSP（`updater` 下载端点由 Rust 侧发起，不受 WebView CSP 约束，无需同步 CSP）。

---

## 设计不变量与原因

> 以下约束由历史 bug 得出（多为「静默失败」类），改动相关代码前先读「原因」，不要按直觉简化。

### 配置（`cores/config.rs`）

- 新增 / 变更配置项必须同步 `CURRENT_SCHEMA_VERSION` 与 `MIGRATIONS` 迁移链（仅改动已有字段语义时才递增版本），否则老用户配置会静默失效。
- 配置写锁三入口：`update` / `save_locale` / `migrate`（内部函数不加锁）；新增写入路径必须走其一。
- 文件规模阈值：`cores/config.rs` 超过 ~1500 行时把迁移链拆为 `cores/config/migrate.rs` 目录模块，读写与副作用保留在 `config.rs`。
- `update` 内幂等 `migrate`；存储不可用直接失败（不基于回落默认值决策）；自启落盘失败回滚操作系统状态。

### 崩溃边界（`cores/system.rs` + `components/common/error-boundary.svelte`）

- 前端 `ErrorBoundary`：经 `@tauri-apps/plugin-log` 上报，堆栈仅 dev 显示；同步 `console.error` 留底 + 2s 节流防刷屏。
- 后端 panic 钩子：先走常规日志，日志不可用时落盘到临时目录 `my_app_crash.log` 兜底（Win `%TEMP%` / Unix `$TMPDIR`，二选一而非双写）；超 512KB 轮转为 `.1` 仅保留一份；时间戳用本地时间 RFC3339（`chrono::Local`）。
- 两者互不替代，禁止另加并行机制。

### 语言数据流（`cores/locale.rs` + `cores/config.rs` + `libs/hooks/config.svelte.ts` + `libs/i18n/locale.svelte.ts`）

- 唯一持久化点是后端 `config.json` 的 `locale` 键，类型为 `Locale` 枚举（经 specta 导出为 `"en" | "zh-CN"` 联合类型，禁止裸 `string` 或类型断言；`Locale::parse` 仅支持这两项，繁体一律归 `zh-CN`，加语言时同步改解析、`locales/*.yml` 与前端文案）。
- 启动：后端先读持久化值，缺失则 `tauri_plugin_os::locale()` 探测并落库，再 `rust_i18n::set_locale`。
- 前端：`+layout.svelte` 挂载期后台经 `commands.getConfig` 水合配置（失败重试一次，不阻断首帧），再用 `config.locale` 调 `libs/i18n/locale.svelte.ts` 的 `alignLocale` 对齐（写 localStorage 缓存 + 更新响应式桥，不整页重载，否则语言闪烁）。刻意不做在 `+layout.ts` 的 `load()` 里——Tauri `invoke` 走 `window.fetch("ipc://...")`，接不进 SvelteKit 传给 `load` 的 `fetch`，且纯 SPA 下 `load` 相对首帧并无时序优势。
- 前端响应式（`libs/i18n/locale.svelte.ts`）：Paraglide 的 `getLocale` 本是普通函数，Svelte 感知不到语言变化，启动对齐（`reload: false`）后已渲染文案不会重渲染（首次启动残留 en 文案，刷新才恢复）。该模块用 `overwriteGetLocale` 把 `getLocale` 覆盖为读取 `$state`，渲染期所有 `m.*()` 自动建立响应式依赖，对齐时原地重渲染；不重载的语言变更只允许经 `alignLocale`（裸调 `setLocale(reload: false)` 会绕过同步）。
- 用户切换：`commands.updateConfig({ locale })` 先落盘再改内存（返回是否成功，调用方按返回值分支，禁止用状态相等代理推断）；运行时 `rust_i18n` 由 `config::apply_runtime_effects` 统一同步；前端随后用 Paraglide 默认 `setLocale` 重载。
- 更新命令 `restart`：桌面端永不结算（勿 `await`）、移动端返回错误走 `.failed()`；检查 120s 超时兜底，待重启态不被新检查覆盖。

### 初始化与首帧镜像（`libs/hooks/appearance.svelte.ts` + `src/app.html`）

- `initAppearance` 类「读存储 → 写内存 → 应用到 DOM」的初始化只允许跑一次，禁止进入跟踪型 `$effect`（init 读写同一批状态，被跟踪即形成「改动→重初始化」回路：未落盘的预览会被存量覆盖，字重滑块冻住即此因；必须包 `untrack`）。
- `app.html` 的首帧内联脚本是 init 在 JS 就绪前的同步镜像，两处键名与分支逻辑必须同步改。
- `localStorage` 存量键名即契约（改名即视为放弃老用户存量）：已发布的存量键冻结豁免，仅新增键统一加 `app.` 前缀，禁止裸通用名。

### 窗口与构建（`cores/config.rs` 的 `setup`）

- 主窗口初始 `visible: false`（防恢复闪烁），由 `setup` 按记住的窗口配置恢复后统一 `show`；任何提前返回前必须显示，否则永久黑屏；仅恢复 `main` 窗口。
- Vite 固定端口 `1420`，忽略监听 `src-tauri/**`，`clearScreen: false` 以保留 Rust 日志。
- 构建产物（`target/`、`build/`、`.svelte-kit/`、`src-tauri/gen/`、`node_modules/`、`src/libs/i18n/paraglide/`）均已忽略；`bindings.ts` 是生成物但需要提交；`Cargo.lock` 需要提交；`src-tauri/app-icon.svg` 为图标源文件（推荐正方形透明底 SVG，标称尺寸任意；PNG 备选 ≥1024）。

---

## 前后端交互约定

- 调用一律经 `src/libs/commands` 链式 API，禁止组件内裸 `invoke`：

```ts
// ✅ 推荐：链式 API + 默认值回落
import commands from "#libs/commands";
const config = await commands.getConfig().value(fallback);
const result = await commands.getConfig().result();
await commands.updateConfig({ locale }).success(onOk).failed(onFail);

// ❌ 禁止：裸 invoke + 手改绑定
import { invoke } from "@tauri-apps/api/core";
const config = await invoke<Config>("get_config");
```

- `EnhancedCommand` 三种用法：取值 `.value()` / 分支 `.result()` / 事务 `.success()/.failed()`。关键取值用 `.result()` 按 `status` 分支——`.value()` 的回落会把失败与空数据压成同一个值。
- 回调在 `await / value()` 之前链式注册；回调内只做幂等赋值 / toast 且保证不抛错（回调抛错只上报不扩散，`await` 仍得原结果；关键分支若放回调内会形成「状态未落但结果为 ok」的不一致）；多步分支 / 跨状态机流转必须用 `result()` 后置分支。
- 错误体：后端 `CommandError::Internal(...)`（`anyhow::Error` 经已有 `From` 自动转换），前端 `.failed()` 收到 `{ kind, message }`；未注册 `.failed()` 的失败会自动经 plugin-log 上报，不会静默丢失。多个业务通用的逻辑应抽取到 `cores/`，避免散落在各业务文件中。
- 时间：后端 `chrono`；业务时间全链路 UTC，展示层转本地时区；禁止无时区信息的本地时间与 UTC 解析混用。
- Prettier 硬性约束：`plugins` 数组中 `prettier-plugin-tailwindcss` **必须在最后**；`importOrderTypeScriptVersion` 必须与 `package.json` 的 `typescript` 版本同步。
- 路径跨平台：后端路径一律经 `PathBuf::join` / Tauri `app.path()` 解析，禁止硬编码分隔符与字符串拼接；跨 FFI/插件边界必须转 `String` 时用 `to_str()` + `context` 大声失败，禁止 `to_string_lossy()` 静默篡改（Linux/macOS 非 UTF-8 路径真实存在）；崩溃兜底用 `env::temp_dir()`，文档中称「临时目录」而不写 `%TEMP%`；前端不碰本地路径，唯一文件出口是后端固定目录 `open_path`，外链只走 `openExternal` 的 `http(s)` 白名单。

---

## 成对维护的配置

| 配置 A                                                   | 配置 B                                  | 校验                                   |
| -------------------------------------------------------- | --------------------------------------- | -------------------------------------- |
| 两个 workflow 的 `dtolnay/rust-toolchain` 的 `toolchain` | `rust-toolchain.toml`                   | 人工评审                               |
| `.prettierrc` 的 `importOrderTypeScriptVersion`          | `package.json` 的 `typescript`          | `check:sync`                           |
| Tauri 插件前后端版本：`pnpm-lock.yaml` 的 npm 包         | `Cargo.lock` 的 cargo 包（major.minor） | `check:sync`（升级入口 `update:all`）  |
| `.prettierignore`                                        | `.gitignore` 中的构建产物               | 人工评审（Prettier 不读 `.gitignore`） |
| 路径别名：`package.json` 的 `imports`                    | `tsconfig.aliases.json` 的 `paths`      | `check:sync`                           |
| CI backend 的 `changes` 路径过滤器                       | 新增的后端配置文件                      | 人工评审                               |

---

<!-- 维护说明：本文件承接原 AGENTS.md 的技术总览与项目结构章节；技术选型或结构变更时优先更新本文件，再改代码 -->

## 文档联动清单

> 改代码时同步改文档：下表是「改了 A 就必须检查 B」的正向清单，与上表「成对维护的配置」同等效力，改完对照打勾。

| 代码改动               | 必须同步的文档位置                                                                                                                                         |
| ---------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 新增后端插件           | 本文件技术栈与目录树、`AGENTS.md` 安全红线#4、`docs/INITIAL.md` §7 白名单与 §8.2 计数、`README.md` 与 `docs/README_zh-CN.md` 功能与外壳、`PRODUCT.md` 能力 |
| 新增 / 变更配置项      | 本文件配置节、`README.md` 与 `docs/README_zh-CN.md` 配置说明（`INITIAL.md` §8.2 文案键数如联动）                                                           |
| 新增 / 改签名命令      | `docs/INITIAL.md` §8.2 命令数（如变化）、`docs/WORKFLOW.md` 命令契约流程（如改流程）                                                                       |
| 新增前端依赖 / 插件    | 本文件技术栈、`README.md` 与 `docs/README_zh-CN.md` 技术选型、`docs/INITIAL.md` §8.2 计数                                                                  |
| 新增文案键             | `docs/INITIAL.md` §8.2 文案键数                                                                                                                            |
| 新增设置分组 / 页面    | `README.md` 与 `docs/README_zh-CN.md` 页面一览与功能、`docs/images/` 截图重拍、本文件目录树                                                                |
| 调整门禁 / 脚本 / 校验 | `docs/WORKFLOW.md` 对应章节、`AGENTS.md`「效力分级与定制」                                                                                                 |

---

<!-- 维护说明：本文件承接原 AGENTS.md 的技术总览与项目结构章节；技术选型或结构变更时优先更新本文件，再改代码 -->
