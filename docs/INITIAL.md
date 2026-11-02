# INITIAL.md — 派生项目定制与发布清单

> 面向 AI Agent 的**派生项目收尾剧本**：把本模板的副本变成一个身份干净、可发布的自有项目。
> 模板主分支已是干净起点：演示页（`/demo`）与业务示例（`/notes`）及其插件、权限、依赖已由模板侧
> 移除，派生项目不需要再删任何代码；需要这些接线示例（沙盒文件 / 对话框 / 通知 / 全局快捷键 /
> 网络请求 / 文件拖放）时，见上游仓库 `v0.3.0` 标签。
> **只做本地开发、暂不发布的，可以跳过 Phase B/C/D 的全部身份步骤**，直接开始写业务；
> 发布前再回来照单执行即可。
> 长期规范永远以 `AGENTS.md`（常驻规则）与 `docs/ARCHITECTURE.md`、`docs/WORKFLOW.md`（细节与流程）为准
> （规范的效力分级见 `AGENTS.md` 末节）；本文件只覆盖「模板 → 自有项目」这一段路，执行完毕可归档或删除。

---

## 0. 使用方式

```text
读 AGENTS.md + docs/ARCHITECTURE.md（结构与不变量）+ docs/WORKFLOW.md（命令与流程）→
Phase 0 收集变量（§1）→ Phase A→G 顺序执行 → 每个 Phase 收尾跑 pnpm format →
全部完成后跑 Phase F 验收矩阵（§8）+ pnpm validate → 按 Phase G（§9）汇报
```

**执行原则：**

1. **先收集变量，再动手**：Phase 0（§1）未完成前不得进入 Phase B；正文任何 `<VAR>` / 变量名
   出现处，必须先替换为 §1 已确认的真实值，禁止带着占位符改名、注册协议或轮换密钥。
2. **顺序不可换**：先改名与品牌（Phase B/C/D），后文档收尾（Phase E）。`about` 页测试与
   `productName` 耦合，Phase F 的残留扫描会顺带验证改名是否干净。
3. **只做身份化**：不擅自新增业务功能、不顺手清理「看起来没用」的常驻能力（§7 白名单）。
4. **文档与代码冲突时以代码为准**，并在汇报里标注冲突点（模板文档可能落后于代码）。
5. **生成物只允许命令重生成**：`src/libs/commands/bindings.ts` 靠 `pnpm test:cargo`，
   `src/libs/i18n/paraglide/` 靠 `pnpm i18n:compile`，两者禁止手改。
6. **缺信息就问，且带着候选问**：§1 任何一项无法从仓库推导时，先按规则推导候选，再一次性
   向用户确认（见 §1.2 收集流程）；必填项未确认前宁可阻塞，不得臆测。
7. **文档不带模板尾巴**：`README.md` / `docs/README_zh-CN.md` 收尾时只留「语言跳转 + 标题 +
   简介」；`AGENTS.md` / `docs/ARCHITECTURE.md` 不保留对上游模板仓库与 `v0.3.0` 示例标签的指引；
   模板截图一律删除。

---

## 1. Phase 0 — 变量收集（执行前必须完成）

> 本文件正文的占位符与变量名都指向本表；值来自「仓库推导 + 用户确认」，模板侧永远保持占位形态。
> 确认后的变量表请随手留档（PR 描述、分支上的临时文件均可），跨会话执行时可原样恢复，避免重复询问。

### 1.1 变量表

