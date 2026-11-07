# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

> 打包形态是桌面应用（Tauri v2 + 系统 WebView），但界面语言与设计基座是 Web（shadcn-svelte + Tailwind），不按任一移动端 OS 的原生设计语言适配。

## Users

**主要用户：需要在桌面端制作 EPUB 电子书的作者与小团队**（个人或小团队），覆盖小说与漫画两种形态。

- 场景：整理书稿 / 图片目录 → 按步骤导入、解析、编辑 → 构建并校验 → 打出标准 EPUB 3.3 → 导出分发。
- 他们的任务：打开应用从首页工作台继续上次的工作 → 按六步骤推进项目 → 在构建 / 打包页确认产物有效 → 导出。将 `README.md` 作为中文向导。

## Product Purpose

提供一条**桌面端 EPUB 生产流水线**：小说与漫画两种项目，按元数据 → 文件 → 编辑 → 构建 → 打包 → 导出六步骤推进，全程校验，最终产出标准的 EPUB 3.3（小说流式排版、漫画定版式）。

成功 = 用户能不看文档凭首页工作台找到上次的工作，一路绿灯做到导出，且打出的包在主流阅读器与墨水屏上都能正常打开。

## Positioning

**「步骤清晰的 EPUB 工作台」**——同类工具要么是纯文本编辑器（不管 EPUB 规范细节），要么是命令行打包脚本（没有项目管理与渐进式校验）。

本应用的机制：能力全部真实接线（非文档承诺），页面是**部分能力**的验证面（`/` 工作台展示最近项目与管线阶段，`/settings` 展示后端落盘 + 纯前端 + 快捷键注册三类设置的完整链，`/about` 展示诊断与更新链路，`/projects` 展示项目增删改查后端链 + `(projects)` 展示分组自带 chrome 的步骤导航），主分支不带演示业务。

## Operating Context

- **平台与分发**：仅桌面三平台（Windows / macOS / Linux），打包目标 deb / AppImage / rpm / app / dmg / msi / nsis；自动更新对接 GitHub Releases `latest.json`。
- **开发工作流**：`pnpm`（Node ≥ 24、pnpm@12.18.0）+ 锁定 Rust 工具链；日常入口 `pnpm tauri:dev` / `pnpm format` / `pnpm validate`；提交走 Conventional Commits（英文、小写），手动提交；CI（`ci.yml`）与发布（`release.yml`）已就位。
- **初始化**：已按初始化清单完成改名、换协议、换更新密钥、换图标与文档收尾。
- **评估面**：README（zh-CN）+ `docs/README_en.md`、`docs/ARCHITECTURE.md` / `docs/WORKFLOW.md`、`docs/images/` 三张截图，以及运行中的应用本身（`/`、`/settings`、`/about`、`/projects` 与 `(projects)` 小说/漫画六步骤）。
- **模板自身页面即展示面**（已确认）：`/settings` 演示持久化设置、外观设置与快捷键设置三条链，`/about` 演示信息 / 诊断 / 更新，`/` 是布局骨架验证起点；三页可以继续打磨，但必须保持「接线真实可用」。

## Capabilities and Constraints

**已确认能力：**

- 项目与工作台：小说 / 漫画两类项目增删改查，首页工作台按最后活跃排序展示最近项目与管线四阶段（素材 → 解析 → 构建 → 打包），来源感知返回。
- 小说管线：元数据 → 原文导入解析（卷章切分 + 问题报告）→ 章节编辑 → 构建 → 打包校验（epubveri）→ 导出 TXT / EPUB / 原图包。
- 漫画管线：原稿导入 → 顺序解析（平铺 / 分话）→ 页序编辑（拖拽 + 键盘重排）→ 格式转换（JPEG / PNG / WebP）→ 图片压缩（等比缩放 + 灰度）→ 定版式构建 → 打包校验 → 导出。
- 标准 EPUB 3.3：小说流式排版、漫画 `pre-paginated` 固定布局 + 每页 viewport；封面挂 `cover-image`；WebP 打包警告。
- tauri-specta 类型安全命令链（`bindings.ts` 生成物 + 链式 API：`.value()` / `.result()` / `.success()` / `.failed()`；未注册的失败经 plugin-log 上报）。
- mode-watcher 明暗主题 + 11 套配色主题（含默认 neutral）（`data-color-theme`）；三套布局（tabs / sidebar / dashboard）；系统字体选择 + 字重 + 字号。
- 前端 Paraglide（en / zh-CN）+ 后端 rust-i18n 双轨国际化，唯一持久化点是后端 `config.json` 的 `locale`。
- 前端 ErrorBoundary + 后端 panic 钩子双崩溃兜底。
- 桌面外壳：无边框窗口 + 自绘标题栏、系统托盘、关闭行为（prompt / exit / minimize_to_tray）、开机自启、窗口状态恢复、单实例聚焦 + 深链转发、文件选择框（封面选图）、显示应用全局快捷键（`shortcut_show` 配置驱动，注册失败 sticky 提示）、更新检查（120s 超时）。

