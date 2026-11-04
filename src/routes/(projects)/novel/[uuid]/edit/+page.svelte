<script lang="ts">
  // 小说编辑页：左栏章节树 + 中栏 CodeMirror6 正文编辑 + 右栏正文筛选；
  // 脏态下切章先确认，保存落盘后刷新大纲；无解析产物时引导回文件步骤。
  // 筛选扫描落盘文件：脏态禁用筛选，编辑后旧结果保留并挂过期提示，保存成功自动重跑。
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { resolve } from "$app/paths";
  import { page } from "$app/state";
  import FileTextIcon from "@lucide/svelte/icons/file-text";
  import { Button } from "#components/shadcn-svelte/button";
  import * as Resizable from "#components/shadcn-svelte/resizable";
  import ConfirmDialog from "#components/common/confirm-dialog.svelte";
  import commands from "#libs/commands";
  import type { FilterOutcome, NovelFilter, ParseOutline } from "#libs/commands/types";
  import { m } from "#libs/i18n/paraglide/messages";
  import { toast } from "#libs/utils/toast";
  import ChapterEditor from "./_components/chapter-editor.svelte";
  import { joinChapterLines, splitChapterLines } from "./_components/chapter-text";
  import ChapterTree from "./_components/chapter-tree.svelte";
  import FilterPanel from "./_components/filter-panel.svelte";

  const uuid = $derived(page.params.uuid ?? "");

  let outline = $state<ParseOutline | null>(null);
  let loading = $state(true);
  let loadFailed = $state(false);

  /** 当前选中：卷章下标，与大纲数组同源。 */
  let selected = $state<{ volume: number; chapter: number } | null>(null);
  /** 选中章标题（只读展示）、落盘快照与编辑器实时文本。 */
  let chapterTitle = $state("");
  let savedContent = $state("");
  let draft = $state("");
  let chapterLoading = $state(false);
  let chapterFailed = $state(false);
  let saving = $state(false);

  /** 脏态：编辑器文本与落盘快照不一致时保存可用。 */
  const dirty = $derived(draft !== savedContent);
  /** 章节唯一键：驱动编辑器切章重建。 */
  const docKey = $derived(selected == null ? "" : `${selected.volume}:${selected.chapter}`);
  /** 切章确认框：脏态下点选他章先弹框，默认停留。 */
  let confirmOpen = $state(false);
  let pendingSelect = $state<{ volume: number; chapter: number } | null>(null);

  /** 筛选面板参数：右栏哑组件双向绑定，本页只读组装。 */
  let filterKind = $state<"keyword" | "regex">("keyword");
  let filterKeyword = $state("");
  let filterCase = $state(false);
  let filterPattern = $state("");
  /** 筛选中、最近成功规格与结果：结果绑定规格快照，面板改动即过期。 */
  let filtering = $state(false);
  let appliedSpec = $state<NovelFilter | null>(null);
  let outcome = $state<FilterOutcome | null>(null);
  /** 跳转目标：nonce 保证连点同一行每次都生效。 */
  let jumpTo = $state<{ line: number; nonce: number } | null>(null);

  /** 当前面板组装的筛选规格：输入为空回落空，不调后端。 */
  function currentSpec(): NovelFilter | null {
    if (filterKind === "keyword") {
      if (filterKeyword.trim() === "") {
        return null;
      }
      return { type: "Keyword", keyword: filterKeyword, case_sensitive: filterCase };
    }
    if (filterPattern.trim() === "") {
      return null;
    }
    return { type: "Regex", pattern: filterPattern };
  }

  /** 筛选是否可用：脏态禁用，结果永远与落盘一致。 */
  const canFilter = $derived(
    !dirty && !filtering && !saving && outline != null && currentSpec() != null,
  );
  /** 结果是否过期：正文改动或面板参数变化后挂过期提示。 */
  const stale = $derived(
    outcome != null &&
      appliedSpec != null &&
      (dirty || JSON.stringify(currentSpec()) !== JSON.stringify(appliedSpec)),
  );
  /** 左栏可见集：筛选激活时仅渲染命中章节，`null` 表全量。 */
  const visibleSet = $derived<Set<string> | null>(
    outcome == null
      ? null
      : new Set(outcome.chapters.map((c) => `${c.volume_index}:${c.chapter_index}`)),
  );
  /** 选中章节的命中行：无命中回落空数组，面板隐藏。 */
  const selectedMatches = $derived(
    outcome?.chapters.find(
      (c) =>
        selected != null &&
        c.volume_index === selected.volume &&
        c.chapter_index === selected.chapter,
    )?.matches ?? [],
  );

  /** 首个可编辑章节：按大纲顺序找首个有正文章节的下标。 */
  function firstChapter(target: ParseOutline): { volume: number; chapter: number } | null {
    for (const [volume, entry] of target.volumes.entries()) {
      if (entry.chapters.length > 0) {
        return { volume, chapter: 0 };
      }
    }
    return null;
  }

  /** 进页拉取大纲：缺失回落空态引导去文件步骤，正常默认选中首章。 */
  async function loadOutline(): Promise<void> {
    loading = true;
    loadFailed = false;
    outline = null;
    selected = null;
    const result = await commands.getNovelContentOutline(uuid).result();
    loading = false;
    if (result.status === "error") {
      loadFailed = true;
      toast.error(m.edit_load_failed());
      return;
    }
    outline = result.data;
    const first = outline == null ? null : firstChapter(outline);
    if (first == null) {
      return;
    }
    await openChapter(first.volume, first.chapter);
  }

  /** 打开章节：取正文快照并同步编辑器，失败走重试分支。 */
  async function openChapter(volume: number, chapter: number): Promise<void> {
    selected = { volume, chapter };
    jumpTo = null;
    chapterLoading = true;
    chapterFailed = false;
    const result = await commands.getNovelChapter(uuid, volume, chapter).result();
    chapterLoading = false;
    if (result.status === "error") {
      chapterFailed = true;
      toast.error(m.edit_chapter_load_failed());
      return;
    }
    chapterTitle = result.data.full_title;
    savedContent = joinChapterLines(result.data.lines);
    draft = savedContent;
  }

  /** 点选章节：同章直接返回，脏态先弹确认框，否则直接打开。 */
  function requestSelect(volume: number, chapter: number): void {
    if (selected?.volume === volume && selected?.chapter === chapter) {
      return;
    }
    if (dirty) {
      pendingSelect = { volume, chapter };
      confirmOpen = true;
      return;
    }
    void openChapter(volume, chapter);
  }

  /** 保存：切分归一化后落盘，成功刷新大纲字数并清脏态；筛选激活时自动重跑。 */
  async function handleSave(): Promise<void> {
    if (selected == null || !dirty || saving) {
      return;
    }
    saving = true;
    const result = await commands
      .updateNovelChapter(uuid, selected.volume, selected.chapter, splitChapterLines(draft))
      .result();
    saving = false;
    if (result.status === "error") {
      toast.error(m.edit_save_failed());
      return;
    }
    outline = result.data;
    savedContent = draft;
    toast.success(m.edit_save_success());
    if (appliedSpec != null) {
      await runFilter();
    }
  }

  /** 章节加载失败重试：选中仍在时重开当前章。 */
  function retryChapter(): void {
    if (selected == null) {
      return;
    }
    void openChapter(selected.volume, selected.chapter);
  }

  /** 执行筛选：落盘扫描，成功后选中若不在命中内则跳首个命中章节。 */
  async function runFilter(): Promise<void> {
    const spec = currentSpec();
    if (spec == null || dirty || filtering || outline == null) {
      return;
    }
    filtering = true;
    const result = await commands.filterNovelContent(uuid, spec).result();
    filtering = false;
    if (result.status === "error") {
      toast.error(m.edit_filter_failed());
      return;
    }
    outcome = result.data;
    appliedSpec = spec;
    jumpTo = null;
    if (outcome.truncated) {
      toast.warning(m.edit_filter_truncated());
    }
    const first = outcome.chapters[0];
    if (
      first != null &&
      (selected == null ||
        selected.volume !== first.volume_index ||
        selected.chapter !== first.chapter_index) &&
      (selected == null ||
        visibleSet == null ||
        !visibleSet.has(`${selected.volume}:${selected.chapter}`))
    ) {
      await openChapter(first.volume_index, first.chapter_index);
    }
  }

  /** 清除筛选：结果与规格全空，左栏恢复全量。 */
  function clearFilter(): void {
    outcome = null;
    appliedSpec = null;
    jumpTo = null;
  }

  onMount(() => {
    void loadOutline();
  });
