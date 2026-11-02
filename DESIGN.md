---
name: Tauri Desktop Template
description: 生产级 Tauri + Svelte 桌面应用基座的视觉系统：中性石墨灰基座、刚好够用的密度、ring 贴合分层
colors:
  # ── 中性基座（浅色） ──
  paper: "oklch(1 0 0)"
  paper-bright: "oklch(0.985 0 0)"
  graphite-ink: "oklch(0.145 0 0)"
  graphite: "oklch(0.205 0 0)"
  graphite-soft: "oklch(0.556 0 0)"
  steel: "oklch(0.97 0 0)"
  hairline: "oklch(0.922 0 0)"
  ring-steel: "oklch(0.708 0 0)"
  # ── 中性基座（深色） ──
  charcoal: "oklch(0.145 0 0)"
  steel-dark: "oklch(0.269 0 0)"
  graphite-ghost: "oklch(0.708 0 0)"
  hairline-dark: "oklch(1 0 0 / 10%)"
  # ── 语义色 ──
  signal-red: "oklch(0.577 0.245 27.325)"
  signal-red-bright: "oklch(0.704 0.191 22.216)"
  # ── 配色主题主色（七套鲜艳主题取浅色主色；三套暗色美学取深色主色，即各自招牌值） ──
  ocean: "oklch(0.55 0.22 256)"
  teal: "oklch(0.6 0.14 192)"
  emerald: "oklch(0.6 0.165 150)"
  amber: "oklch(0.58 0.15 80)"
  tangerine: "oklch(0.6 0.19 52)"
  rose: "oklch(0.585 0.21 350)"
  amethyst: "oklch(0.55 0.225 292)"
  nord: "oklch(0.78 0.07 210)"
  dracula: "oklch(0.72 0.15 305)"
  tokyo-night: "oklch(0.7 0.14 262)"
typography:
  headline:
    fontFamily: "Geist Variable, sans-serif"
    fontSize: "1.5rem"
    fontWeight: 500
    lineHeight: 1.333
  title:
    fontFamily: "Geist Variable, sans-serif"
    fontSize: "1rem"
    fontWeight: 500
    lineHeight: 1.375
  body:
    fontFamily: "Geist Variable, sans-serif"
    fontSize: "0.875rem"
    fontWeight: 400
    lineHeight: 1.43
  label:
    fontFamily: "Geist Variable, sans-serif"
    fontSize: "0.875rem"
    fontWeight: 500
    lineHeight: 1.43
  caption:
    fontFamily: "Geist Variable, sans-serif"
    fontSize: "0.75rem"
    fontWeight: 400
    lineHeight: 1.333
rounded:
  sm: "6px"
  md: "8px"
  lg: "10px"
  xl: "14px"
  "2xl": "18px"
  "3xl": "22px"
  "4xl": "26px"
  full: "9999px"
spacing:
  "1": "4px"
  "2": "8px"
  "3": "12px"
  "4": "16px"
  "5": "20px"
  "6": "24px"
  "8": "32px"
components:
  button-primary:
    backgroundColor: "{colors.graphite}"
    textColor: "{colors.paper-bright}"
    rounded: "{rounded.lg}"
    height: "32px"
    padding: "0 10px"
  button-outline:
    backgroundColor: "{colors.paper}"
    textColor: "{colors.graphite-ink}"
    rounded: "{rounded.lg}"
    height: "32px"
    padding: "0 10px"
  button-ghost:
    textColor: "{colors.graphite}"
    rounded: "{rounded.md}"
    size: "28px"
  button-destructive:
    textColor: "{colors.signal-red}"
    rounded: "{rounded.lg}"
    height: "32px"
    padding: "0 10px"
  input:
    backgroundColor: "transparent"
    textColor: "{colors.graphite-ink}"
    rounded: "{rounded.lg}"
    height: "32px"
    padding: "0 10px"
  card:
    backgroundColor: "{colors.paper}"
    textColor: "{colors.graphite-ink}"
    rounded: "{rounded.xl}"
    padding: "16px"
  badge:
    backgroundColor: "{colors.graphite}"
    textColor: "{colors.paper-bright}"
    rounded: "{rounded.full}"
    height: "20px"
    padding: "0 8px"
  switch-on:
    backgroundColor: "{colors.graphite}"
    rounded: "{rounded.full}"
    width: "32px"
    height: "18.4px"
  tooltip:
    backgroundColor: "{colors.graphite-ink}"
    textColor: "{colors.paper}"
    rounded: "{rounded.md}"
    padding: "6px 12px"
