<script lang="ts">
  // CodeMirror6 挂载点：只在挂载时快照正文建视图，卸载销毁；
  // 切章由父级 key 块重建本组件，编辑历史天然隔离。
  // 命中装饰经 compartment 按需重配（不重建视图不断光标）；
  // 跳转由 jumpTo 驱动，行号越界直接忽略。
  import { onMount } from "svelte";
  import { untrack } from "svelte";
  import { RangeSet, StateEffect, StateField } from "@codemirror/state";
  import { Decoration } from "@codemirror/view";
  import { EditorView, basicSetup } from "codemirror";
  import type { LineMatch } from "#libs/commands/types";
  import { charSpanToRange } from "./chapter-text";

  let {
    content,
    matches = [],
    jumpTo = null,
    onEdit,
  }: {
    content: string;
    matches?: LineMatch[];
    jumpTo?: { line: number; nonce: number } | null;
    onEdit: (text: string) => void;
  } = $props();

  let host: HTMLDivElement | null = null;
  let view: EditorView | null = $state(null);

  /** 命中装饰槽：重配事件替换全集，文档编辑时自动映射偏移。 */
  const setMatchesEffect = StateEffect.define<RangeSet<Decoration>>();
  const matchField = StateField.define<RangeSet<Decoration>>({
    create: () => Decoration.none,
    update: (value, tr) => {
      for (const effect of tr.effects) {
        if (effect.is(setMatchesEffect)) {
          return effect.value;
        }
      }
      return value.map(tr.changes);
    },
    provide: (field) => EditorView.decorations.from(field),
  });

  // 编辑器配色引用语义令牌：明暗模式翻转时变量跟随，无需重建视图。
  const appTheme = EditorView.theme({
    "&": {
      height: "100%",
      backgroundColor: "var(--background)",
      color: "var(--foreground)",
      fontSize: "14px",
    },
    ".cm-content": { fontFamily: "inherit", lineHeight: "1.7" },
    ".cm-gutters": {
      backgroundColor: "var(--background)",
      color: "var(--muted-foreground)",
      borderRight: "1px solid var(--border)",
    },
    // 激活行不用整行底色（会吃掉选中底，CodeMirror 默认也没有）；
    // 行信号移到行号栏，选中底在透明纸底上处处一致。
    ".cm-activeLine": { backgroundColor: "transparent" },
    ".cm-activeLineGutter": { backgroundColor: "var(--muted)" },
    "&.cm-focused": { outline: "none" },
    // 命中行不用整行底色（同激活行，会吃掉选中底），改用行首主色条指示；
    // 命中词仍加粗下划线，选中底在透明纸底上处处一致。
    ".cm-match-line": { boxShadow: "inset 3px 0 0 var(--primary)" },
    ".cm-match-text": {
      fontWeight: "700",
      textDecoration: "underline",
      textUnderlineOffset: "2px",
    },
    // 选中底必须压过行底（含激活行）：默认规则特异度高（聚焦态 6 个类），
    // 此处把目标类名双写以超出之；`theme()` 不支持 `&light/&dark`，故不用。
    "& > .cm-scroller > .cm-selectionLayer .cm-selectionBackground.cm-selectionBackground": {
      backgroundColor: "color-mix(in oklch, var(--primary) 18%, transparent)",
    },
    "&.cm-focused > .cm-scroller > .cm-selectionLayer .cm-selectionBackground.cm-selectionBackground":
      {
        backgroundColor: "color-mix(in oklch, var(--primary) 32%, transparent)",
      },
  });

  /** 按当前文档与命中清单构建装饰：行淡底 + 词强调，越界行跳过。 */
  function buildMatchDecorations(docText: string, list: LineMatch[]): RangeSet<Decoration> {
    const lines = docText.split("\n");
    const spans: { from: number; to: number; value: Decoration }[] = [];
    for (const match of list) {
      const lineText = lines[match.line - 1];
      if (lineText === undefined) {
        continue;
      }
      const lineStart = lines.slice(0, match.line - 1).join("\n").length + (match.line > 1 ? 1 : 0);
      spans.push({
        from: lineStart,
        to: lineStart,
        value: Decoration.line({ class: "cm-match-line" }),
      });
      for (const span of match.spans) {
        const { from, to } = charSpanToRange(lineText, span.start, span.len);
        if (to > from) {
          spans.push({
            from: lineStart + from,
            to: lineStart + to,
            value: Decoration.mark({ class: "cm-match-text" }),
          });
        }
      }
    }
    return RangeSet.of(
      spans.sort((a, b) => a.from - b.from || a.to - b.to),
      true,
    );
  }

  onMount(() => {
    if (host == null) {
      return;
    }
    const current = new EditorView({
      doc: content,
      parent: host,
      extensions: [
        basicSetup,
        EditorView.lineWrapping,
        appTheme,
        matchField,
        EditorView.updateListener.of((update) => {
          if (update.docChanged) {
            onEdit(update.state.doc.toString());
          }
        }),
      ],
    });
    view = current;
    view.focus();
    return () => {
      view = null;
      current.destroy();
    };
  });

  // 只订阅命中与跳转：编辑回写不触发重配；装饰按实时文档构建，跳转行号钳制。
  $effect(() => {
    const list = matches;
    const target = jumpTo;
    const current = view;
    if (current == null) {
      return;
    }
    untrack(() => {
      current.dispatch({
        effects: setMatchesEffect.of(buildMatchDecorations(current.state.doc.toString(), list)),
      });
      if (target != null) {
        const lineNo = Math.max(1, Math.min(target.line, current.state.doc.lines));
        const pos = current.state.doc.line(lineNo).from;
        current.dispatch({ selection: { anchor: pos }, scrollIntoView: true });
        current.focus();
      }
    });
  });
</script>

<div class="h-full min-h-0 overflow-hidden" bind:this={host}></div>
