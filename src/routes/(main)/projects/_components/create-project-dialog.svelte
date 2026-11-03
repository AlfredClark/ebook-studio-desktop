<script lang="ts">
  // 新建项目弹窗：标题随类型（新建小说 / 新建漫画），名称输入 + 创建/取消；
  // 每次打开清空输入，空名禁用确认，提交走后端命令，失败 toast 且保持打开可改。
  import { Button } from "#components/shadcn-svelte/button";
  import * as Dialog from "#components/shadcn-svelte/dialog";
  import { Input } from "#components/shadcn-svelte/input";
  import { m } from "#libs/i18n/paraglide/messages";
  import type { ProjectType } from "#libs/commands/types";

  let {
    open = $bindable(false),
    type,
    creating,
    onConfirm,
  }: {
    open: boolean;
    type: ProjectType;
    creating: boolean;
    onConfirm: (name: string) => void;
  } = $props();

  let name = $state("");

  /** 打开即清空，保证每次都是空白输入；关窗不同路径（确认/取消/右上/Esc）都覆盖到。 */
  $effect(() => {
    if (open) {
      name = "";
    }
  });

  /** 去首尾空白后仍有内容且非提交中才可提交。 */
  const canSubmit = $derived(name.trim() !== "" && !creating);

  /** 回车等同确认；空名禁用由按钮守住，此处再防一层。 */
  function handleSubmit(event: SubmitEvent): void {
    event.preventDefault();
    if (canSubmit) {
      onConfirm(name.trim());
    }
  }
</script>

<Dialog.Root bind:open>
  <Dialog.Content aria-describedby={undefined}>
    <Dialog.Header>
      <Dialog.Title>
        {type === "novel"
          ? m.projects_create_dialog_title_novel()
          : m.projects_create_dialog_title_comic()}
      </Dialog.Title>
    </Dialog.Header>
    <form class="flex flex-col gap-4" onsubmit={handleSubmit}>
      <div class="flex flex-col gap-1.5">
        <label for="project-name" class="text-sm font-medium">
          {m.projects_create_name_label()}
        </label>
        <Input
          id="project-name"
          bind:value={name}
          type="text"
          maxlength={100}
          placeholder={m.projects_create_name_placeholder()}
          autocomplete="off"
        />
      </div>
      <Dialog.Footer>
        <Button variant="outline" type="button" onclick={() => (open = false)}>
          {m.projects_create_cancel()}
        </Button>
        <Button type="submit" disabled={!canSubmit}>
          {m.projects_create_confirm()}
        </Button>
      </Dialog.Footer>
    </form>
  </Dialog.Content>
</Dialog.Root>
