<script lang="ts">
  // 小说导出页：进页探针查全产物，表单勾选文件与命名，目录框选定后改名落盘。
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import { resolve } from "$app/paths";
  import { open } from "@tauri-apps/plugin-dialog";
  import DownloadIcon from "@lucide/svelte/icons/download";
  import { Button } from "#components/shadcn-svelte/button";
  import { Checkbox } from "#components/shadcn-svelte/checkbox";
  import { Input } from "#components/shadcn-svelte/input";
  import * as RadioGroup from "#components/shadcn-svelte/radio-group";
  import {
    Select,
    SelectContent,
    SelectItem,
    SelectTrigger,
    SelectValue,
  } from "#components/shadcn-svelte/select";
  import {
    DialogContent,
    DialogFooter,
    DialogHeader,
    DialogTitle,
    Dialog,
  } from "#components/shadcn-svelte/dialog";
  import { Card, CardContent } from "#components/shadcn-svelte/card";
  import CardSection from "#components/common/card-section.svelte";
  import commands from "#libs/commands";
  import type {
    ExportNaming,
    ExportOutcome,
    ExportStatus,
    ChapterNumbering,
    MetadataField,
    TxtTitleKind,
  } from "#libs/commands/types";
  import { m } from "#libs/i18n/paraglide/messages";
  import { toast } from "#libs/utils/toast";
  import {
    METADATA_FIELDS,
    TXT_TITLE_KINDS,
    canExport,
    isValidCustomName,
    isValidTxtOptions,
    namingKinds,
    namingValue,
  } from "./_components/export-form";

  const uuid = $derived(page.params.uuid ?? "");

  let status = $state<ExportStatus | null>(null);
  let loading = $state(true);
  let loadFailed = $state(false);
  let checkEpub = $state(false);
  let checkTxt = $state(false);
  let checkCover = $state(false);
  let naming = $state<ExportNaming>("project");
  let customName = $state("");
  let exporting = $state(false);
  let outcome = $state<ExportOutcome | null>(null);
  /** 打开导出目录进行中：按钮禁用态。 */
  let openingDir = $state(false);
  /** TXT 卷章标题格式与自定义模板。 */
  let chapterKind = $state<TxtTitleKind>("original");
  let chapterPattern = $state("");
  let volumeKind = $state<TxtTitleKind>("original");
  let volumePattern = $state("");
  /** 章编号方式：分卷重置为现状口径，跨卷连续按文档顺序全局累加。 */
  let numbering = $state<ChapterNumbering>("per_volume");
  /** 格式示例：首章/首卷各渲染一行；自定义按输入模板实时渲染（防抖），空白模板不展示，失败静默隐藏。 */
  let chapterPreview = $state<string | null>(null);
  let volumePreview = $state<string | null>(null);
  let chapterNonce = $state(0);
  let volumeNonce = $state(0);
  /** 自定义模板输入防抖定时器：避免每敲一键一次 IPC。 */
  let chapterDebounce: ReturnType<typeof setTimeout> | undefined;
  let volumeDebounce: ReturnType<typeof setTimeout> | undefined;
  /** 元数据：行首复选 + 弹窗多选（草稿经确定提交）。 */
  let metaEnabled = $state(false);
  let metaFields = $state<MetadataField[]>([]);
  let metaDialogOpen = $state(false);
  let metaDraft = $state<MetadataField[]>([]);

  /** 是否空态：三类产物皆无时引导回文件步骤。 */
  const isEmpty = $derived(
    status !== null && !status.has_content && !status.has_epub && !status.has_cover,
  );
  /** 命名候选：项目名与书名相同时合并。 */
  const kinds = $derived(status == null ? [] : namingKinds(status.project_name, status.book_title));
  /** 下拉候选项：触发器关闭时靠 `items` 解析展示文案。 */
  const namingItems = $derived.by(() => {
    if (status == null) {
      return [];
    }
    const { project_name, book_title } = status;
    return kinds.map((kind) => ({
      value: kind,
      label:
        kind === "custom"
          ? m.export_naming_custom()
          : `${kind === "book" ? m.export_naming_book() : m.export_naming_project()} · ${namingValue(kind, project_name, book_title)}`,
    }));
  });
  /** 分卷书：多卷时展示卷标题格式与编号方式，单卷两种编号等价故隐藏。 */
  const isVolumeBook = $derived(status !== null && (status.content?.volume_count ?? 0) > 1);
  /** 导出可用：有勾选、命名合法、TXT 参数合法、空闲中。 */
  const ready = $derived(
    status !== null &&
      canExport({ epub: checkEpub, txt: checkTxt, cover: checkCover }, status) &&
      (naming !== "custom" || isValidCustomName(customName)) &&
      (!checkTxt ||
        isValidTxtOptions(
          chapterKind,
          chapterPattern,
          volumeKind,
          volumePattern,
          metaEnabled,
          metaFields,
        )) &&
      !exporting &&
      !loading,
  );

  /** 标题格式下拉候选项：触发器关闭时靠 `items` 解析展示文案。 */
  const titleKindItems = TXT_TITLE_KINDS.map((kind) => ({
    value: kind,
    label: titleKindLabel(kind),
  }));

  /** 标题格式下拉文案。 */
  function titleKindLabel(kind: TxtTitleKind): string {
    switch (kind) {
      case "original":
        return m.export_txt_title_original();
      case "arabic":
        return m.export_txt_title_arabic();
      case "padded":
        return m.export_txt_title_padded();
      case "chinese_lower":
        return m.export_txt_title_chinese_lower();
      case "chinese_upper":
        return m.export_txt_title_chinese_upper();
      case "custom":
        return m.export_txt_title_custom();
    }
  }

  /** 元数据字段文案：复用元数据表单标签。 */
  function metaFieldLabel(field: MetadataField): string {
    switch (field) {
      case "title":
        return m.metadata_title_label();
      case "alternative":
        return m.metadata_alternative_label();
      case "creator":
        return m.metadata_creator_label();
      case "contributor":
        return m.metadata_contributor_label();
      case "language":
        return m.metadata_language_label();
      case "publisher":
        return m.metadata_publisher_label();
      case "series":
        return m.metadata_series_label();
      case "volume":
        return m.metadata_volume_label();
      case "date_published":
        return m.metadata_date_published_label();
      case "rights":
        return m.metadata_rights_label();
      case "source":
        return m.metadata_source_label();
      case "subjects":
        return m.metadata_subjects_label();
      case "description":
        return m.metadata_description_label();
    }
  }

  /** 进页探针：所有导出项默认不选中，由用户按需勾选；无产物展示空态引导。 */
  async function loadStatus(): Promise<void> {
    loading = true;
    loadFailed = false;
    const result = await commands.getNovelExportStatus(uuid).result();
    loading = false;
    if (result.status === "error") {
      loadFailed = true;
      toast.error(m.export_status_failed());
      return;
    }
    status = result.data;
    checkEpub = false;
    checkTxt = false;
    checkCover = false;
    naming = "project";
    customName = "";
    chapterKind = "original";
    chapterPattern = "";
    volumeKind = "original";
    volumePattern = "";
    numbering = "per_volume";
    metaEnabled = false;
    metaFields = [];
    outcome = null;
  }

  /** 下拉给字符串，经候选项收窄后才提交，无类型断言。 */
  function handleNamingChange(value: string): void {
    if (value === "project" || value === "book" || value === "custom") {
      naming = value;
    }
  }

  /** 标题格式下拉：同上收窄，切 custom 空模板时预填默认，切换后刷新对应示例。 */
  function handleTitleKindChange(value: string, target: "chapter" | "volume"): void {
    if ((TXT_TITLE_KINDS as string[]).includes(value)) {
      if (target === "chapter") {
        chapterKind = value as TxtTitleKind;
        if (chapterKind === "custom" && chapterPattern.trim() === "") {
          chapterPattern = "第#N章 #T";
        }
      } else {
        volumeKind = value as TxtTitleKind;
        if (volumeKind === "custom" && volumePattern.trim() === "") {
          volumePattern = "第#N卷 #T";
        }
      }
      void refreshTitlePreview(target);
    }
  }

  /** 编号方式切换：分卷重置为现状口径，跨卷连续按文档顺序全局累加；刷新章预览。 */
  function handleNumberingChange(value: string): void {
    if (value === "continuous" || value === "per_volume") {
      numbering = value;
      void refreshTitlePreview("chapter");
    }
  }

  /** 自定义模板输入：更新后防抖 400ms 刷新预览。 */
  function handlePatternInput(target: "chapter" | "volume", value: string): void {
    if (target === "chapter") {
      chapterPattern = value;
      clearTimeout(chapterDebounce);
      chapterDebounce = setTimeout(() => void refreshTitlePreview("chapter"), 400);
    } else {
      volumePattern = value;
      clearTimeout(volumeDebounce);
      volumeDebounce = setTimeout(() => void refreshTitlePreview("volume"), 400);
    }
  }

  /** 格式示例：内置格式与原文取首章/首卷渲染，自定义按输入模板实时渲染；空模板不展示，失败静默隐藏；序号防迟到覆盖。 */
  async function refreshTitlePreview(target: "chapter" | "volume"): Promise<void> {
    const kind = target === "chapter" ? chapterKind : volumeKind;
    const pattern = target === "chapter" ? chapterPattern : volumePattern;
    if (target === "chapter") {
      chapterPreview = null;
    } else {
      volumePreview = null;
    }
    if (!checkTxt) {
      return;
    }
    if (kind === "custom" && pattern.trim() === "") {
      return;
    }
    if (target === "chapter") {
      const nonce = (chapterNonce += 1);
      const result = await commands
        .previewNovelTxtTitle(uuid, kind, kind === "custom" ? pattern : "", false, numbering)
        .result();
      if (nonce !== chapterNonce || result.status === "error") {
        return;
      }
      chapterPreview = result.data;
    } else {
      const nonce = (volumeNonce += 1);
      const result = await commands
        .previewNovelTxtTitle(uuid, kind, kind === "custom" ? pattern : "", true, numbering)
        .result();
      if (nonce !== volumeNonce || result.status === "error") {
        return;
      }
      volumePreview = result.data;
    }
  }

  /** 打开源数据弹窗：有选择则以当前选择为草稿，否则默认全选，取消不回写。 */
  function openMetaDialog(): void {
    metaDraft = metaFields.length > 0 ? [...metaFields] : [...METADATA_FIELDS];
    metaDialogOpen = true;
  }

  /** 弹窗内单项切换。 */
  function toggleMetaDraft(field: MetadataField, checked: boolean): void {
    metaDraft = checked ? [...metaDraft, field] : metaDraft.filter((item) => item !== field);
  }

  /** 导出：目录框选定后直接落盘，取消静默返回。 */
  async function runExport(): Promise<void> {
    if (!ready || status == null) {
      return;
    }
    let picked: string | string[] | null;
    try {
      picked = await open({ directory: true, multiple: false });
    } catch {
      toast.error(m.export_failed());
      return;
    }
    if (picked == null) {
      return;
    }
    const dest = Array.isArray(picked) ? picked[0] : picked;
    if (dest == null) {
      return;
    }
    exporting = true;
    const result = await commands
      .exportNovelContent(
        uuid,
        dest,
        naming,
        customName,
        {
          epub: checkEpub,
          txt: checkTxt,
          cover: checkCover,
        },
        {
          chapter_kind: chapterKind,
          chapter_pattern: chapterPattern,
          volume_kind: volumeKind,
          volume_pattern: volumePattern,
          chapter_numbering: numbering,
          metadata: metaEnabled ? metaFields : [],
        },
      )
      .result();
    exporting = false;
    if (result.status === "error") {
      toast.error(m.export_failed());
      return;
    }
    outcome = result.data;
    toast.success(m.export_success());
  }

  /** 打开导出目录：调系统文件管理器定位落盘目录，失败 toast。 */
  async function openExportDir(): Promise<void> {
    if (outcome == null || openingDir) {
      return;
    }
    openingDir = true;
    const result = await commands.openExportDir(outcome.dir).result();
    openingDir = false;
    if (result.status === "error") {
      toast.error(m.export_open_dir_failed());
    }
  }

  onMount(() => {
    void loadStatus();
  });