| 变量           | 规则与约束                                                                                          | 推导候选（默认值）                                                             | 主要落点                                                                                                                                                                                     |
| -------------- | --------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `PROJECT_SLUG` | kebab-case 包名；npm 规则：小写、无空格、不以 `.` / `_` 开头                                        | 仓库目录名，或 `git remote get-url origin` 的仓库名                            | `package.json` `name`；`Cargo.toml` `[package].name`；`cliff.toml` `remote.github.repo`；README / AGENTS 标题与简介                                                                          |
| `PRODUCT_NAME` | 展示名，可含空格；含非 ASCII 时须在打包产物上目视验证                                               | 用户指定；草稿 = `PROJECT_SLUG` 去掉尾部 `-desktop` / `-app` 后 Title Case     | `tauri.conf.json` `productName`；`app.html` `<title>`；窗口 `title`；`about` 页两处测试（`app-info-section.component.test.ts` / `about/page.component.test.ts`）                             |
| `IDENTIFIER`   | 反向域名，全局唯一（同机安装隔离 + store 路径隔离都认它）；避免 `com.tauri.*`；发布后再改等于换应用 | `io.github.<REPO_OWNER 小写>.<PROJECT_SLUG>`；有自有域名则 `<域名倒序>.<slug>` | `tauri.conf.json` `identifier`                                                                                                                                                               |
| `LIB_NAME`     | snake_case 的 Rust lib 名；合法标识符（不能数字开头，必要时加前缀）                                 | `<PROJECT_SLUG 的 - 换 _>_lib`                                                 | `Cargo.toml` `[lib].name`；`main.rs` 的 `lib::run()` 引用                                                                                                                                    |
| `SCHEME`       | 深链协议名，小写 + 连字符；OS 级全局注册、撞名即串台，必须含项目可辨识片段                          | `PROJECT_SLUG` 去掉尾部 `-desktop` / `-app`                                    | `tauri.conf.json` `plugins.deep-link.desktop.schemes`；`cores/deep_link.rs` 的 `SCHEME_PREFIX`（含文件内单测样例 URL）；`libs/hooks/deep-link.svelte.ts` 的 `DEEP_LINK_SCHEME`；两侧深链单测 |
| `CRASH_LOG`    | 崩溃日志文件名；临时目录（Win `%TEMP%` / Unix `$TMPDIR`）根部多应用共享，同名互覆盖                 | `<snake_case(PRODUCT_NAME)>_crash.log`                                         | `cores/system.rs` 的 `persist_fallback`；`AGENTS.md`（崩溃边界不变量）与 `docs/ARCHITECTURE.md`（崩溃边界）；README / docs 提及                                                              |
| `REPO_OWNER`   | 仓库归属（GitHub 用户名，注意与账号大小写一致）                                                     | `git remote get-url origin` 的 owner 段；无 remote 则问用户                    | `package.json` 三处 URL（homepage / repository / bugs）；`cliff.toml` `[remote.github].owner`；`tauri.conf.json` `plugins.updater.endpoints`                                                 |
| `DESCRIPTION`  | 一句话描述（英文、单行）                                                                            | 用户指定；草稿 `<PRODUCT_NAME> desktop application based on Tauri and Svelte.` | `package.json` / `Cargo.toml`                                                                                                                                                                |
| `KEYWORDS`     | 字符串数组：小写、无空格；Cargo 侧 ≤ 5 个且每个 ≤ 20 字符                                           | `["tauri", "desktop", <slug 分词>…]`（替换模板的 `template`）                  | `package.json` / `Cargo.toml`                                                                                                                                                                |
| `INIT_VERSION` | 语义化版本；新项目起点（与模板自身版本无关）                                                        | 默认 `0.1.0`；用户已有版本规划则从其                                           | `package.json` / `tauri.conf.json` / `Cargo.toml` 三处                                                                                                                                       |

**附带确认项（与上表一起问，未确认前不落盘）：**

- **作者 / 版权**：默认沿用模板作者（模板 `package.json` 的 `author`）；更换时同步 `LICENSE`。
- **许可证**：模板为 `GPL-3.0-only`（见 §9 第 4 条）；是否更换由用户决定，闭源分发前必须评估传染性义务。
- **设计资产**：是否保留 impeccable 相关文件（`PRODUCT.md` / `DESIGN.md` / `.impeccable/`）。默认保留；不保留则按 §2 第 4、5 步删除。

**示例（虚构，仅示意格式，勿直接采用）：** `acme-notes-desktop` → `Acme Notes` →
`io.github.acme.acme-notes-desktop` → `acme_notes_desktop_lib` → scheme `acme-notes` →
`acme_notes_crash.log`。

**推导命令参考：**

```bash
basename "$(git rev-parse --show-toplevel)"                 # PROJECT_SLUG 候选：仓库目录名
git remote get-url origin 2>/dev/null | sed -E 's#\.git$##' # REPO_OWNER / PROJECT_SLUG 候选：owner/repo
```

### 1.2 收集流程（Agent 必须执行）

1. **先推导，后询问**：按「推导候选」列的规则 / 命令算出候选值，不要空手问用户；无法推导的项
   标出「需用户提供」。
