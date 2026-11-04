<script lang="ts">
  // 构建文件查看器（哑组件）：纯挂载，按扩展名切 xml/css/纯文本高亮；
  // 不可编辑时禁用输入，切文件由父级 key 块重建，不保留草稿；
  // 外部改动（格式化回填）经 content 同步进视图，视图内编辑经 onEdit 回写。
  import { onMount, untrack } from "svelte";
  import { EditorView, basicSetup } from "codemirror";
  import { css as cssLang } from "@codemirror/lang-css";
  import { xml as xmlLang } from "@codemirror/lang-xml";

  let {
    content,
    editable,
    language,
    onEdit,
  }: {
    content: string;
    editable: boolean;
    language: "xml" | "css" | "text";
    onEdit: (text: string) => void;
  } = $props();

  let host: HTMLDivElement | null = null;
  let view: EditorView | null = $state(null);

  // 编辑器配色引用语义令牌：明暗模式翻转时变量跟随，无需重建视图；
  // 与编辑页挂载点同口径（含选中底覆盖），选中渲染在所有行一致。
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
    ".cm-activeLine": { backgroundColor: "transparent" },
    ".cm-activeLineGutter": { backgroundColor: "var(--muted)" },
    "&.cm-focused": { outline: "none" },
    "& > .cm-scroller > .cm-selectionLayer .cm-selectionBackground.cm-selectionBackground": {
      backgroundColor: "color-mix(in oklch, var(--primary) 18%, transparent)",
    },
    "&.cm-focused > .cm-scroller > .cm-selectionLayer .cm-selectionBackground.cm-selectionBackground":
      {
        backgroundColor: "color-mix(in oklch, var(--primary) 32%, transparent)",
      },
  });

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
        EditorView.editable.of(editable),
        ...(language === "xml" ? [xmlLang()] : language === "css" ? [cssLang()] : []),
        EditorView.updateListener.of((update) => {
          if (update.docChanged) {
            onEdit(update.state.doc.toString());
          }
        }),
      ],
    });
    view = current;
    return () => {
      view = null;
      current.destroy();
    };
  });

  // 外部 content 变更（格式化回填）同步进视图；视图内编辑回写后两者一致，不重复派发。
  $effect(() => {
    const text = content;
    const current = view;
    if (current == null) {
      return;
    }
    untrack(() => {
      if (current.state.doc.toString() !== text) {
        current.dispatch({
          changes: { from: 0, to: current.state.doc.length, insert: text },
        });
      }
    });
  });
</script>

<div class="h-full min-h-0 overflow-hidden" bind:this={host}></div>
