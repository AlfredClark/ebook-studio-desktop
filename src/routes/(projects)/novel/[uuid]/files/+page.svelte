<script lang="ts">
  // 小说文件步骤页：左栏原文导入与分卷分章参数，右栏识别结果预览（解析接入前为占位）。
  import { onMount } from "svelte";
  import { page } from "$app/state";
  import { open } from "@tauri-apps/plugin-dialog";
  import FileTextIcon from "@lucide/svelte/icons/file-text";
  import { Button } from "#components/shadcn-svelte/button";
  import * as Resizable from "#components/shadcn-svelte/resizable";
  import commands from "#libs/commands";
  import type { NovelTextStatus, ParseOutline } from "#libs/commands/types";
  import { m } from "#libs/i18n/paraglide/messages";
  import { toast } from "#libs/utils/toast";
  import ParseParamsCard from "./_components/parse-params-card.svelte";
  import ParseOutlineCard from "./_components/parse-outline-card.svelte";
  import TxtSourceCard from "./_components/txt-source-card.svelte";
  import {
    DEFAULT_CHAPTER_PATTERN,
    DEFAULT_VOLUME_PATTERN,
    isValidPattern,
  } from "./_components/novel-text-params";

  const uuid = $derived(page.params.uuid ?? "");
  let status = $state<NovelTextStatus | null>(null);
  let loading = $state(true);
  let loadFailed = $state(false);
  let importing = $state(false);
  /** 卷章正则：本步仅本地态暂存，解析命令接入后再提交。 */
  let volumePattern = $state(DEFAULT_VOLUME_PATTERN);
  let chapterPattern = $state(DEFAULT_CHAPTER_PATTERN);
  /** 识别大纲：解析成功后落本地态，右栏渲染卷章标题树。 */
  let outline = $state<ParseOutline | null>(null);
  let parsing = $state(false);
  /** 强制重排进行中：按钮禁用态。 */
  let renumbering = $state(false);

  /** 解析参数是否合法：按钮可用性的唯一门禁。 */
  const patternsValid = $derived(isValidPattern(volumePattern) && isValidPattern(chapterPattern));

  /** 进页拉取一次：原文状态缺文件回落空态，已有 content.json 直接回显大纲，失败走错误态可重试。 */
  async function loadStatus(): Promise<void> {
    loading = true;
    loadFailed = false;
    outline = null;
    const result = await commands.getNovelTextStatus(uuid).result();
    if (result.status === "error") {
      loading = false;
      loadFailed = true;
      toast.error(m.files_load_failed());
      return;
    }
    status = result.data;
    const outlineResult = await commands.getNovelContentOutline(uuid).result();
    loading = false;
    if (outlineResult.status === "error") {
      loadFailed = true;
      toast.error(m.files_parse_failed());
      return;
    }
    outline = outlineResult.data;
  }

  /** 选 TXT 并导入：对话框取消直接返回；成功刷新状态，失败 toast。 */
  async function handleChoose(): Promise<void> {
    let picked: string | string[] | null;
    try {
      picked = await open({
        multiple: false,
        filters: [{ name: "Text", extensions: ["txt"] }],
      });
    } catch {
      toast.error(m.files_source_failed());
      return;
    }
    if (typeof picked !== "string" || picked === "") {
      return;
    }
    importing = true;
    const result = await commands.importNovelText(uuid, picked).result();
    importing = false;
    if (result.status === "error") {
      toast.error(m.files_source_failed());
      return;
    }
    status = result.data;
    // 新原文使旧大纲失效，待重新解析。
    outline = null;
  }

  /** 开始解析：调后端切分并落盘 content.json，大纲进右栏，失败 toast。 */
  async function handleParse(): Promise<void> {
    if (!patternsValid || parsing) {
      return;
    }
    parsing = true;
    const result = await commands.parseNovelText(uuid, volumePattern, chapterPattern).result();
    parsing = false;
    if (result.status === "error") {
      toast.error(m.files_parse_failed());
      return;
    }
    outline = result.data;
    if (result.data.numbers_missing) {
      toast.warning(m.files_numbers_missing());
    }
  }

  /** 强制重排：确认后按文档顺序重写卷章编号，大纲刷新，失败 toast。 */
  async function handleRenumber(): Promise<void> {
    if (renumbering) {
      return;
    }
    renumbering = true;
    const result = await commands.renumberNovelContent(uuid).result();
    renumbering = false;
    if (result.status === "error") {
      toast.error(m.files_renumber_failed());
      return;
    }
    outline = result.data;
    toast.success(m.files_renumber_success());
  }

  onMount(() => {
    void loadStatus();
  });
</script>

<h1 class="sr-only">{m.project_step_files()}</h1>
<div class="flex h-full min-h-0 w-full min-w-0">
  <Resizable.PaneGroup direction="horizontal">
    <Resizable.Pane defaultSize={50} minSize={30} maxSize={70} class="min-w-0">
      <div class="flex h-full min-h-0 flex-col gap-4 overflow-auto p-6">
        {#if loading}
          <p class="text-sm text-muted-foreground" aria-busy="true">{m.files_loading()}</p>
        {:else if loadFailed || status === null}
          <div class="flex flex-col items-center justify-center gap-3 text-center">
            <p class="text-sm text-muted-foreground">{m.files_load_failed()}</p>
            <Button variant="outline" size="sm" onclick={() => void loadStatus()}>
              {m.files_retry()}
            </Button>
          </div>
        {:else}
          <TxtSourceCard {status} {importing} onChoose={() => void handleChoose()} />
          <ParseParamsCard bind:volumePattern bind:chapterPattern />
          <div class="mt-auto pt-2">
            <Button
              class="w-full"
              disabled={!patternsValid || parsing}
              onclick={() => void handleParse()}
            >
              {parsing ? m.files_parsing() : m.files_parse_start()}
            </Button>
          </div>
        {/if}
      </div>
    </Resizable.Pane>
    <Resizable.Handle withHandle aria-label={m.page_files_resize_label()} />
    <Resizable.Pane class="min-w-0">
      <div class="flex h-full min-h-0 flex-col p-6">
        {#if parsing}
          <p class="text-sm text-muted-foreground" aria-busy="true">{m.files_parsing()}</p>
        {:else if outline !== null}
          <div class="min-h-0 flex-1 overflow-auto">
            <ParseOutlineCard {outline} {renumbering} onRenumber={() => void handleRenumber()} />
          </div>
        {:else}
          <div
            class="flex h-full min-h-0 flex-1 flex-col items-center justify-center gap-2 text-center"
          >
            <FileTextIcon class="size-10 text-muted-foreground" />
            <p class="text-sm font-medium">{m.files_parse_result_title()}</p>
            <p class="text-sm text-muted-foreground">{m.files_preview_empty_description()}</p>
          </div>
        {/if}
      </div>
    </Resizable.Pane>
  </Resizable.PaneGroup>
</div>
