<script lang="ts">
  // 漫画后续步骤引导（哑组件群的智能外壳）：无解析产物时引导回文件步骤，
  // 已有大纲时展示识别规模并提示待接入；编辑 / 构建 / 打包 / 导出四页共用。
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { resolve } from "$app/paths";
  import FileTextIcon from "@lucide/svelte/icons/file-text";
  import ImageIcon from "@lucide/svelte/icons/image";
  import { Button } from "#components/shadcn-svelte/button";
  import commands from "#libs/commands";
  import type { ComicOutline } from "#libs/commands/types";
  import { m } from "#libs/i18n/paraglide/messages";
  import { toast } from "#libs/utils/toast";

  let { uuid }: { uuid: string } = $props();

  let outline = $state<ComicOutline | null>(null);
  let loading = $state(true);
  let loadFailed = $state(false);

  /** 进页拉取大纲：缺失回落空态引导去文件步骤，失败走错误态可重试。 */
  async function loadOutline(): Promise<void> {
    loading = true;
    loadFailed = false;
    outline = null;
    const result = await commands.getComicContentOutline(uuid).result();
    loading = false;
    if (result.status === "error") {
      loadFailed = true;
      toast.error(m.comic_files_load_failed());
      return;
    }
    outline = result.data;
  }

  onMount(() => {
    void loadOutline();
  });
</script>

{#if loading}
  <div class="mx-auto flex w-full max-w-3xl flex-1 flex-col p-6">
    <div class="flex flex-1 items-center justify-center">
      <p class="text-sm text-muted-foreground" aria-busy="true">{m.comic_files_loading()}</p>
    </div>
  </div>
{:else if loadFailed}
  <div class="mx-auto flex w-full max-w-3xl flex-1 flex-col p-6">
    <div class="flex flex-1 flex-col items-center justify-center gap-3 text-center">
      <p class="text-sm text-muted-foreground">{m.comic_files_load_failed()}</p>
      <Button variant="outline" class="w-full max-w-md" onclick={() => void loadOutline()}>
        {m.files_retry()}
      </Button>
    </div>
  </div>
{:else if outline === null}
  <div class="mx-auto flex w-full max-w-3xl flex-1 flex-col p-6">
    <div class="flex flex-1 flex-col items-center justify-center gap-2 text-center">
      <FileTextIcon class="size-10 text-muted-foreground" />
      <p class="text-sm font-medium">{m.comic_guide_empty_title()}</p>
      <p class="text-sm text-muted-foreground">{m.comic_guide_empty_description()}</p>
      <Button class="w-full max-w-md" onclick={() => void goto(resolve(`/comic/${uuid}/files`))}>
        {m.comic_guide_goto_files()}
      </Button>
    </div>
  </div>
{:else}
  <div class="mx-auto flex w-full max-w-3xl flex-1 flex-col p-6">
    <div class="flex flex-1 flex-col items-center justify-center gap-2 text-center">
      <ImageIcon class="size-10 text-muted-foreground" />
      <p class="text-sm font-medium">{m.comic_guide_pending_title()}</p>
      <p class="text-sm text-muted-foreground">
        {m.comic_guide_pending_description({
          volumes: outline.volume_count,
          pages: outline.page_count,
        })}
      </p>
      <Button
        variant="outline"
        class="w-full max-w-md"
        onclick={() => void goto(resolve(`/comic/${uuid}/files`))}
      >
        {m.comic_guide_goto_files()}
      </Button>
    </div>
  </div>
{/if}
