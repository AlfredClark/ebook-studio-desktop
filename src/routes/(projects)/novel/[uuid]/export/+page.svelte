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
  import CardSection from "#components/common/card-section.svelte";
  import commands from "#libs/commands";
  import type {
    ExportNaming,
    ExportOutcome,
    ExportStatus,
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
  /** TXT 标题格式与自定义模板。 */
  let titleKind = $state<TxtTitleKind>("original");
  let titlePattern = $state("");
  /** 格式示例：首章渲染一行；原文与自定义不展示，失败静默隐藏。 */
  let titlePreview = $state<string | null>(null);
  let previewNonce = $state(0);
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
  /** 导出可用：有勾选、命名合法、TXT 参数合法、空闲中。 */
  const ready = $derived(
    status !== null &&
      canExport({ epub: checkEpub, txt: checkTxt, cover: checkCover }, status) &&
      (naming !== "custom" || isValidCustomName(customName)) &&
      (!checkTxt || isValidTxtOptions(titleKind, titlePattern, metaEnabled, metaFields)) &&
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

  /** 进页探针：有产物默认全选，无产物展示空态引导。 */
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
    checkEpub = status.has_epub;
    checkTxt = status.has_content;
    checkCover = status.has_cover;
    naming = "project";
    customName = "";
    titleKind = "original";
    titlePattern = "";
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

  /** 标题格式下拉：同上收窄，切换后刷新示例。 */
  function handleTitleKindChange(value: string): void {
    if ((TXT_TITLE_KINDS as string[]).includes(value)) {
      titleKind = value as TxtTitleKind;
      void refreshTitlePreview();
    }
  }

  /** 格式示例：内置格式取首章渲染，失败静默隐藏；序号防迟到覆盖。 */
  async function refreshTitlePreview(): Promise<void> {
    titlePreview = null;
    if (!checkTxt || titleKind === "original" || titleKind === "custom") {
      return;
    }
    const nonce = (previewNonce += 1);
    const result = await commands.previewNovelTxtTitle(uuid, titleKind, "").result();
    if (nonce !== previewNonce || result.status === "error") {
      return;
    }
    titlePreview = result.data;
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
          title_kind: titleKind,
          title_pattern: titlePattern,
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
      <Button variant="outline" size="sm" onclick={() => void loadStatus()}>
        {m.files_retry()}
      </Button>
    </div>
  {:else if status !== null && isEmpty}
    <div class="flex flex-1 flex-col items-center justify-center gap-2 text-center">
      <DownloadIcon class="size-10 text-muted-foreground" />
      <p class="text-sm font-medium">{m.export_empty_title()}</p>
      <p class="text-sm text-muted-foreground">{m.export_empty_description()}</p>
      <Button size="sm" onclick={() => void goto(resolve(`/novel/${uuid}/files`))}>
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
                void refreshTitlePreview();
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
          <div class="flex items-center gap-2 py-2">
            <span class="shrink-0 text-sm font-medium">{m.export_txt_title_label()}</span>
            <Select
              type="single"
              value={titleKind}
              items={titleKindItems}
              onValueChange={handleTitleKindChange}
            >
              <SelectTrigger
                class="w-36 shrink-0"
                aria-label={m.export_txt_title_label()}
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
            {#if titleKind === "custom"}
              <Input
                type="text"
                value={titlePattern}
                oninput={(event) => (titlePattern = event.currentTarget.value)}
                placeholder={m.export_txt_custom_placeholder()}
                aria-label={m.export_txt_title_custom()}
                disabled={exporting}
                class="min-w-0 flex-1"
              />
            {:else if titleKind !== "original" && titlePreview !== null}
              <span class="min-w-0 flex-1 truncate text-xs text-muted-foreground">
                {m.export_txt_preview_label()}: {titlePreview}
              </span>
            {/if}
          </div>
          <div class="flex items-center gap-2 py-2">
            <label class="flex min-w-0 flex-1 cursor-pointer items-center gap-2 text-sm">
              <Checkbox
                checked={metaEnabled}
                disabled={exporting}
                onCheckedChange={(value) => (metaEnabled = value === true)}
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
    </CardSection>
    <CardSection title={m.export_naming_title()}>
      <div class="flex flex-col gap-1.5 py-3">
        <Select type="single" value={naming} items={namingItems} onValueChange={handleNamingChange}>
          <SelectTrigger class="w-full" aria-label={m.export_naming_title()}>
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
        {#if naming === "custom"}
          <Input
            type="text"
            value={customName}
            oninput={(event) => (customName = event.currentTarget.value)}
            placeholder={m.export_custom_placeholder()}
            aria-label={m.export_naming_custom()}
          />
        {/if}
      </div>
    </CardSection>
    <Button size="sm" disabled={!ready} onclick={() => void runExport()}>
      {exporting ? m.export_running() : m.export_start()}
    </Button>
    {#if outcome !== null}
      <CardSection title={m.export_success()} description={m.export_dir_label()}>
        <p class="py-3 font-mono text-xs break-all text-muted-foreground">{outcome.dir}</p>
        <ul class="flex flex-col divide-y divide-border">
          {#each outcome.files as file (file)}
            <li class="truncate py-2 font-mono text-sm">{file}</li>
          {/each}
        </ul>
      </CardSection>
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
