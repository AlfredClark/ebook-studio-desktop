<script lang="ts">
  // 小说打包页：无解析产物引导去文件、无构建引导去构建、无产物引导打包、有产物展示信息并自动校验；校验列表逐条可复制。
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import { resolve } from "$app/paths";
  import FileTextIcon from "@lucide/svelte/icons/file-text";
  import CircleCheckIcon from "@lucide/svelte/icons/circle-check";
  import PackageIcon from "@lucide/svelte/icons/package";
  import { Badge } from "#components/shadcn-svelte/badge";
  import { Button } from "#components/shadcn-svelte/button";
  import {
    Card,
    CardContent,
    CardDescription,
    CardHeader,
    CardTitle,
  } from "#components/shadcn-svelte/card";
  import { Progress } from "#components/shadcn-svelte/progress";
  import { ScrollArea } from "#components/shadcn-svelte/scroll-area";
  import CardSection from "#components/common/card-section.svelte";
  import ConfirmDialog from "#components/common/confirm-dialog.svelte";
  import commands from "#libs/commands";
  import type { PackageInfo, ValidationOutcome } from "#libs/commands/types";
  import { m } from "#libs/i18n/paraglide/messages";
  import { toast } from "#libs/utils/toast";
  import {
    countValidation,
    formatPackageSize,
    formatPackageTime,
  } from "./_components/package-format";
  import { readCachedValidation, writeCachedValidation } from "./_components/validation-cache";
  import ValidationList from "./_components/validation-list.svelte";

  const uuid = $derived(page.params.uuid ?? "");

  /** 打包进度事件名（见 Rust 侧 `features::novel_package::PACKAGE_PROGRESS_EVENT`）。 */
  const PROGRESS_EVENT = "novel-package-progress";

  let pkg = $state<PackageInfo | null>(null);
  let hasBuild = $state(false);
  /** 是否有解析产物：无构建时决定整页引导去文件还是去构建。 */
  let hasContent = $state(true);
  let loading = $state(true);
  let statusFailed = $state(false);
  let packaging = $state(false);
  /** 打包进度：`zip` 确定性百分比 / `validate` 不确定态；`null` 表无可显示进度。 */
  let progress = $state<{ stage: "zip" | "validate"; percent: number } | null>(null);
  let validation = $state<ValidationOutcome | null>(null);
  let validating = $state(false);
  let validateFailed = $state(false);
  let confirmRepack = $state(false);

  /** 校验计数：汇总行展示用，无结果时不展示。 */
  const counts = $derived(validation == null ? null : countValidation(validation.messages));

  /** 进页恢复：有产物直接展示并自动校验，无产物查构建树与解析产物决定引导去向。 */
  async function loadStatus(): Promise<void> {
    loading = true;
    statusFailed = false;
    hasContent = true;
    const status = await commands.getNovelPackageStatus(uuid).result();
    if (status.status === "error") {
      loading = false;
      statusFailed = true;
      toast.error(m.package_status_failed());
      return;
    }
    pkg = status.data;
    if (pkg == null) {
      const tree = await commands.getNovelBuildTree(uuid).result();
      if (tree.status === "error") {
        loading = false;
        statusFailed = true;
        toast.error(m.package_status_failed());
        return;
      }
      hasBuild = tree.data != null;
      if (!hasBuild) {
        const outlineResult = await commands.getNovelContentOutline(uuid).result();
        if (outlineResult.status === "error") {
          loading = false;
          statusFailed = true;
          toast.error(m.package_status_failed());
          return;
        }
        const outline = outlineResult.data;
        hasContent =
          outline !== null && outline.volumes.some((volume) => volume.chapters.length > 0);
      }
      loading = false;
      return;
    }
    loading = false;
    // 会话缓存命中直接展示，仅未命中才走后端校验。
    const cached = readCachedValidation(uuid, pkg);
    if (cached != null) {
      validation = cached;
      validateFailed = false;
      return;
    }
    await runValidate();
  }

  /** 打包：进度经事件推送，落盘后后端已附带一次校验，直接落态展示。 */
  async function runPackage(): Promise<void> {
    if (packaging) {
      return;
    }
    packaging = true;
    progress = null;
    // 进度订阅：非 Tauri 环境（纯浏览器预览）无事件可订，按钮文案即反馈。
    let unlisten: (() => void) | null = null;
    try {
      const { listen } = await import("@tauri-apps/api/event");
      unlisten = await listen<{
        id: string;
        stage: "zip" | "validate";
        done_files: number;
        total_files: number;
      }>(PROGRESS_EVENT, (event) => {
        if (event.payload.id !== uuid) {
          return;
        }
        if (event.payload.stage === "zip") {
          const total = event.payload.total_files;
          progress = {
            stage: "zip",
            percent: total === 0 ? 0 : Math.round((event.payload.done_files / total) * 100),
          };
        } else {
          progress = { stage: "validate", percent: 100 };
        }
      });
    } catch {
      progress = null;
    }
    const result = await commands.packageNovelContent(uuid).result();
    packaging = false;
    progress = null;
    unlisten?.();
    if (result.status === "error") {
      toast.error(m.package_failed());
      return;
    }
    pkg = result.data.package;
    validation = result.data.validation;
    validateFailed = false;
    writeCachedValidation(uuid, pkg, validation);
    toast.success(m.package_success());
  }

  /** 手动校验：恒走后端（用户显式动作，不读缓存），成功后刷新缓存。 */
  async function runValidate(): Promise<void> {
    if (validating) {
      return;
    }
    validating = true;
    validateFailed = false;
    const result = await commands.validateNovelPackage(uuid).result();
    validating = false;
    if (result.status === "error") {
      validateFailed = true;
      toast.error(m.package_validate_failed());
      return;
    }
    validation = result.data;
    if (pkg != null) {
      writeCachedValidation(uuid, pkg, validation);
    }
  }

  onMount(() => {
    void loadStatus();
  });
