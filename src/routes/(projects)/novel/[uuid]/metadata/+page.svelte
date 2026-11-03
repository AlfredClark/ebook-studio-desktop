<script lang="ts">
  // 小说元数据步骤页：进页拉取（缺失即建）、表单编辑保存、封面选图导入。
  import { onMount } from "svelte";
  import { page } from "$app/state";
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-dialog";
  import { Button } from "#components/shadcn-svelte/button";
  import ConfirmDialog from "#components/common/confirm-dialog.svelte";
  import commands from "#libs/commands";
  import type { ProjectMetadata } from "#libs/commands/types";
  import { m } from "#libs/i18n/paraglide/messages";
  import { toast } from "#libs/utils/toast";
  import MetadataForm from "../../../_components/metadata-form.svelte";

  /** 封面可选的图片扩展名，与后端白名单对齐。 */
  const COVER_EXTENSIONS = ["jpg", "jpeg", "png", "webp", "gif"];

  const uuid = $derived(page.params.uuid ?? "");
  let metadata = $state<ProjectMetadata | null>(null);
  let coverUrl = $state<string | null>(null);
  let loading = $state(true);
  let loadFailed = $state(false);
  let saving = $state(false);
  let importing = $state(false);
  let resetConfirmOpen = $state(false);

  /** 封面预览 URL：绝对路径转 `asset:`，失败或无封面回落空态。 */
  async function loadCoverUrl(): Promise<void> {
    const result = await commands.getProjectCoverPath(uuid).result();
    if (result.status === "error" || result.data == null) {
      coverUrl = null;
      return;
    }
    try {
      coverUrl = convertFileSrc(result.data);
    } catch {
      coverUrl = null;
    }
  }

  /** 进页拉取一次：后端缺文件即新建，失败走错误态可重试。 */
  async function loadMetadata(): Promise<void> {
    loading = true;
    loadFailed = false;
    const result = await commands.getProjectMetadata(uuid).result();
    loading = false;
    if (result.status === "error") {
      loadFailed = true;
      toast.error(m.metadata_load_failed());
      return;
    }
    metadata = result.data;
    await loadCoverUrl();
  }

  /** 保存：后端回填服务端字段，返回值直接落本地态。 */
  async function handleSave(next: ProjectMetadata): Promise<void> {
    saving = true;
    const result = await commands.updateProjectMetadata(uuid, next).result();
    saving = false;
    if (result.status === "error") {
      toast.error(m.metadata_save_failed());
      return;
    }
    metadata = result.data;
    toast.success(m.metadata_save_success());
  }

  /** 选封面：对话框取消直接返回；导入成功刷新元数据与预览。 */
  async function handleChooseCover(): Promise<void> {
    let picked: string | string[] | null;
    try {
      picked = await open({
        multiple: false,
        filters: [{ name: "Images", extensions: COVER_EXTENSIONS }],
      });
    } catch {
      toast.error(m.metadata_cover_import_failed());
      return;
    }
    if (typeof picked !== "string" || picked === "") {
      return;
    }
    importing = true;
    const result = await commands.importProjectCover(uuid, picked).result();
    importing = false;
    if (result.status === "error") {
      toast.error(m.metadata_cover_import_failed());
      return;
    }
    metadata = result.data;
    await loadCoverUrl();
  }

  /** 重置：确认后删文件按默认重建，刷新表单与封面预览。 */
  async function handleResetConfirm(): Promise<void> {
    const result = await commands.resetProjectMetadata(uuid).result();
    if (result.status === "error") {
      toast.error(m.metadata_reset_failed());
      return;
    }
    metadata = result.data;
    await loadCoverUrl();
    toast.success(m.metadata_reset_success());
  }

  onMount(() => {
    void loadMetadata();
  });
</script>

<h1 class="sr-only">{m.project_step_metadata()}</h1>
<div class="flex h-full min-h-0 flex-col">
  {#if loading}
    <p class="p-6 text-sm text-muted-foreground" aria-busy="true">{m.metadata_loading()}</p>
  {:else if loadFailed || metadata === null}
    <div class="flex flex-col items-center justify-center gap-3 p-6 text-center">
      <p class="text-sm text-muted-foreground">{m.metadata_load_failed()}</p>
      <Button variant="outline" size="sm" onclick={() => void loadMetadata()}>
        {m.metadata_retry()}
      </Button>
    </div>
  {:else}
    <MetadataForm
      {metadata}
      {coverUrl}
      projectType="novel"
      {saving}
      {importing}
      onSave={(next) => void handleSave(next)}
      onChooseCover={() => void handleChooseCover()}
      onReset={() => (resetConfirmOpen = true)}
    />
  {/if}
</div>
<ConfirmDialog
  bind:open={resetConfirmOpen}
  title={m.metadata_reset_confirm_title()}
  description={m.metadata_reset_confirm_description()}
  cancelLabel={m.projects_create_cancel()}
  confirmLabel={m.metadata_reset_confirm_ok()}
  variant="destructive"
  onConfirm={() => void handleResetConfirm()}
/>
