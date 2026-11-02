# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

> 打包形态是桌面应用（Tauri v2 + 系统 WebView），但界面语言与设计基座是 Web（shadcn-svelte + Tailwind），不按任一移动端 OS 的原生设计语言适配。

## Users

**主要用户：准备起步新桌面应用的开发者**（个人或小团队），通过 GitHub「Use this template」或 clone 得到起点，技术栈取向为 Tauri v2 + Svelte。

- 场景：从零开始一个 Windows / macOS / Linux 桌面应用，不想重复解决命令契约、i18n、主题、崩溃兜底、托盘 / 自启 / 更新这些非差异化管道。
- 他们的任务：读 README 与截图评估是否值得用 → 本地跑起来核对质量 → 按 `docs/INITIAL.md` 改名换标派生自有项目 → 把业务写进 `/` 首页占位区。
- 附带角色：模板作者本人把本仓库作为后续自有应用的起手基座（顺带收益，非定位）。

## Product Purpose

提供一个**开箱即用的桌面应用生产级起点**：类型安全的 Tauri 命令链、明暗与配色主题、前后端双轨国际化、崩溃兜底，以及完整桌面外壳（托盘、自启、窗口状态、单实例、深链、更新）全部预接线，开发者拉下来就能写业务。

成功 = 开发者能在不重解管道问题的前提下派生并发布自己的应用；模板主分支保持无演示业务、可长期维护。

## Positioning

**「生产级接线 + 刻意干净的主分支」**——同类模板要么是教学示例仓（业务样例占满、派生时先大扫除），要么是空脚手架（关键能力留给使用者自己接）。

本模板的机制：能力全部真实接线（非文档承诺）、页面是**部分能力**的验证面（`/settings` 展示后端落盘 + 纯前端 + 快捷键注册三类设置的完整链，`/about` 展示诊断与更新链路），但主分支不带任何业务示例；完整插件样例（沙盒文件、对话框、通知、网络请求、文件拖放）放在 `v0.3.0` 标签而非主线（全局快捷键已是主线真实功能，不再只算示例）。

## Operating Context

- **平台与分发**：仅桌面三平台（Windows / macOS / Linux），打包目标 deb / AppImage / rpm / app / dmg / msi / nsis；自动更新对接 GitHub Releases `latest.json`。
- **开发工作流**：`pnpm`（Node ≥ 24、pnpm@12.15.0）+ 锁定 Rust 工具链；日常入口 `pnpm tauri:dev` / `pnpm format` / `pnpm validate`；提交走 Conventional Commits（英文、小写），手动提交；CI（`ci.yml`）与发布（`release.yml`）已就位。
- **派生流程**：`docs/INITIAL.md` 是「模板 → 自有项目」的剧本（改名、换协议、换更新密钥、换图标、文档收尾、验收矩阵），变量未确认前不得臆测。
- **评估面**：README（en）+ `docs/README_zh-CN.md`、`docs/ARCHITECTURE.md` / `docs/WORKFLOW.md`、`docs/images/` 三张截图，以及运行中的应用本身（`/`、`/settings`、`/about`）。
- **模板自身页面即展示面**（已确认）：`/settings` 演示持久化设置、外观设置与快捷键设置三条链，`/about` 演示信息 / 诊断 / 更新，`/` 是布局骨架验证起点；三页可以继续打磨，但必须保持「接线真实可用」。

## Capabilities and Constraints

**已确认能力：**

- tauri-specta 类型安全命令链（`bindings.ts` 生成物 + 链式 API：`.value()` / `.result()` / `.success()` / `.failed()`；未注册的失败经 plugin-log 上报）。
- mode-watcher 明暗主题 + 11 套配色主题（含默认 neutral）（`data-color-theme`）；三套布局（tabs / sidebar / dashboard）；系统字体选择 + 字重 + 字号。
- 前端 Paraglide（en / zh-CN）+ 后端 rust-i18n 双轨国际化，唯一持久化点是后端 `config.json` 的 `locale`。
- 前端 ErrorBoundary + 后端 panic 钩子双崩溃兜底。
- 桌面外壳：无边框窗口 + 自绘标题栏、系统托盘、关闭行为（prompt / exit / minimize_to_tray）、开机自启、窗口状态恢复、单实例聚焦 + 深链转发、显示应用全局快捷键（`shortcut_show` 配置驱动，注册失败 sticky 提示）、更新检查（120s 超时）。

