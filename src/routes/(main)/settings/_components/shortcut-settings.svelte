<script lang="ts">
  // 快捷键设置分组：“快捷显示”全局快捷键，后端持久化（config.json）+ Rust 侧动态注册。
  // 展示态用徽章加号呈现组合；编辑态切换为录制，Enter 提交、Esc 取消、Backspace 清空。
  import { configState, updateConfig } from "#libs/hooks/config.svelte";
  import CardRow from "#components/common/card-row.svelte";
  import CardSection from "#components/common/card-section.svelte";
  import { Badge } from "#components/shadcn-svelte/badge";
  import { Button } from "#components/shadcn-svelte/button";
  import CheckIcon from "@lucide/svelte/icons/check";
  import PencilIcon from "@lucide/svelte/icons/pencil";
  import PlusIcon from "@lucide/svelte/icons/plus";
  import XIcon from "@lucide/svelte/icons/x";
  import { SvelteSet } from "svelte/reactivity";
  import { m } from "#libs/i18n/paraglide/messages";
  import { toast } from "#libs/utils/toast";
  import {
    SHORTCUT_MODIFIER_ORDER,
    keyToMainKey,
    keyToModifier,
    normalizeRecorded,
    splitShortcut,
    type ShortcutModifier,
  } from "#libs/shortcut/shortcut";

  /** 后端配置是唯一真值；水合前按空（未设置）渲染 */
  const current = $derived(configState.value?.shortcut_show ?? "");
  /** 录制中 */
  let recording = $state(false);
  /** 落盘在途时禁用确认，避免重复提交 */
  let saving = $state(false);
  /** 录制中累计的修饰键（`SvelteSet` 自带响应式，无需 `$state` 包裹） */
  let modifiers = new SvelteSet<ShortcutModifier>();
  /** 录制中捕获的主键 */
  let mainKey = $state<string | null>(null);

  /** 录制中的预览组合；缺主键或零修饰键时为空，不可提交 */
  const preview = $derived(normalizeRecorded(modifiers, mainKey));
  /** 有序的已捕获修饰键，供录制态实时展示 */
  const orderedModifiers = $derived(
    SHORTCUT_MODIFIER_ORDER.filter((modifier) => modifiers.has(modifier)),
  );
  /** 展示片段：录制中看实时捕获，否则看存量 */
  const parts = $derived(
    recording
      ? [...orderedModifiers, ...(mainKey === null ? [] : [mainKey])]
      : splitShortcut(current),
  );

  /** 进入录制：清空上次捕获 */
  function startRecording(): void {
    modifiers.clear();
    mainKey = null;
    recording = true;
  }

  /** 退出录制并丢弃捕获 */
  function stopRecording(): void {
    recording = false;
    modifiers.clear();
    mainKey = null;
  }

  /** 提交组合：与存量一致直接收尾；成功静默，失败弹必须手动关闭的错误提示 */
  async function submit(next: string): Promise<void> {
    if (next === current) {
      stopRecording();
      return;
    }
    if (saving) return;
    saving = true;
    try {
      if (!(await updateConfig({ shortcut_show: next }))) {
        toast.error(m.settings_shortcut_register_failed(), {
          duration: Infinity,
          closeButton: true,
        });
      }
    } finally {
      saving = false;
      stopRecording();
    }
  }

  /** 确认录制：组合不完整时停留录制态 */
  async function confirmRecording(): Promise<void> {
    const next = normalizeRecorded(modifiers, mainKey);
    if (next === null) return;
    await submit(next);
  }

  // 录制态全局按键：Enter 提交、Esc 取消、Backspace 清空为未设置，其余累积捕获
  $effect(() => {
    if (!recording) return;
    function onKeyDown(event: KeyboardEvent): void {
      event.preventDefault();
      if (event.repeat) return;
      if (event.key === "Escape") {
        stopRecording();
        return;
      }
      if (event.key === "Enter") {
        void confirmRecording();
        return;
      }
      if (event.key === "Backspace") {
        void submit("");
        return;
      }
      const modifier = keyToModifier(event.key);
      if (modifier !== null) {
        modifiers.add(modifier);
        return;
      }
      const key = keyToMainKey(event.key);
      if (key !== null) mainKey = key;
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  });
</script>

<CardSection title={m.settings_shortcut_title()} description={m.settings_shortcut_description()}>
  <CardRow
    label={m.settings_shortcut_show_label()}
    description={m.settings_shortcut_show_description()}
  >
    {#snippet control()}
      <div class="flex min-w-0 items-center justify-end gap-2">
        {#if parts.length > 0}
          <!-- 单行展示：常见组合恰好占满控件列，超长时横向滚动（滚动条隐藏），尾部主键始终可见 -->
          <span
            class="flex min-w-0 flex-1 [scrollbar-width:none] flex-nowrap items-center justify-end gap-1 overflow-x-auto [-ms-overflow-style:none] [&::-webkit-scrollbar]:hidden"
          >
            {#each parts as part, index (index)}
              {#if index > 0}
                <PlusIcon class="size-3 shrink-0 text-muted-foreground" />
              {/if}
              <Badge variant="secondary" class="tabular-nums">{part}</Badge>
            {/each}
          </span>
        {:else}
          <span class="text-sm text-muted-foreground">
            {recording ? m.settings_shortcut_recording() : m.settings_shortcut_unset()}
          </span>
        {/if}
        {#if recording}
          <Button
            variant="ghost"
            size="icon-sm"
            aria-label={m.settings_shortcut_confirm()}
            disabled={preview === null || saving}
            onclick={() => void confirmRecording()}
          >
            <CheckIcon />
          </Button>
          <Button
            variant="ghost"
            size="icon-sm"
            aria-label={m.settings_shortcut_cancel()}
            disabled={saving}
            onclick={stopRecording}
          >
            <XIcon />
          </Button>
        {:else}
          <Button
            variant="ghost"
            size="icon-sm"
            aria-label={m.settings_shortcut_edit()}
            onclick={startRecording}
          >
            <PencilIcon />
          </Button>
        {/if}
      </div>
    {/snippet}
  </CardRow>
</CardSection>