**约束：**

- 许可证 AGPL-3.0-only（派生项目闭源分发前必须评估传染性义务，网络服务形态同样触发源码提供义务）；作者 Alfred Clark。
- 语言仅 en / zh-CN 两个（`Locale` 枚举，禁裸 string；繁体归 zh-CN）。
- 生成物禁手改：`bindings.ts` 只经 `cargo test` 重生成，`paraglide/` 只经 `pnpm i18n:compile`。
- 应用身份：包名 `ebook-studio-desktop` / 展示名 `Ebook Studio` / 反向域名 / 深链 scheme `ebook-studio` / 更新密钥 / 图标均已确定为本产品所有。
- `AGENTS.md` 的安全红线与架构不变量为最高效力规则，任何产品改动不得触碰。

**未决事实：**

- 移动端（iOS / Android）：当前仅桌面三平台，不承诺移动端可迁移；如要扩展需单独确认。

## Brand Commitments

- 名称：仓库 / 包名 `ebook-studio-desktop`，展示名 `Ebook Studio`。
- Ebook Studio 品牌：白色圆角 + 黑色衬线 E 图标（`src-tauri/app-icon.svg`），页脚只留版权行（`© {year} Alfred Clark. All rights reserved`）。
- 声音与语气：技术、精确、直接；README 简体中文为主 + 英文镜像（`docs/README_en.md`），文档中的术语与代码标识符保留英文。
- 字体：Geist Variable 为默认界面字体（可被用户系统字体偏好覆盖）。

## Evidence on Hand

- `README.md` + `docs/README_en.md`：功能清单、快速开始、页面一览、配置说明。
- `docs/ARCHITECTURE.md`（结构、分层、设计不变量）与 `docs/WORKFLOW.md`（命令契约、i18n、CI、验证矩阵）。
- `docs/images/`：按路由名存放真实界面截图（产品已有界面，待重拍）。
- **不存在且禁止编造**：用户评价、采用数据（star / 下载量）、客户案例、性能基准、企业背书。

## Product Principles

1. **身份干净**：主分支即产品本身、无演示业务、可长期维护；占位身份已整体替换为本产品，不留模板尾巴。
2. **生产级接线，拒绝演示业务**：能力以真实可用的接线存在，而不是文档承诺或玩具示例；示例代码放标签 / 文档，不占主线。
3. **页面即证据，但不是全部证据**：`/`（工作台最近项目与管线阶段）、`/projects`（项目增删改查）、`/settings`（配置持久化、外观、快捷键）、`/about`（诊断、更新）是**页面自身承载的那部分**能力的验证面，改动后必须仍然真实可用、可被截图取证；不为观感牺牲真实性。托盘菜单、深链、单实例聚焦、窗口状态、panic 兜底等能力**不要求有界面对象**——它们的证据是接线代码与测试，不是页面；不要为了让能力「可见」而给界面加多余控件。
4. **桌面三平台为准**：一切设计决策以 Windows / macOS / Linux 桌面使用场景为前提，除非另行确认扩展移动端。
5. **规则优先于便利**：`AGENTS.md` 的安全红线与架构不变量高于局部实现便利；生成物、门禁、配置唯一来源不因产品需求绕过。

## Accessibility & Inclusion

- 双语（en / zh-CN）为产品级要求，任何界面文案改动必须同步两份。
- 界面字体族 / 字重 / 字号为用户可调项（系统字体选择器 + 滑块），须保留可用性；自绘无边框标题栏必须保持窗口控制（置顶 / 最小化 / 最大化 / 关闭）可识别可操作。
- 未发现其他已确认的无障碍标准或用户需求；后续如需 WCAG 级别目标需单独确认。