2. **一次性成组确认**：把候选值与来源整理成一张表，用一次成组提问交给用户（可整体接受或逐项
   覆盖）；不要一个变量一个问题。优先使用宿主 Agent 的提问机制（如选项式提问），无该机制时
   直接在对话里列出表格请用户逐项确认。
3. **缺信息处理**：必填项（`PROJECT_SLUG` / `PRODUCT_NAME` / `REPO_OWNER` / `IDENTIFIER` /
   `SCHEME`）不得臆测，问不到就阻塞；可推导项（`LIB_NAME` / `CRASH_LOG` / `KEYWORDS` /
   `INIT_VERSION`）采用推导默认，并在汇报里标注「取默认值」。
4. **回填与留档**：确认后把真实值填入 1.1 表，并在收尾汇报中完整复述。
5. **改值要全量回查**：变量出现在包名、证书标识、协议注册、签名端点等多处；一旦确认后修改，
   必须按「主要落点」列全部重新替换一次，不允许只改一半。

**成组询问模板（可直接改写后发给用户）：**

```text
初始化前需要确认以下变量（括号内为按仓库推导的候选，可直接接受或逐项修改）：
1. PROJECT_SLUG（包名，kebab-case）：<候选>（来源：<仓库目录名 / git remote>）
2. PRODUCT_NAME（显示名）：<候选>
3. IDENTIFIER（反向域名，全局唯一）：<候选>
4. REPO_OWNER（GitHub 用户名）：<候选>
5. DESCRIPTION（一句话英文描述）：<候选>
6. SCHEME（深链协议）：<候选>
7. 设计资产（PRODUCT.md / DESIGN.md / .impeccable/）：<保留 / 删除>（默认保留）
其余按推导默认：LIB_NAME=<推导值>、CRASH_LOG=<推导值>、KEYWORDS=<推导值>、INIT_VERSION=0.1.0；
作者与许可证默认沿用模板（GPL-3.0-only），设计资产默认保留；如需更换请说明。
```

---

## 2. Phase A — 建立基线

1. 起点：GitHub `Use this template` 建库，或 `git clone` 后 `rm -rf .git && git init` 另起干净历史
   （二选一即定，模板历史是否保留溯源自选）。
2. `pnpm install --frozen-lockfile`；确认 `git status` 干净，直接在 `main` 分支上执行初始化，不必另拉分支。
3. 基线自检：模板原状下 `pnpm validate` 应全绿，红则先修环境（Node ≥ 24 / pnpm 12.15.0 /
   Rust 1.99.0）再继续，避免把环境问题混进初始化提交。
4. 删除设计资产（仅 §1 确认删除时执行）：删除 `PRODUCT.md`、`DESIGN.md`、`.impeccable/`。
5. 去掉设计引用（仅 §1 确认删除时执行）：去掉 `src/routes/layout.css` 里指向 `DESIGN.md` 的注释。

**DoD：** 依赖装好、工作区干净、模板基线可跑，设计资产已按 §1 取舍。

---

## 3. Phase B — 改标识（漏任一项即带着模板身份发布）

> 下表 `PROJECT_SLUG` 等均指 §1 已确认值。