---

# Design System: Tauri Desktop Template

## Overview

**Creative North Star: "The Instrument Bench"（仪器工作台）**

这个界面是一件被校准过的仪器，而不是一页被设计过的画面。每个控件都有刻度与用途：设置页的每个开关都对应一条真实生效的链路，下拉里的每个值都能即时改变界面本身（主题、配色、布局、字体），标题栏的每个按钮都真实地控制窗口。界面的可信度来自"它说什么就做什么"，而不是来自装饰——这与产品「生产级接线 + 刻意干净主分支」的定位是同一件事的两面。

基调是中性：默认配色是一套石墨灰（Paper / Graphite / Steel），没有品牌色，没有插画，没有渐变。彩色只在两个地方出现——语义位置（危险操作用 Signal Red）和用户主动选择的配色（11 套配色主题含默认 neutral，经 `data-color-theme` 叠加在灰阶基座上）。换言之，**颜色是一种可选刻度，不是系统的性格**；系统默认状态下完全不替用户做审美声明，这正是模板基座应有的姿态。

密度紧凑但呼吸充足：行高 32px、正文 14px、卡片内边距 16px、页面单列封顶 672px，信息不堆叠、不折叠、不藏进二级菜单。所有层级用 1px 描边和细分隔线表达，投影只留给真正浮起来的层。整套系统以桌面三平台为前提（窗口最小 1280×800），不假设触摸、不放大圆角、不加入移动端的宽松尺寸。

**Key Characteristics:**

- 中性石墨灰基座，彩色只出现在语义位置与用户选定的配色主题里
- 紧凑 32px 控件行高，14px 正文，1.5rem 页标题——只有三个真正用到的字号档位在承担全部层级
- 贴合式分层：1px ring / 边框表达静止结构，投影只属于悬浮层
- 圆角克制（10–14px 主力），只有徽标、开关、滑块是胶囊形
- 状态变化短促明确：100–200ms，只服务反馈，不做表演
- 用户可校准：字体族 / 字重 / 字号 / 明暗 / 配色 / 布局六项均为运行时参数，系统在两端点都必须与默认态同样成立

## Colors

一套以石墨灰为骨架的中性系统，色彩是可选配件而非身份。

### Primary

- **Graphite** (`oklch(0.205 0 0)`): 系统的"唯一原色"。承担实心主按钮（`bg-primary`）、开关打开态、滑块已填充段、徽标默认底；深色模式下它翻转为近白（`oklch(0.922 0 0)`），与纸面互换成对。它存在的意义是"这里可以按"，而不是"这里是品牌"。

### Neutral

- **Paper** (`oklch(1 0 0)`): 浅色模式的应用底色与卡片底；深色模式到消歧义由 **Charcoal** (`oklch(0.145 0 0)`) 接管。
- **Graphite Ink** (`oklch(0.145 0 0)`): 正文与主文本色，也是深色模式下 tooltip 的底色。
- **Graphite Soft** (`oklch(0.556 0 0)`): 次级文本、行说明、占位符、页脚版权；深色模式对应 **Graphite Ghost** (`oklch(0.708 0 0)`)。信息层级的第一工具是它，而不是缩小字号。
- **Steel** (`oklch(0.97 0 0)`): 静音面（`muted` / `secondary` / `accent`）与 hover 底；深色模式对应 **Steel Dark** (`oklch(0.269 0 0)`)。
- **Hairline** (`oklch(0.922 0 0)`): 边框与输入框描边；深色模式换成 10% 白（`oklch(1 0 0 / 10%)`），因为深色下纯灰线会脏。
- **Ring Steel** (`oklch(0.6 0 0)`): 键盘焦点环色，与 3px `ring/50` 光晕搭配；它必须始终可见，是自绘无边框窗口下的键盘可达性底线。
- **Control Boundary** (`oklch(0.6 0 0)`): 非文字边界色——开关关闭态轨道、滑块未填充轨道。刻意**不**随配色主题变化：同一个中间灰对白色卡片（3.95:1）与深色卡片（4.54:1）都过 3:1，而这两处背景都无法再承受更浅的边界。
- **Paper Bright** (`oklch(0.985 0 0)`): 实心主按钮上的文字色（浅色模式的 `primary-foreground`）。