</script>

<!-- 小说导出页：文件勾选 + 命名 + 目录导出 -->
<h1 class="sr-only">{m.project_step_export()}</h1>
<div class="mx-auto flex w-full max-w-3xl flex-1 flex-col gap-4 overflow-y-auto p-6">
  {#if loading}
    <div class="flex flex-1 items-center justify-center">
      <p class="text-sm text-muted-foreground" aria-busy="true">{m.export_loading()}</p>
    </div>
  {:else if loadFailed}
    <div class="flex flex-1 flex-col items-center justify-center gap-3 text-center">
      <p class="text-sm text-muted-foreground">{m.export_status_failed()}</p>
      <Button variant="outline" class="w-full max-w-md" onclick={() => void loadStatus()}>
        {m.files_retry()}
      </Button>
    </div>
  {:else if status !== null && isEmpty}
    <div class="flex flex-1 flex-col items-center justify-center gap-2 text-center">
      <DownloadIcon class="size-10 text-muted-foreground" />
      <p class="text-sm font-medium">{m.export_empty_title()}</p>
      <p class="text-sm text-muted-foreground">{m.export_empty_description()}</p>
      <Button class="w-full max-w-md" onclick={() => void goto(resolve(`/novel/${uuid}/files`))}>
        {m.export_goto_files()}
      </Button>
    </div>
  {:else if status !== null}
    <CardSection title={m.export_files_title()}>
      <label class="flex cursor-pointer items-center gap-2 py-3 text-sm">
        <Checkbox
          checked={checkEpub}
          disabled={!status.has_epub || exporting}
          onCheckedChange={(value) => (checkEpub = value === true)}
        />
        <span class="min-w-0 flex-1">
          <span class="font-medium">{m.export_epub_label()}</span>
          {#if status.has_epub && status.epub_name !== null}
            <span class="block truncate font-mono text-xs text-muted-foreground">
              {status.epub_name}
            </span>
          {:else}
            <span class="block text-xs text-muted-foreground">{m.export_missing_epub()}</span>
          {/if}
        </span>
      </label>
      <!-- TXT 一组：复选 + 标题格式 + 元数据，组内无分隔线 -->
      <div class="flex flex-col">
        <label class="flex cursor-pointer items-center gap-2 py-2 text-sm">
          <Checkbox
            checked={checkTxt}
            disabled={!status.has_content || exporting}
            onCheckedChange={(value) => {
              checkTxt = value === true;
              if (checkTxt) {
                void refreshTitlePreview("volume");
                void refreshTitlePreview("chapter");
              }
            }}
          />
          <span class="min-w-0 flex-1">
            <span class="font-medium">{m.export_txt_label()}</span>
            {#if status.has_content && status.content !== null}
              <span class="block text-xs text-muted-foreground">
                {m.export_summary_content({
                  volumes: status.content.volume_count,
                  chapters: status.content.chapter_count,
                  chars: status.content.char_count,
                })}
              </span>
            {:else}
              <span class="block text-xs text-muted-foreground">{m.export_missing_txt()}</span>
            {/if}
          </span>
        </label>
        {#if checkTxt}
          {#if isVolumeBook}
            <div class="flex items-center gap-2 py-2">
              <span class="shrink-0 text-sm font-medium">{m.export_txt_volume_title_label()}</span>
              <Select
                type="single"
                value={volumeKind}
                items={titleKindItems}
                onValueChange={(value) => handleTitleKindChange(value, "volume")}
              >
                <SelectTrigger
                  class="w-36 shrink-0"
                  aria-label={m.export_txt_volume_title_label()}
                  disabled={exporting}
                >
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {#each TXT_TITLE_KINDS as kind (kind)}
                    <SelectItem value={kind}>{titleKindLabel(kind)}</SelectItem>
                  {/each}
                </SelectContent>
              </Select>
              {#if volumeKind === "custom"}
                <Input
                  type="text"
                  value={volumePattern}
                  oninput={(event) => handlePatternInput("volume", event.currentTarget.value)}
                  placeholder={m.export_txt_volume_custom_placeholder()}
                  aria-label={m.export_txt_title_custom()}
                  disabled={exporting}
                  class="min-w-0 flex-1"
                />
              {:else if volumePreview !== null}
                <span class="min-w-0 flex-1 truncate text-xs text-muted-foreground">
                  {m.export_txt_preview_label()}: {volumePreview}
                </span>
              {/if}
            </div>
            {#if volumeKind === "custom"}
              <div class="flex min-h-6 items-center gap-2 py-1">
                {#if volumePreview !== null}
                  <span class="min-w-0 flex-1 truncate text-xs text-muted-foreground">
                    {m.export_txt_preview_label()}: {volumePreview}
                  </span>
                  <span class="shrink-0 text-xs text-muted-foreground">
                    {m.export_txt_custom_legend()}
                  </span>
                {/if}
              </div>
            {/if}
          {/if}
          <div class="flex items-center gap-2 py-2">
            <span class="shrink-0 text-sm font-medium">{m.export_txt_chapter_title_label()}</span>
            <Select
              type="single"
              value={chapterKind}
              items={titleKindItems}
              onValueChange={(value) => handleTitleKindChange(value, "chapter")}
            >
              <SelectTrigger
                class="w-36 shrink-0"
                aria-label={m.export_txt_chapter_title_label()}
                disabled={exporting}
              >
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {#each TXT_TITLE_KINDS as kind (kind)}
                  <SelectItem value={kind}>{titleKindLabel(kind)}</SelectItem>
                {/each}
              </SelectContent>
            </Select>
            {#if chapterKind === "custom"}
              <Input
                type="text"
                value={chapterPattern}
                oninput={(event) => handlePatternInput("chapter", event.currentTarget.value)}
                placeholder={m.export_txt_custom_placeholder()}
                aria-label={m.export_txt_title_custom()}
                disabled={exporting}
                class="min-w-0 flex-1"
              />
            {:else if chapterPreview !== null}
              <span class="min-w-0 flex-1 truncate text-xs text-muted-foreground">
                {m.export_txt_preview_label()}: {chapterPreview}
              </span>
            {/if}
          </div>
          {#if chapterKind === "custom"}
            <div class="flex min-h-6 items-center gap-2 py-1">
              {#if chapterPreview !== null}
                <span class="min-w-0 flex-1 truncate text-xs text-muted-foreground">
                  {m.export_txt_preview_label()}: {chapterPreview}
                </span>
                <span class="shrink-0 text-xs text-muted-foreground">
                  {m.export_txt_custom_legend()}
                </span>
              {/if}
            </div>
          {/if}
          {#if isVolumeBook}
            <div class="flex items-center gap-x-4 py-2">
              <span class="shrink-0 text-sm font-medium"
                >{m.export_txt_chapter_numbering_label()}</span
              >
              <RadioGroup.Root
                value={numbering}
                onValueChange={handleNumberingChange}
                class="flex items-center gap-x-4"
              >
                <label class="flex cursor-pointer items-center gap-1.5 text-sm">
                  <RadioGroup.Item
                    value="continuous"
                    aria-label={m.export_txt_numbering_continuous()}
                    disabled={exporting}
                  />
                  <span class="whitespace-nowrap">{m.export_txt_numbering_continuous()}</span>
                </label>
                <label class="flex cursor-pointer items-center gap-1.5 text-sm">
                  <RadioGroup.Item
                    value="per_volume"
                    aria-label={m.export_txt_numbering_per_volume()}
                    disabled={exporting}
                  />
                  <span class="whitespace-nowrap">{m.export_txt_numbering_per_volume()}</span>
                </label>
              </RadioGroup.Root>
            </div>
          {/if}
          <div class="flex items-center gap-2 py-2">
            <label class="flex min-w-0 flex-1 cursor-pointer items-center gap-2 text-sm">
              <Checkbox
                checked={metaEnabled}
                disabled={exporting}
                onCheckedChange={(value) => {
                  metaEnabled = value === true;
                  if (metaEnabled) {
                    metaFields = [...METADATA_FIELDS];
                  }
                }}
              />
              <span class="font-medium">{m.export_meta_label()}</span>
            </label>
            <Button variant="outline" size="sm" disabled={exporting} onclick={openMetaDialog}>
              {m.export_meta_choose()}
            </Button>
          </div>
        {/if}
      </div>
      <label class="flex cursor-pointer items-center gap-2 py-3 text-sm">
        <Checkbox
          checked={checkCover}
          disabled={!status.has_cover || exporting}
          onCheckedChange={(value) => (checkCover = value === true)}
        />
        <span class="min-w-0 flex-1">
          <span class="font-medium">{m.export_cover_label()}</span>
          {#if status.has_cover && status.cover_ext !== null}
            <span class="block font-mono text-xs text-muted-foreground">
              cover.{status.cover_ext}
            </span>
          {:else}
            <span class="block text-xs text-muted-foreground">{m.export_missing_cover()}</span>
          {/if}
        </span>
      </label>
      <div class="flex items-center gap-2 py-2">
        <span class="shrink-0 text-sm font-medium">{m.export_naming_title()}</span>
        <Select type="single" value={naming} items={namingItems} onValueChange={handleNamingChange}>
          <SelectTrigger
            class="min-w-0 flex-1"
            aria-label={m.export_naming_title()}
            disabled={exporting}
          >
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {#each kinds as kind (kind)}
              <SelectItem value={kind}>
                {kind === "custom"
                  ? m.export_naming_custom()
                  : `${kind === "book" ? m.export_naming_book() : m.export_naming_project()} · ${namingValue(kind, status.project_name, status.book_title)}`}
              </SelectItem>
            {/each}
          </SelectContent>
        </Select>
      </div>
      {#if naming === "custom"}
        <div class="flex items-center gap-2 py-2">
          <Input
            type="text"
            value={customName}
            oninput={(event) => (customName = event.currentTarget.value)}
            placeholder={m.export_custom_placeholder()}
            aria-label={m.export_naming_custom()}
            disabled={exporting}
            class="min-w-0 flex-1"
          />
        </div>
      {/if}
    </CardSection>
    <Button class="w-full" disabled={!ready} onclick={() => void runExport()}>
      {exporting ? m.export_running() : m.export_start()}
    </Button>
    {#if outcome !== null}
      <Card>
        <CardContent class="flex items-center gap-2 py-3">
          <span class="shrink-0 text-sm font-medium">{m.export_success()}</span>
          <span
            class="min-w-0 flex-1 truncate font-mono text-sm text-muted-foreground"
            title={outcome.dir}
          >
            {outcome.dir}
          </span>
          <Button
            variant="outline"
            size="sm"
            class="shrink-0"
            disabled={openingDir}
            onclick={() => void openExportDir()}
          >
            {m.export_open_dir()}
          </Button>
        </CardContent>
      </Card>
    {/if}
    <Dialog bind:open={metaDialogOpen}>
      <DialogContent aria-label={m.export_meta_dialog_title()}>
        <DialogHeader>
          <DialogTitle>{m.export_meta_dialog_title()}</DialogTitle>
        </DialogHeader>
        <div class="flex max-h-80 flex-col gap-1 overflow-y-auto py-2">
          {#each METADATA_FIELDS as field (field)}
            <label class="flex cursor-pointer items-center gap-2 py-1.5 text-sm">
              <Checkbox
                checked={metaDraft.includes(field)}
                onCheckedChange={(value) => toggleMetaDraft(field, value === true)}
              />
              <span>{metaFieldLabel(field)}</span>
            </label>
          {/each}
        </div>
        <DialogFooter>
          <Button variant="outline" size="sm" onclick={() => (metaDialogOpen = false)}>
            {m.projects_create_cancel()}
          </Button>
          <Button
            size="sm"
            onclick={() => {
              metaFields = [...metaDraft];
              metaEnabled = metaDraft.length > 0;
              metaDialogOpen = false;
            }}
          >
            {m.export_meta_dialog_ok()}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  {/if}
</div>