| 序号 | 文件                                                                           | 改动                                                                                                                                                                                                                                                                                                     |
| ---- | ------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| B1   | `package.json`                                                                 | `name` → `PROJECT_SLUG`；`version` → `INIT_VERSION`；`description`；`homepage`；`repository.url`；`bugs.url`；`keywords`                                                                                                                                                                                 |
| B2   | `src-tauri/Cargo.toml`                                                         | `[package]` 的 `name` / `version` / `description` / `repository` / `keywords`；`[lib].name` → `LIB_NAME`                                                                                                                                                                                                 |
| B3   | `src-tauri/src/main.rs`                                                        | `tauri_desktop_template_lib::run()` → `LIB_NAME::run()`（B2 改名的连带引用，漏改即编译失败）                                                                                                                                                                                                             |
| B4   | `src-tauri/tauri.conf.json`                                                    | `productName` / `identifier` / `version` / `app.windows[0].title`                                                                                                                                                                                                                                        |
| B5   | `cliff.toml`                                                                   | `[remote.github]` 的 `owner` / `repo`                                                                                                                                                                                                                                                                    |
| B6   | `src/app.html`                                                                 | `<title>`（首帧标题，SvelteKit 之后不再覆盖）                                                                                                                                                                                                                                                            |
| B7   | `src/libs/i18n/messages/*.json`                                                | `page_home_intro`（含模板自述，改为本产品一句话介绍，en + zh-CN 两份）                                                                                                                                                                                                                                   |
| B8   | `src/tests/component/...`                                                      | `routes/(main)/about/_components/app-info-section.component.test.ts` 与 `routes/(main)/about/page.component.test.ts` 的 `productName` 期望值                                                                                                                                                             |
| B9   | `README.md` / `docs/README_zh-CN.md`                                           | 标题与简介（两文件同改，避免漂移）；删除对上游模板仓库与 `v0.3.0` 示例标签的引用；其余章节不在此处修补，Phase E 会整体瘦身                                                                                                                                                                               |
| B10  | `AGENTS.md` / `docs/ARCHITECTURE.md`                                           | `AGENTS.md` 头部模板自述行；`docs/ARCHITECTURE.md` 的目录树首行目录名与「上游示例索引」的 `v0.3.0` 指引                                                                                                                                                                                                  |
| B11  | `PRODUCT.md` / `DESIGN.md` / `.impeccable/design.json`（仅保留设计资产时执行） | `Tauri Desktop Template` → `<PRODUCT_NAME>`（`DESIGN.md` 标题写作 `Design System: <PRODUCT_NAME>`）；`design.json` 的 `title` 与内嵌示例名同步改，或重跑生成流程重建 sidecar；`PRODUCT.md` 的包名、展示名、作者、许可证按 §1 确认值更新，模板身份占位行同步改写，并清理其中指向 `docs/INITIAL.md` 的文字 |

**DoD：**

```bash
rg -i "tauri-desktop-template|tauri_desktop_template|Tauri Desktop Template" \
  --glob '!**/node_modules/**' --glob '!**/images/**' --glob '!**/*.lock' .
# 期望：无命中（排除 node_modules / images / 锁文件；锁文件由依赖安装与构建自动更新）
pnpm check:sync && pnpm validate
```

---

## 4. Phase C — 换协议 / 密钥 / 兜底常量

**C1 深链 scheme（三处 + 两侧测试，OS 级全局注册，撞名即串台）：**

- `src-tauri/tauri.conf.json` → `plugins.deep-link.desktop.schemes` 改为 `SCHEME`
- `src-tauri/src/cores/deep_link.rs` → `SCHEME_PREFIX = "<SCHEME>://"`（含文件内单测的样例 URL）
- `src/libs/hooks/deep-link.svelte.ts` → `DEEP_LINK_SCHEME = "<SCHEME>"`
- `src/tests/unit/libs/hooks/deep-link.test.ts` 与 `cores/deep_link.rs` 的单测断言同步改

**C2 崩溃日志名：** `src-tauri/src/cores/system.rs` 的 `persist_fallback` 落盘文件名改为
`<CRASH_LOG>`（临时目录根部多应用共享，同名互覆盖；Win `%TEMP%` / Unix `$TMPDIR`）；连带改 `AGENTS.md`（崩溃边界不变量）与
`docs/ARCHITECTURE.md`（崩溃边界一节）及 README / docs 的提及。

**C3 更新密钥与端点（不换则验签失败，发版流水线必挂）：**