**约束：**

- 许可证 GPL-3.0-only（派生项目闭源分发前必须评估传染性义务）；作者 Alfred Clark。
- 语言仅 en / zh-CN 两个（`Locale` 枚举，禁裸 string；繁体归 zh-CN）。
- 生成物禁手改：`bindings.ts` 只经 `cargo test` 重生成，`paraglide/` 只经 `pnpm i18n:compile`。
- 模板身份（包名 / productName / identifier / 深链 scheme `tdt` / 更新公钥 / 图标 / 截图）是占位性质，派生项目必须按 `docs/INITIAL.md` 整体替换。
- `AGENTS.md` 的安全红线与架构不变量为最高效力规则，任何产品改动不得触碰。

**未决事实：**

- 移动端（iOS / Android）：当前仅桌面三平台，不承诺移动端可迁移；如要扩展需单独确认。

## Brand Commitments

- 名称：仓库 / 包名 `tauri-desktop-template`，展示名 `Tauri Desktop Template`。
- 模板刻意保持**中性无品牌**：不设自有 logo / 营销人格 / 独特视觉身份，图标为占位 (`src-tauri/app-icon.svg`)，页脚只留版权行（`© {year} Alfred Clark. All rights reserved`）。
- 声音与语气：技术、精确、直接；README 英文为主 + 简体中文镜像（`docs/README_zh-CN.md`），文档中的术语与代码标识符保留英文。
- 字体：Geist Variable 为默认界面字体（可被用户系统字体偏好覆盖）。

## Evidence on Hand

- `README.md` + `docs/README_zh-CN.md`：功能清单、快速开始、页面一览、配置说明。
- `docs/ARCHITECTURE.md`（结构、分层、设计不变量）与 `docs/WORKFLOW.md`（命令契约、i18n、CI、验证矩阵）。
- `docs/INITIAL.md`：派生项目全流程与验收矩阵。
- `docs/images/home.png` / `settings.png` / `about.png`：当前界面的真实截图。
- `v0.3.0` 标签：完整插件接线示例（沙盒文件 / 对话框 / 通知 / 全局快捷键 / 网络 / 文件拖放）。
- **不存在且禁止编造**：用户评价、采用数据（star / 下载量）、客户案例、性能基准、企业背书。

## Product Principles

1. **派生项目优先**：主分支上的任何内容都要经得起「派生者用它起步」的检验；占位身份必须能被 `docs/INITIAL.md` 干净替换，不留模板尾巴。
2. **生产级接线，拒绝演示业务**：能力以真实可用的接线存在，而不是文档承诺或玩具示例；示例代码放标签 / 文档，不占主线。
3. **页面即证据，但不是全部证据**：`/`、`/settings`、`/about` 是**页面自身承载的那部分**能力（配置持久化、外观、诊断、更新、布局骨架）的验证面，改动后必须仍然真实可用、可被截图取证；不为观感牺牲真实性。托盘菜单、深链、单实例聚焦、窗口状态、panic 兜底等能力**不要求有界面对象**——它们的证据是接线代码与测试，不是页面；不要为了让能力「可见」而给界面加多余控件。
4. **桌面三平台为准**：一切设计决策以 Windows / macOS / Linux 桌面使用场景为前提，除非另行确认扩展移动端。
5. **规则优先于便利**：`AGENTS.md` 的安全红线与架构不变量高于局部实现便利；生成物、门禁、配置唯一来源不因产品需求绕过。

## Accessibility & Inclusion

- 双语（en / zh-CN）为产品级要求，任何界面文案改动必须同步两份。
- 界面字体族 / 字重 / 字号为用户可调项（系统字体选择器 + 滑块），须保留可用性；自绘无边框标题栏必须保持窗口控制（置顶 / 最小化 / 最大化 / 关闭）可识别可操作。
- 未发现其他已确认的无障碍标准或用户需求；后续如需 WCAG 级别目标需单独确认。
