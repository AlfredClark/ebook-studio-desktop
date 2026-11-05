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
  import CardSection from "#components/common/card-section.svelte";
  import commands from "#libs/commands";
  import type { ExportNaming, ExportOutcome, ExportStatus } from "#libs/commands/types";
  import { m } from "#libs/i18n/paraglide/messages";
  import { toast } from "#libs/utils/toast";
  import {
    canExport,
    isValidCustomName,
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
  /** 导出可用：有勾选、命名合法、空闲中。 */
  const ready = $derived(
    status !== null &&
      canExport({ epub: checkEpub, txt: checkTxt, cover: checkCover }, status) &&
      (naming !== "custom" || isValidCustomName(customName)) &&
      !exporting &&
      !loading,
  );

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
    outcome = null;
  }

  /** 下拉给字符串，经候选项收窄后才提交，无类型断言。 */
  function handleNamingChange(value: string): void {
    if (value === "project" || value === "book" || value === "custom") {
      naming = value;
    }
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
      .exportNovelContent(uuid, dest, naming, customName, {
        epub: checkEpub,
        txt: checkTxt,
        cover: checkCover,
      })
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
      <label class="flex cursor-pointer items-center gap-2 py-3 text-sm">
        <Checkbox
          checked={checkTxt}
          disabled={!status.has_content || exporting}
          onCheckedChange={(value) => (checkTxt = value === true)}
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
  {/if}
</div>