1. 生成新密钥对，落到用户目录下与项目同名的文件夹（`-w` 会同时写出私钥
   `~/<PROJECT_SLUG>/<PROJECT_SLUG>.key` 与公钥 `~/<PROJECT_SLUG>/<PROJECT_SLUG>.key.pub`，
   父目录自动创建；同名密钥已存在时会拒绝覆盖，确需轮换再加 `--force`）：

   ```bash
   pnpm tauri signer generate -w ~/<PROJECT_SLUG>/<PROJECT_SLUG>.key
   ```

   密码提示**直接回车两次**（留空即可：空密码密钥在更新签名链路与 CI 中都能正常使用，
   因此无需再配 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`）。执行环境没有可交互终端时（Agent 管道 / CI），
   用 `--ci` 等价跳过提示：`pnpm tauri signer generate --ci -w ~/<PROJECT_SLUG>/<PROJECT_SLUG>.key`。

2. 公钥**自动填入**：读取 `~/<PROJECT_SLUG>/<PROJECT_SLUG>.key.pub` 的内容，替换
   `src-tauri/tauri.conf.json` 的 `plugins.updater.pubkey`（生成器 stdout 也会打印同一公钥）。
   用户若选择自行生成密钥，则由用户手动替换公钥，其余步骤不变。
3. `plugins.updater.endpoints` 改为新仓库的 `releases/latest/download/latest.json`。
4. 私钥配入新仓库 Secret `TAURI_SIGNING_PRIVATE_KEY`（`.github/workflows/release.yml` 引用名）：
   粘贴 `~/<PROJECT_SLUG>/<PROJECT_SLUG>.key` 的完整内容；空密码密钥不需要密码 Secret。
5. **提醒用户保存好私钥**（Agent 执行至此必须显式提示）：该文件是唯一能为更新签名的凭据，
   丢失后已发布的旧版本将无法验证任何新更新，请备份到密码管理器 / 加密存储。**密钥文件全程不入仓库**：
   `~` 下的路径天然在仓库外，不要复制进工作区、不要提交、不要粘贴进日志或聊天记录。

**DoD：** `cargo test`（深链单测）+ `pnpm test:vitest src/tests/unit/libs/hooks/deep-link.test.ts` 通过；
`rg "tdt://|my_app_crash" src src-tauri docs README.md AGENTS.md` 无命中（模板旧值已全部替换）；
`git ls-files | rg '\.(key|pub)$'` 无命中，`~/<PROJECT_SLUG>/<PROJECT_SLUG>.key` 已生成且已提醒用户备份。

---

## 5. Phase D — 品牌与图标

1. **替换图标源**：准备本项目标识，直接覆盖 `src-tauri/app-icon.svg`——**推荐正方形透明底 SVG**（标称尺寸
   任意，矢量逐档渲染）；**PNG 备选请用 1024×1024**（`icon.icns` 内嵌 1024 档、iOS 亦生成 1024，过小会被
   放大）。源文件须自包含（不引外部字体 / 图片）；`tauri icon` 仅接受「正方形 PNG 或 SVG」。换用 PNG 或
   改文件名时，同步 `package.json` 的 `tauri:icon` 脚本源路径（模板脚本指向 `src-tauri/app-icon.svg`）。
2. **重生成平台图标**：`pnpm tauri:icon`（脚本自带 `rimraf src-tauri/icons/{ios,android}`；
   `src-tauri/icons/` 下十余个二进制文件会被整体覆盖，属预期变更）。**随后同步托盘图标**：
   `src-tauri/icons/tray.png` 不随该命令重生成（`cores/tray.rs` 以 `include_bytes!` 独立引用），
   必须手动换上本产品的 64×64 图标——最简做法是复制刚生成的 64×64 图：
   `cp src-tauri/icons/64x64.png src-tauri/icons/tray.png`（托盘图标也可独立设计，保持 64×64 PNG 即可）。
3. **同步 favicon**：`src/app.html` 经 `/favicon.png` 引用（CSP 只放行 self），同样从刚生成的
   图标集复制，保证全局图标同源：`cp src-tauri/icons/64x64.png static/favicon.png`
   （64×64 兼顾高 DPI 标签页；想省体积也可用 `32x32.png`）。
4. **内联图标组件**：`src/assets/icons/app-icon.svelte` 换成本产品标识（单色 + `currentColor`，
   明暗主题自适应；`aria-hidden`，纯装饰）。
5. **清掉模板技术栈图标**：删除 `src/assets/icons/{svelte,tauri,vite}-icon.svelte`，并把
   `src/routes/(main)/+page.svelte` 英雄区改为只用 `AppIcon`（模板技术栈 logo 不应出现在产品里）。
6. **截图**：`docs/images/` 现有三张模板截图（`home` / `settings` / `about`，标题栏与导航均为模板
   身份）；初始化时删除整目录并移除 README 的截图段，产品有真实界面后按路由名重拍。

**DoD：** `pnpm dev` 目视首页（英雄区只剩 `AppIcon` + 应用名 + 简介）、标题栏与浏览器标签页图标正常；
`pnpm tauri:dev` 目视托盘图标为新图；`rg "svelte-icon|tauri-icon|vite-icon|svelte\.svg|tauri\.svg|vite\.svg" src static` 无命中。

---

## 6. Phase E — 文档收尾

| 文件                                                      | 改动                                                                                                                                                                                                                                                                                  |
| --------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `README.md`                                               | **只保留三段：语言跳转、标题、简介**（外加一行 TODO 注释标记待补章节）。模板版长文——Features / Tech stack / Quickstart / Pages / Configuration / Workflows / Structure——整体删除，项目落地后再补，避免带着模板自述发布                                                                |
| `docs/README_zh-CN.md`                                    | 与 README 同源的对应改动（两文件必须同时改）                                                                                                                                                                                                                                          |
| `AGENTS.md` / `docs/ARCHITECTURE.md` / `docs/WORKFLOW.md` | `AGENTS.md` 头部模板自述；`docs/ARCHITECTURE.md` 的目录树首行、「上游示例索引」的 `v0.3.0` 指引、崩溃日志名、图标源文件路径（换格式时）；调整过门禁时同步 `AGENTS.md` 的「效力分级与定制」；**凡是引用 `docs/INITIAL.md` 的文字：本文件删除后即悬空引用，须一并清理**；选择留档则保留 |
| `docs/images/`                                            | 三张模板截图（`home` / `settings` / `about`）属模板身份，初始化时删除；产品有真实界面后按路由名重拍并恢复 README 截图段                                                                                                                                                               |
| `INITIAL.md`（本文件）                                    | 执行完成后删除，或在文件头标注「已于 <日期> 执行，结果见 commit <hash>」留档（留档时上文悬空引用可豁免）                                                                                                                                                                              |

> Agent 提示：Phase E 是最容易被跳过的一步。若只允许做一件事，也要保证
> `rg -i "tauri-desktop-template|template" README.md docs AGENTS.md` 不会把用户引导回模板身份；若本文件将被删除，
> `rg -n "docs/INITIAL.md" AGENTS.md README.md docs` 也不得留下悬空引用。

**README 初始化后的期望形态（可直接照此落地）：**

```markdown
English | [简体中文](docs/README_zh-CN.md)