### Semantic

- **Signal Red** (`oklch(0.577 0.245 27.325)`): 唯一的功能色。用于"恢复默认设置"按钮的 10% 染色底 + 红色文字、关闭行为确认弹窗、标题栏关闭按钮的 hover 态。永远以"染色底 + 彩色文字"或纯文字出现，不做大面积实心红；深色模式用更亮的 `oklch(0.704 0.191 22.216)` 保证对比度。

### 配色主题（Color Themes）

十一套配色（含默认 neutral）以 `html[data-color-theme]` 属性叠加在基座上，与明暗模式（`.dark` 类）正交组合：七套鲜艳主题（Ocean / Teal / Emerald / Amber / Tangerine / Rose / Amethyst）在浅色模式用 L 0.55–0.60 的高饱和主色实心填充、深色模式统一抬到 L 0.72–0.80 并降饱和；三套暗色美学（Nord / Dracula / Tokyo Night）的招牌值在深色块，浅色块是它们的"日间版本"。每套主题同时重定义 `primary` / `secondary` / `muted` / `accent` / `ring` / `sidebar-*`，保证整套表面同色相，而不是只换一个按钮色。

**The One Hue Rule.** 任一配色主题内只允许一个色相承载强调（primary 谱系）；其余表面只能是同色相淡彩或中性灰。永远不叠加第二个彩色作为"次强调"——那会让仪器变成海报。

**The Orthogonal Rule.** 明暗与配色是两个独立旋钮，11 × 2 = 22 种组合都必须成立：新增或修改配色时必须成对（浅色 + `:root.dark[data-color-theme]`）提供，且主色与主色上文字成对校验对比度，不允许任一组合下文字读不清。

**The Non-Text Contrast Rule.** 承载状态的"非文字边界"——焦点环、开关轨道、滑块轨道——对相邻底色必须 ≥3:1（WCAG 2.2 SC 1.4.11）：文字看不清还能猜，控件看不见就无从操作。这类边界统一走 `--control-boundary`，**不**交给配色主题决定；而主题化的 `ring` 必须逐个主题校验——它同样是 22 组合的一部分。

## Typography

**Display Font / Body Font:** Geist Variable（`@fontsource-variable/geist`），回退 `sans-serif`。系统只有一个字体家族，且用户可在设置里换成任意系统字体（此时栈为 `"<Selected>", "Geist Variable", sans-serif`）。

**Character:** 一个安静的工程无衬线体：字形中立、数字清晰（字重与字号常伴数值展示，用 `tabular-nums` 对齐），不参与任何表达欲。层级完全靠字号、字重和颜色三件工具完成。

### Hierarchy

- **Headline** (500, 1.5rem, 行高 1.33): 页面标题，每页唯一（`/settings`、`/about` 的 `h1`，`text-2xl font-medium`）。
- **Title** (500, 1rem, `leading-snug`): 卡片/分组标题（CardTitle），一行说清这个分组是什么。
- **Body** (400, 0.875rem, 行高 1.43): 应用正文的默认档位；行说明、卡片描述、下拉选项、按钮文字全部在 14px 这一档，行说明用 Graphite Soft 变灰而不是变小。
- **Label** (500, 0.875rem): 设置行的字段名、导航标签、表单标签；与 Body 同字号靠字重区分。
- **Caption** (400, 0.75rem): 页脚版权、徽标文字、tooltip；系统里唯一低于 14px 的档位，只用于"读一次就够"的辅助信息。