</script>

<!-- 小说打包页：产物信息 + 自动校验 + 逐条可复制的校验列表 -->
<h1 class="sr-only">{m.project_step_package()}</h1>
{#snippet progressView()}
  {#if packaging && progress !== null}
    <div class="flex w-full flex-col gap-1.5" role="status" aria-live="polite">
      <p class="text-sm text-muted-foreground">
        {progress.stage === "zip" ? m.package_stage_zip() : m.package_stage_validate()}
        {#if progress.stage === "zip"}
          <span class="tabular-nums">{progress.percent}%</span>
        {/if}
      </p>
      {#if progress.stage === "zip"}
        <Progress value={progress.percent} />
      {/if}
    </div>
  {/if}
{/snippet}
<div class="mx-auto flex w-full max-w-3xl flex-1 flex-col gap-4 overflow-hidden p-6">
  {#if loading}
    <div class="flex flex-1 items-center justify-center">
      <p class="text-sm text-muted-foreground" aria-busy="true">{m.package_loading()}</p>
    </div>
  {:else if statusFailed}
    <div class="flex flex-1 flex-col items-center justify-center gap-3 text-center">
      <p class="text-sm text-muted-foreground">{m.package_status_failed()}</p>
      <Button variant="outline" class="w-full max-w-md" onclick={() => void loadStatus()}>
        {m.files_retry()}
      </Button>
    </div>
  {:else if pkg === null}
    <div class="flex flex-1 flex-col items-center justify-center gap-2 text-center">
      {#if !hasContent}
        <FileTextIcon class="size-10 text-muted-foreground" />
        <p class="text-sm font-medium">{m.edit_empty_title()}</p>
        <p class="text-sm text-muted-foreground">{m.edit_empty_description()}</p>
        <Button class="w-full max-w-md" onclick={() => void goto(resolve(`/novel/${uuid}/files`))}>
          {m.edit_goto_files()}
        </Button>
      {:else if !hasBuild}
        <PackageIcon class="size-10 text-muted-foreground" />
        <p class="text-sm font-medium">{m.package_no_build_title()}</p>
        <p class="text-sm text-muted-foreground">{m.package_no_build_description()}</p>
        <Button class="w-full max-w-md" onclick={() => void goto(resolve(`/novel/${uuid}/build`))}>
          {m.package_goto_build()}
        </Button>
      {:else}
        <PackageIcon class="size-10 text-muted-foreground" />
        <p class="text-sm font-medium">{m.package_ready_title()}</p>
        <p class="text-sm text-muted-foreground">{m.package_ready_description()}</p>
        <Button class="w-full max-w-md" disabled={packaging} onclick={() => void runPackage()}>
          {packaging ? m.package_running() : m.package_start()}
        </Button>
        {@render progressView()}
      {/if}
    </div>
  {:else}
    <CardSection title={pkg.file_name} description={m.package_success()}>
      <div class="grid grid-cols-3 gap-4 py-3">
        <div class="min-w-0">
          <p class="text-sm font-medium">{m.package_file_label()}</p>
          <p class="truncate text-sm tabular-nums" title={pkg.file_name}>{pkg.file_name}</p>
        </div>
        <div class="min-w-0">
          <p class="text-sm font-medium">{m.package_size_label()}</p>
          <p class="truncate text-sm tabular-nums">{formatPackageSize(pkg.size_bytes)}</p>
        </div>
        <div class="min-w-0">
          <p class="text-sm font-medium">{m.package_time_label()}</p>
          <p class="truncate text-sm tabular-nums" title={formatPackageTime(pkg.updated_at)}>
            {formatPackageTime(pkg.updated_at)}
          </p>
        </div>
      </div>
    </CardSection>
    <div class="flex items-center gap-2">
      <Button
        variant="outline"
        size="sm"
        disabled={packaging}
        onclick={() => (confirmRepack = true)}
      >
        {packaging ? m.package_running() : m.package_repackage()}
      </Button>
      <Button variant="outline" size="sm" disabled={validating} onclick={() => void runValidate()}>
        {validating ? m.package_validating() : m.package_revalidate()}
      </Button>
      <div class="ml-auto">
        {#if validation == null || validation.valid}
          <Badge variant="default">
            <CircleCheckIcon />
            {m.package_valid()}
          </Badge>
        {:else}
          <Badge variant="destructive">{m.package_invalid()}</Badge>
        {/if}
      </div>
    </div>
    {@render progressView()}
    {#if validating}
      <p class="text-sm text-muted-foreground" aria-busy="true">{m.package_validating()}</p>
    {:else if validateFailed}
      <div class="flex flex-col items-start gap-2">
        <p class="text-sm text-muted-foreground">{m.package_validate_failed()}</p>
        <Button variant="outline" size="sm" onclick={() => void runValidate()}>
          {m.files_retry()}
        </Button>
      </div>
    {:else if validation !== null}
      {#if validation.messages.length === 0}
        <div class="flex items-center gap-2 text-sm">
          <CircleCheckIcon class="size-4 text-primary" />
          <p>{m.package_no_issues()}</p>
        </div>
      {:else}
        <Card class="flex min-h-0 flex-1 flex-col">
          <CardHeader>
            <CardTitle>{validation.valid ? m.package_valid() : m.package_invalid()}</CardTitle>
            {#if counts !== null}
              <CardDescription>
                {m.package_summary({ errors: counts.errors, warnings: counts.warnings })}
              </CardDescription>
            {/if}
          </CardHeader>
          <CardContent class="min-h-0 flex-1">
            <ScrollArea class="h-full pr-3">
              <ValidationList messages={validation.messages} />
            </ScrollArea>
          </CardContent>
        </Card>
      {/if}
    {/if}
  {/if}
</div>
<ConfirmDialog
  bind:open={confirmRepack}
  title={m.package_repackage_confirm_title()}
  description={m.package_repackage_confirm_description()}
  cancelLabel={m.projects_create_cancel()}
  confirmLabel={m.package_repackage_confirm_ok()}
  onConfirm={() => void runPackage()}
/>