# <PROJECT_SLUG>

<PRODUCT_NAME> desktop application based on Tauri v2 + SvelteKit 2 + Svelte 5.

<!-- TODO: add Features / Tech stack / Quickstart / Pages / Structure / Release sections once the product lands -->
```

---

## 7. 常驻能力白名单（初始化后仍须存活）

| 能力                                                     | 存活理由                                                 |
| -------------------------------------------------------- | -------------------------------------------------------- |
| `log`（plugin-log / capability `log:default`）           | 未注册 `.failed()` 的失败上报通道，删除即静默丢错        |
| `opener`                                                 | about 页项目外链（前端 `openExternal` 仅放行 `http(s)`） |
| `clipboard-manager`（仅 Rust 侧使用，不声明 capability） | about 页复制系统信息经后端命令走 Rust API                |
| `deep-link` / `single-instance`                          | 深链与二次启动聚焦，改 scheme 见 §4 C1                   |
| `global-shortcut`（仅 Rust 侧使用，不声明 capability）   | 显示应用全局快捷键（配置驱动动态注册）                   |
| `updater` / `autostart` / `window-state`                 | 桌面外壳三大件，仅非移动端启用                           |
| `store`                                                  | `config.json` 持久化（语言 / 通用设置）                  |

**禁止**「顺手清理」上表任一项；也禁止把 `store` / `autostart` / `window-state` / `single-instance` / `clipboard-manager` / `global-shortcut`
补进 `capabilities/plugins.json`（它们仅 Rust 侧注册使用，无前端 JS 直调，属刻意最小授权）。

---

## 8. Phase F — 验收矩阵（全部完成后逐项核对）

### 8.1 残留扫描

```bash
rg -i "tauri-desktop-template|tauri_desktop_template|Tauri Desktop Template" . \
  --glob '!**/node_modules/**' --glob '!**/*.lock'                        # 无命中（模板身份清零）