**The One Voice Rule.** 只有一个字体家族。层级永远用"字号 / 字重 / 颜色"表达，禁止引入第二字体、禁止用斜体或全大写制造层次。

**The Calibration Rule.** 字重（100–900，步进 100）与整体字号（75%–125%）是用户可调刻度（默认 400 / 100%）。`--app-font-weight` 是用户设定的**基准**：无类名文本继承它，`font-weight` 工具类则在其上做固定偏移（`font-medium` = 基准 +100、`font-semibold` = 基准 +200，依此类推；`font-light` 及以下同理向下偏移）。用户调的是"整体粗细"，**相对阶梯在任何档位都必须保留**，越界值由浏览器夹到 [1, 1000]，故拧到两端时阶梯自然收敛为同一档。任何界面（含两端极值）都必须不截断、不重叠、不溢出行列——"带刻度的精密件"必须经得起拧到两端。

## Layout

桌面固定壳层：窗口最小 1280×800，`h-screen` 全高分栏，无页面级横向滚动。三套布局（`data-layout`）共享同一内容模型：**上部标题栏（40px，含窗口控制）→ 导航 → 内容 → 底部版权条**。`tabs` 布局是导航横排 + 内容居中；`sidebar` 布局是左侧全高栏（208px）+ 右侧标题栏/内容/底边；`dashboard` 布局是标题栏下左侧导航（展开 208px / 收起 56px，默认展开，折叠偏好记入 localStorage）。

内容区遵循单列标定：`max-w-2xl`（672px）居中，`px-4 py-6`，分组间 `gap-6`（24px）；页面超高时由 ScrollArea 在视口内滚动。卡片内部：纵向 `gap-4`（16px）、左右内边距 16px、上下由内容自身承担；卡片行（设置项）`py-3`（12px）+ `divide-y` 发丝分隔线。右侧控件列固定宽 176px（`--control-column-width`）保持跨行对齐。导航标签横排高 32px、侧栏项高 32px。窄于 768px（`md` 断点）时，sidebar/dashboard 的导航折叠为图标形态。

**The Single Column Rule.** 设置 / 关于类内容永远是居中单列，封顶 672px；窗口拉宽时增加的是留白，不是列数。信息密度靠分组与分隔线管理，不靠加列。

## Elevation & Depth

贴合式平面（Flat-by-default）：界面是贴在台面上的一片片薄板，不是浮在空中的卡片。静止结构一律用 1px 描边表达——卡片 / 弹窗 / 下拉用 `ring-1 ring-foreground/10`，布局分区用 `border-b` / `border-t` / `border-r` 发丝线。投影是稀缺资源，只给真正脱离平面的层：下拉/弹出层、抽屉、以及"内容段选中"这一种抬起信号。深色模式下强烈的底色对比（Charcoal vs Paper）本身承担大部分分层，描边退为 10% 白。

### Shadow Vocabulary

- **贴合描边** (`box-shadow: 0 0 0 1px oklch(0 0 0 / 10%)`，项目内为 `ring-1 ring-foreground/10`): 卡片、对话框、下拉、popover 的静默轮廓——替代投影的主力。
- **段选抬起** (`box-shadow: 0 1px 2px 0 rgb(0 0 0 / 0.05)`，`shadow-sm`): 默认 tab 段选中态与 floating 侧栏；是系统里唯一允许出现在"非悬浮"元素上的投影。
- **悬浮菜单** (`box-shadow: 0 4px 6px -1px rgb(0 0 0 / 0.1), 0 2px 4px -2px rgb(0 0 0 / 0.1)`，`shadow-md`): 下拉与 popover。
- **抽屉** (`box-shadow: 0 10px 15px -3px rgb(0 0 0 / 0.1), 0 4px 6px -4px rgb(0 0 0 / 0.1)`，`shadow-lg`): Sheet 从边缘滑出时。
- **遮罩** (`background: rgb(0 0 0 / 10%)` + `backdrop-filter: blur(4px)`): 模态层的地面，把背景轻推一层而不是压黑。

**The Flat-By-Default Rule.** 静止面一律贴合：用描边分层，不用投影。投影只能由"离开平面"（悬浮层）或"被选中"（段选）触发；给普通卡片加投影属于改变系统性格，不做。

