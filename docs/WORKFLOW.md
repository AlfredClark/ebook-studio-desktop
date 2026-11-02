# WORKFLOW.md — 命令、流程与验证

> 按需文档：跑命令、改文案、加后端命令、发版、验收时读本文件；日常规则结论见 `AGENTS.md`。
> 所有命令以仓库根目录为基准执行；优先使用以下命令，禁止另起等效命令。
> 包管理器为 `pnpm@12.15.0`（以 `package.json` 的 `packageManager` 为准），Node `>=24`；Rust 工具链锁定 `1.99.0`（见 `rust-toolchain.toml`）。此处为高频清单，完整脚本以 `package.json` 为准。

---

## 常用命令

```bash
# ---- 环境初始化 ----
pnpm install --frozen-lockfile     # 按锁文件安装依赖（不要用 npm / yarn）

# ---- 本地启动 ----
pnpm tauri:dev                     # 完整桌面联调：Vite（固定端口 1420）+ Tauri 窗口
pnpm dev                           # 仅启动前端（浏览器中 Tauri 命令不可用，会自动降级）

# ---- 代码检查与格式化（每次修改后必跑 pnpm format + pnpm validate） ----
pnpm format                        # 自动修复两端：Prettier + ESLint + cargo fmt
pnpm lint                          # 前端 + 后端全量检查
pnpm lint:frontend                 # prettier --check . + eslint .
pnpm lint:backend                  # cargo fmt --check + clippy -D warnings + cargo check
pnpm check                         # svelte-kit sync + svelte-check（严格 TS 检查）
pnpm validate                      # test（含 vitest + cargo test 与绑定重生成）+ lint + check（每次修改后必跑）

# ---- 构建 ----
pnpm build                         # 仅构建前端，adapter-static 输出 SPA 到 build/（需先 i18n:compile）
pnpm tauri:build                   # 完整打包；tauri:build:local 仅编译不打包（--no-bundle）
pnpm clean                         # 清理构建产物；clean:frontend / clean:backend 分侧清理

# ---- 测试 ----
pnpm test                          # vitest + cargo test 单次运行；test:vitest 仅前端单测；test:watch 为监听模式
pnpm test:cargo                    # 后端测试，同时重新生成 src/libs/commands/bindings.ts

# ---- 依赖升级（禁止裸 pnpm update / cargo update，必须经以下脚本以触发对齐检查） ----
pnpm update:frontend               # 仅前端：caret 范围内升级（不跨 major）
pnpm update:backend                # 仅后端：semver 兼容范围内升级（不跨 major）
pnpm update:all                    # 双端一起升 + check:sync 对齐检查；之后必跑 validate，单独提交 chore(deps:update)

# ---- 依赖安全审计（可选，不纳入 CI 强制门禁） ----
pnpm audit                         # 前端依赖高危漏洞检查（可选）
pnpm audit:backend                 # 后端 cargo deny 安全检查（可选，本地需安装 cargo-deny）

# ---- 国际化 / 图标 / 发布 ----
pnpm i18n:compile                  # 编译前端国际化文案，改完 messages/ 后必须执行
pnpm i18n:translate                # 走 inlang 机器翻译
pnpm tauri:icon                    # 由 src-tauri/app-icon.svg 生成各平台图标
pnpm changelog                     # git-cliff 生成变更日志；changelog:preview 仅预览未发布部分
pnpm release                       # bumpp 联动升级三处版本号（package.json + tauri.conf.json + Cargo.toml）
```

---

## 国际化流程

前端在 `src/libs/i18n/project.inlang/` 下改文案 → `pnpm i18n:compile` → 用 `m.<键>(...)` 按键取值、`setLocale(locale)` 切换（整页重载）；启动对齐经 `libs/i18n/locale.svelte.ts` 的 `alignLocale`（写 localStorage 缓存 + 响应式重渲染，不重载）——不重载的语言变更只允许走它，禁止裸调 `setLocale(locale, { reload: false })`；后端改 `src-tauri/locales/*.yml` → `rust_i18n::t!(...)` 取文案。

- `paraglide/` 生成物已忽略提交，禁止编辑；禁止从外部导入其内部文件。
- `project.inlang/paraglide.config.ts` 与 `settings.json` 已被该目录 `.gitignore` 放行，直接 `git add` 即可，无需 `git add -f`。
- 语言数据流（持久化、水合、切换时序）见 `docs/ARCHITECTURE.md` 的「设计不变量与原因 → 语言数据流」。

---

## 命令契约流程

有对应业务时 `features/<name>.rs` 实现业务（`features/mod.rs` 声明 `pub mod <name>;`），无对应业务（纯通用能力）直接调 `cores/` → `commands/<name>.rs` 薄封装（`CommandResult` + 双注解 `#[tauri::command]` / `#[specta::specta]` + 进 `collect_commands!`）→ `cargo test` 重生成 `bindings.ts` → 前端链式 API 调用 → 联调。