rg "tdt://|my_app_crash" src src-tauri docs README.md AGENTS.md           # 无命中（<SCHEME> / <CRASH_LOG> 已替换）
rg -n "v0.3.0" README.md AGENTS.md docs                                   # 无命中（上游示例指引已删）
test "$(wc -l < README.md)" -le 10 && echo "README 已瘦身"                # README / docs 各 ≤ 10 行
rg -n "docs/INITIAL.md" AGENTS.md README.md docs 2>/dev/null             # 本文件删除后须无命中；留档则豁免
```

### 8.2 计数基线（模板现值 → 定制后，实测参考）

| 指标                                     | 模板现值                                | 定制后                            |
| ---------------------------------------- | --------------------------------------- | --------------------------------- |
| `collect_commands!` 注册命令数           | 11                                      | 不变（新增业务命令时增加）        |
| `capabilities/plugins.json` 权限项       | 4                                       | 不变                              |
| 后端 `tauri-plugin-*` 依赖               | 12                                      | 不变                              |
| 前端 `@tauri-apps/plugin-*` 依赖         | 3                                       | 不变                              |
| 前端测试文件（`src/tests/**/*.test.ts`） | 32                                      | 不变                              |
| 前端文案键（`en.json`）                  | 150                                     | 不变                              |
| `src-tauri/locales/*.yml`                | 1（`tray.yml`）                         | 不变                              |
| 导航标签（`nav-tabs.ts`）                | 3                                       | 不变（新增页面时增加）            |
| `README.md` 行数                         | 约 180                                  | ≤ 10（跳转 + 标题 + 简介 + TODO） |
| `docs/images/`                           | 3 张模板截图（home / settings / about） | 删除或重拍（只含真实界面）        |

说明：测试文件数按 `find src/tests -name '*.test.ts' | wc -l` 计数；文案键数按 `en.json` 顶层键数计数。改动测试或文案后同步更新本表。

### 8.3 质量门禁

```bash
pnpm format      # 必跑，钩子只看暂存文件
pnpm validate    # = test（vitest + cargo test + 绑定重生成）+ lint（前后端）+ check（svelte-check）
git diff src/libs/commands/bindings.ts   # 应已提交且无未预期的改动
```

### 8.4 端到端联调（`pnpm tauri:dev`）

- [ ] 标题栏显示 `<PRODUCT_NAME>`；无边框窗口拖拽 / 置顶 / 最大化 / 关闭正常
- [ ] 三个页面（`/`、`/settings`、`/about`）可访问；about 页应用名与版本正确
- [ ] 切换语言 → 重启 → 语言保持（`config.json` 持久化链路未破）
- [ ] `<SCHEME>://settings` 唤起应用并跳转（深链三处同源）
- [ ] 托盘菜单可用且托盘图标已替换为新图标（64×64 `tray.png`）、开机自启、更新检查面板可用
- [ ] 崩溃兜底：人为 panic 后临时目录（Win `%TEMP%` / Unix `$TMPDIR`）下 `<CRASH_LOG>` 生成

---

## 9. Phase G — 首次提交与汇报

1. **提交**：Conventional Commits、英文小写；初始化整体算一件事，合为一条提交：

   ```text
   chore(init): initialize <PROJECT_SLUG> from template
   ```

   提交前必跑 `pnpm format` + `pnpm validate`；禁止自动提交（由开发者手动执行）。

2. **CI**：推送后确认 `ci.yml` 全绿；`main` 受保护时走 PR + Code Review + Squash 合并。
3. **汇报格式**（Agent 收尾输出）：

   ```text
   变量确认：§1 全表取值与来源（含取默认值的项与用户覆盖项）
   改动清单：按 Phase 罗列的「文件 / 动作 / 理由」
   验证结果：执行的命令 + 结果（validate 全绿 / rg 清零 / 计数基线对齐）
   未完成项：如「真实界面截图待补」「更新私钥尚未配 Secret」，逐条说明阻塞原因
   下一步建议：首个业务功能的入口（libs → commands → features 的落点）
   ```

4. **协议提醒**：本模板为 `GPL-3.0-only`；派生项目（含闭源分发）须先评估传染性义务。

---

<!-- 维护说明：本文件与模板主分支结构同步维护（改规范文档（`AGENTS.md` / `docs/ARCHITECTURE.md` / `docs/WORKFLOW.md`）或仓库结构时，检查 §1~§6 落点表与 §8.2 计数基线）；派生项目执行后删除或留档 -->