## Shapes

机加工边缘：圆角是一道 10px 的倒角，不是造型语言。基础半径 `--radius: 0.625rem`（10px），整套刻度由它推导：6 / 8 / 10 / 14 / 18 / 22 / 26px。实际使用收敛到三档——**10px**（按钮、输入框、下拉、菜单）、**14px**（卡片、对话框）、**胶囊**（徽标、开关、滑块轨道与滑块）。边框一律 1px 发丝线：浅色模式实色灰（Hairline），深色模式 10% 白。没有梯形、没有斜切、没有装饰性几何、没有插画。

**The Machined Edge Rule.** 圆角深浅由"这个元素被手指/鼠标怎么碰"决定：可点的小控件 10px，内容容器 14px，状态类的活动件（开关、滑块）才能用胶囊。禁止用超大圆角（>26px）把矩形控件包成糖果。

## Components

组件基调：**带刻度的精密件**——紧凑、稳健、状态明确，指哪打哪。所有组件来自 shadcn-svelte（nova / neutral）生成物与 `components/common` 手写封装；视觉参数一律引用语义令牌，不硬编码颜色。

### Buttons

- **Shape:** 圆角 10px，高 32px（`h-8`），横向内边距 10px，图标 16px，间距 6px。
- **Primary:** Graphite 实心 + Paper Bright 文字；仅 `[a]`（链接按钮）与 disabled 之外的 hover 有变化（`bg-primary/80`）。
- **Hover / Focus / Active:** `outline` / `secondary` / `ghost` hover 变 Steel 底或半透明层；`ghost` 是标题栏与工具位的默认选择。焦点统一 `focus-visible:border-ring` + `ring-3 ring-ring/50`。按下时 `active:translate-y-px` 下沉 1px——唯一的"物理触感"。
- **Destructive:** Signal Red 10% 染色底 + 红色文字，hover 加深到 20%；不出现实心红大按钮（唯一的全宽用法是设置页底部的"恢复默认设置"）。
- **Sizes:** `xs` 24px / `sm` 28px / default 32px / `lg` 36px；图标位 24 / 28 / 32 / 36px。标题栏窗口控制统一 `icon-sm`（28px ghost）。

### Chips / Badges

- **Style:** 胶囊形（高 20px），默认 Graphite 实心 + Paper Bright 文字；`secondary` 变体用 Steel 底 + 深字（版本状态"已是最新"用的就是它）。
- **State:** 徽标是只读状态标记，永远不可点击，也不承担主操作。
- **设置行徽章组：** 快捷键这类"组合值"用 `secondary` 徽章逐键展示，`Plus` 图标作连接符；整组单行不换行（`flex-nowrap`），超长横向滚动（滚动条隐藏），尾部主键始终可见；编辑走右侧 ghost 图标按钮，徽标本身不可点击（与"只读"一致）。

### Cards / Containers

- **Corner Style:** 14px（`rounded-xl`）。
- **Background:** 卡片底 = Paper（深色 = 抬升后的 Charcoal）；页面底 = Paper / Charcoal，两者靠 ring 而非色差区分。
- **Shadow Strategy:** 无投影，`ring-1 ring-foreground/10` 描边（见 Elevation）。
- **Border / Padding:** 内边距 16px（`sm` 尺寸 12px）；标题区与内容区同左对齐；卡片行之间 `divide-y` 发丝线。

### Inputs / Fields

- **Style:** 高 32px、圆角 10px、1px `--input` 描边、透明底（深色模式 `input/30` 半透明白底）；文字 14px。
- **Focus:** 边框转 Ring Steel + 3px `ring/50` 光晕；错误态（`aria-invalid`）转 Signal Red 描边 + 红色光晕。
- **选择器（Select / Slider / Switch）:** 触发器与输入框同规格（宽 176px 用于设置行）；开关 32×18.4px、打开态 Graphite 底、滑块 16px 纯白拇指 + Ring Steel 描边；滑块轨道 4px 高、已填充段 Graphite。三者是设置页的"旋钮"，状态必须一眼可读。