</script>

<!-- 小说编辑页：左栏章节树 + 中栏正文编辑 + 右栏正文筛选 -->
<h1 class="sr-only">{m.project_step_edit()}</h1>
<div class="flex h-full min-h-0 w-full min-w-0">
  <Resizable.PaneGroup direction="horizontal">
    <Resizable.Pane defaultSize={22} minSize={15} maxSize={35} class="min-w-0">
      <!-- 滚动容器自身无内边距：卷头吸顶才能通栏贴顶，缩进由树内部补回 -->
      <div class="h-full min-h-0 overflow-y-auto">
        {#if loading}
          <p class="px-3 pt-3 text-sm text-muted-foreground" aria-busy="true">{m.edit_loading()}</p>
        {:else if loadFailed}
          <div class="flex flex-col items-center justify-center gap-3 px-2 py-8 text-center">
            <p class="text-sm text-muted-foreground">{m.edit_load_failed()}</p>
            <Button variant="outline" size="sm" onclick={() => void loadOutline()}>
              {m.files_retry()}
            </Button>
          </div>
        {:else if outline !== null && visibleSet !== null && visibleSet.size === 0}
          <p class="px-3 pt-3 text-sm text-muted-foreground">{m.edit_filter_no_match()}</p>
        {:else if outline !== null}
          <ChapterTree {outline} {selected} visible={visibleSet} onSelect={requestSelect} />
        {/if}
      </div>
    </Resizable.Pane>
    <Resizable.Handle withHandle aria-label={m.edit_resize_left_label()} />
    <Resizable.Pane defaultSize={56} minSize={30} class="min-w-0">
      <div class="h-full min-h-0">
        {#if loading || chapterLoading}
          <div class="flex h-full min-h-0 items-center justify-center">
            <p class="text-sm text-muted-foreground" aria-busy="true">
              {chapterLoading ? m.edit_chapter_loading() : m.edit_loading()}
            </p>
          </div>
        {:else if loadFailed}
          <div
            class="flex h-full min-h-0 flex-col items-center justify-center gap-3 p-6 text-center"
          >
            <FileTextIcon class="size-10 text-muted-foreground" />
            <p class="text-sm text-muted-foreground">{m.edit_load_failed()}</p>
            <Button variant="outline" size="sm" onclick={() => void loadOutline()}>
              {m.files_retry()}
            </Button>
          </div>
        {:else if outline === null || firstChapter(outline) === null}
          <div
            class="flex h-full min-h-0 flex-col items-center justify-center gap-2 p-6 text-center"
          >
            <FileTextIcon class="size-10 text-muted-foreground" />
            <p class="text-sm font-medium">{m.edit_empty_title()}</p>
            <p class="text-sm text-muted-foreground">{m.edit_empty_description()}</p>
            <Button
              variant="outline"
              size="sm"
              onclick={() => void goto(resolve(`/novel/${uuid}/files`))}
            >
              {m.edit_goto_files()}
            </Button>
          </div>
        {:else if chapterFailed || selected === null}
          <div
            class="flex h-full min-h-0 flex-col items-center justify-center gap-3 p-6 text-center"
          >
            <FileTextIcon class="size-10 text-muted-foreground" />
            <p class="text-sm text-muted-foreground">{m.edit_chapter_load_failed()}</p>
            {#if selected !== null}
              <Button variant="outline" size="sm" onclick={retryChapter}>
                {m.files_retry()}
              </Button>
            {/if}
          </div>
        {:else}
          <ChapterEditor
            {docKey}
            title={chapterTitle}
            content={savedContent}
            {dirty}
            {saving}
            matches={selectedMatches}
            {stale}
            {jumpTo}
            onEdit={(text) => (draft = text)}
            onSave={() => void handleSave()}
            onJump={(line) => (jumpTo = { line, nonce: Date.now() })}
          />
        {/if}
      </div>
    </Resizable.Pane>
    <Resizable.Handle withHandle aria-label={m.edit_resize_right_label()} />
    <Resizable.Pane defaultSize={22} minSize={15} maxSize={35} class="min-w-0">
      <div class="h-full min-h-0 overflow-hidden">
        <FilterPanel
          bind:kind={filterKind}
          bind:keyword={filterKeyword}
          bind:caseSensitive={filterCase}
          bind:pattern={filterPattern}
          {filtering}
          {canFilter}
          hasOutcome={outcome != null}
          onRun={() => void runFilter()}
          onClear={clearFilter}
        />
      </div>
    </Resizable.Pane>
  </Resizable.PaneGroup>
</div>
<ConfirmDialog
  bind:open={confirmOpen}
  title={m.edit_unsaved_title()}
  description={m.edit_unsaved_description()}
  cancelLabel={m.projects_create_cancel()}
  confirmLabel={m.edit_unsaved_discard()}
  onConfirm={() => {
    if (pendingSelect != null) {
      void openChapter(pendingSelect.volume, pendingSelect.chapter);
    }
    pendingSelect = null;
  }}
/>