禁止跳过生成步骤直接改代码：必须先改 `commands/`（有对应业务时先改 `features/` 对应业务文件），再 `cargo test` 重生成 `bindings.ts`，最后改前端调用；禁止手改 `bindings.ts`。

---

## 发版流程

必须由开发者手动发版：`main` 受保护 → 提 PR → CI 全绿 → Code Review → Squash 合并 → `pnpm release` 联动三处版本号（`package.json` + `src-tauri/tauri.conf.json` + `src-tauri/Cargo.toml`）→ 打 tag `vX.Y.Z`（须与 `tauri.conf.json` 一致）→ `release.yml` 自动打包 → `pnpm changelog` 生成日志。

发布流水线会校验三处版本号与 `v*` 标签一致。

---

## 构建与忽略

- 构建产物（`target/`、`build/`、`.svelte-kit/`、`src-tauri/gen/`、`node_modules/`、`src/libs/i18n/paraglide/`）均已忽略。
- `bindings.ts` 虽是生成物但**需要提交**；`Cargo.lock` 需要提交；`src-tauri/app-icon.svg` 为图标源文件（推荐正方形透明底 SVG，标称尺寸任意；PNG 备选 ≥1024；`pnpm tauri:icon` 生成各平台图标）。
- Vite 固定端口 `1420`，忽略监听 `src-tauri/**`，`clearScreen: false` 以保留 Rust 日志。
- 主窗口 `visible: false` 与统一 `show` 的时序约束见 `docs/ARCHITECTURE.md` 的「窗口与构建」。

---

## 验证矩阵与回归 checklist

> 硬性规则：任何文件改动完成后必须先 `pnpm format`，再 `pnpm validate`；两者全绿前不得宣称完成、不得提交。钩子只覆盖暂存文件，是最后防线而非替代。

| 修改范围                                               | 最低验证要求                                                                                                   |
| ------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------- |
| 仅前端 UI / 文案 / 配置                                | `pnpm format` + `pnpm validate`（含 `prettier --check`、`eslint`、`svelte-check`、`vitest`、`cargo test`）通过 |
| 前端逻辑 / commands 封装                               | 上述 + `pnpm test:vitest src/tests/<相关用例>` 通过                                                            |
| 后端 features / cores / commands                       | `pnpm test:cargo`（重写绑定 + 后端测试）+ `pnpm format` + `pnpm validate` 通过                                 |
| 命令契约变更（新增 / 改签名 / 改 `collect_commands!`） | 上述 + 确认 `git diff src/libs/commands/bindings.ts` 有同步更新 + 前后端联调                                   |
| 前端文案变更（`messages/`）                            | `pnpm i18n:compile` + `pnpm format` + `pnpm validate` 通过（跳过 compile 会导致 `build` / CI 失败）            |
| 发布相关（版本号 / CI / 打包）                         | `pnpm build` + `pnpm tauri:build:local --no-bundle`（按需）+ CI 全绿                                           |

**回归 checklist（提交前逐项确认）：**

- [ ] 修改功能以外的旧功能主流程不受影响
- [ ] 新增逻辑有单测覆盖核心分支（含异常分支；前端测试集中 `src/tests/` 按源路径镜像，后端 `cargo test`）
- [ ] 无 `console.log / print / debugger` 残留（失败上报走 plugin-log / 后端 log）
- [ ] 无硬编码 URL / 密钥 / 本地绝对路径（外部 URL 已同步 capabilities + CSP）
- [ ] 改文案已补跑 `i18n:compile`；`validate` 已含 `cargo test`（含绑定重生成），无需另跑

**提交前钩子**（`lint-staged`，最后防线）：`*.{js,ts,svelte}` → `prettier --write` + `eslint --fix`；`*.{json,md,html,css,yml,yaml}` → prettier（范围须与 CI 的 `prettier --check .` 一致）；`**/*.rs` → 暂存文件 `cargo fmt` + 工作区级 `clippy -D warnings`。

---

## CI（`.github/workflows/ci.yml`）

在 `main` 分支上按变更路径触发：

- **前端**：`i18n:compile` → `lint:frontend` → `check` → `test` → `build`。
- **后端**：`fmt --check` → `clippy -D warnings`（警告即错误）→ `check` → `test` → `git diff --exit-code src/libs/commands/bindings.ts`（绑定同步校验；`bindings.ts` 已纳入 backend 的 `changes` 过滤器，单改它也会触发后端校验）。
- 门禁作业（`ci-passed`）同时拦截 `changes`（变更检测）作业的失败——它失败时前后端都会被跳过，只查前后端会漏放。

发布流水线（`.github/workflows/release.yml`）会校验三处版本号（`package.json` + `tauri.conf.json` + `Cargo.toml`）与 `v*` 标签一致。

---

<!-- 维护说明：本文件承接原 AGENTS.md 的常用命令、特殊流程与测试验证章节；脚本或门禁变更时优先更新本文件，再改配置 -->