### Navigation

- **Style:** 三套布局共享同一导航注册表，视觉统一为：标签高 32px、14px 中型字、图标 16px。`tabs` 布局用 line 变体——未选中 `text-foreground/60`，选中转全色并在下方 2px 处亮出 `0.5` 下划线；`sidebar` 布局用列表项（radius 8px，选中 Steel 底 + 中字重）；`dashboard` 收起时只剩图标（56px 栏）并靠 tooltip 补标签。
- **状态:** 只有"当前页 / 非当前页"两个状态，不做悬停放大、不做动效位移。

### Title Bar（签名组件）

无边框窗口的自绘标题栏（40px，`data-tauri-drag-region` 拖拽区）：左侧应用图标（20px）+ 应用名（14px 中型），右侧四个 28px ghost 图标按钮——置顶 / 最小化 / 最大化（图标随窗口状态切换）/ 关闭（hover 时才染 Signal Red 10% 底 + 红字）。它是这个系统"仪器感"最集中的表达：窗口的物理行为直接长在界面上，不需要菜单栏。

### Dialogs / Tooltips / Toasts

- **Dialog / AlertDialog:** `max-w-sm`（384px），14px 圆角，Paper/Charcoal 抬升底 + ring 描边，遮罩 `black/10` + 轻模糊；标题 14px 中型、描述 14px Graphite Soft、底部右对齐双按钮（取消 = outline，确认 = primary 或 destructive）。
- **Tooltip:** Graphite Ink 底 + Paper 文字（深色模式反转），圆角 8px，12px 文字，8px 箭头菱形——它是"标签不够放"时的唯一后备，不做富内容。
- **Toast（svelte-sonner）:** 跟随 popover 令牌与当前明暗；图标 16px，加载态是旋转 loader。所有设置项提交都以此反馈结果，成功/失败文案必须双语齐备。

## Do's and Don'ts

### Do:

- **Do** 一切颜色 / 圆角 / 间距引用语义令牌（`bg-background`、`rounded-lg`、`gap-4`）；同一语义在 22 种明暗 × 配色组合下必须成立。
- **Do** 用 1px ring / 发丝分隔线表达分组与边界；卡片、弹窗、下拉一律贴合式描边。
- **Do** 保 32px 控件行高与 14px 正文默认档；层级优先用"颜色变灰 + 字重"，其次才考虑字号。
- **Do** 让每个交互状态可见：hover、`focus-visible`（Ring Steel 3px 光晕）、`active`（1px 下沉）、disabled（50% 透明）。键盘焦点环永不隐藏。
- **Do** 为双语（en / zh-CN）留出弹性：文案两种语言都不得截断、换行错位或撑破 176px 控件列。
- **Do** 危险操作（重置、退出、关闭）走 Signal Red 染色 + 确认弹窗；成功/失败用 Toast 收尾。
- **Do** 新配色主题成对提供浅色与 `:root.dark` 两块，并同时登记到外观取值元组、设置页下拉与两份文案。

### Don't:

- **Don't** 让界面变成营销落地页：不允许 hero 区、CTA 大按钮、卖点式排版进入应用界面（模板自己的 `/` 首页也只放技术栈图标 + 一句简介）。
- **Don't** 加炫技动画与特效：视差、粒子、大面积动效、背景视频一律禁止；动效只服务状态反馈（100–200ms）。
- **Don't** 引入装饰性插画与图形：无插画、无装饰形状、无渐变背景、无 emoji 装饰。
- **Don't** 用高饱和撞色：任一主题一次只挂一个色相，禁止把多个彩色主题的主色混搭在同一屏。
- **Don't** 用发光与玻璃拟物：无霓虹发光、无大面积渐变、无玻璃拟物、无重投影（尤其不要给静止卡片加投影）。
- **Don't** 堆成批量灌录的企业后台：禁止为了密度塞满表格、取消分组与留白、把说明文字压到 12px 以下。
- **Don't** 硬编码颜色值或手改 shadcn-svelte 生成组件来"调一下样式"；改视觉先改令牌（`layout.css` / `themes.css`），再回到组件。
