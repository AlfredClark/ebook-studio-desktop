<script lang="ts">
  // 校验信息列表：每行严重度图标 + 标准 ID + 描述 + 位置，右侧按钮复制本条文本。
  import CircleXIcon from "@lucide/svelte/icons/circle-x";
  import CopyIcon from "@lucide/svelte/icons/copy";
  import InfoIcon from "@lucide/svelte/icons/info";
  import TriangleAlertIcon from "@lucide/svelte/icons/triangle-alert";
  import { Button } from "#components/shadcn-svelte/button";
  import type { ValidationMessage, ValidationSeverity } from "#libs/commands/types";
  import { m } from "#libs/i18n/paraglide/messages";
  import { cn } from "#libs/utils/shadcn-svelte";
  import { toast } from "#libs/utils/toast";

  let { messages }: { messages: ValidationMessage[] } = $props();

  /** 严重度视觉桶：致命 / 错误红、警告琥珀、信息与建议灰。 */
  function toneOf(severity: ValidationSeverity): "error" | "warning" | "muted" {
    if (severity === "fatal" || severity === "error") {
      return "error";
    }
    if (severity === "warning") {
      return "warning";
    }
    return "muted";
  }

  /** 位置后缀：`路径:行:列`，缺项止步。 */
  function locationOf(message: ValidationMessage): string | null {
    if (message.path == null) {
      return null;
    }
    const parts = [message.path];
    if (message.line != null) {
      parts.push(String(message.line));
    }
    if (message.column != null) {
      parts.push(String(message.column));
    }
    return parts.join(":");
  }

  /** 复制本条：`[严重度] ID 描述 [位置]` 拼成单行文本。 */
  async function copyMessage(message: ValidationMessage): Promise<void> {
    const location = locationOf(message);
    const text =
      location == null
        ? `[${message.severity}] ${message.id} ${message.message}`
        : `[${message.severity}] ${message.id} ${message.message} [${location}]`;
    try {
      await navigator.clipboard.writeText(text);
      toast.success(m.package_copy_success());
    } catch {
      toast.error(m.package_copy_failed());
    }
  }
</script>

<ul class="flex flex-col divide-y divide-border">
  {#each messages as message, index (index)}
    {@const tone = toneOf(message.severity)}
    {@const location = locationOf(message)}
    <li class="flex items-start gap-2 py-2">
      {#if tone === "error"}
        <CircleXIcon class="mt-0.5 size-4 shrink-0 text-destructive" />
      {:else if tone === "warning"}
        <TriangleAlertIcon class="mt-0.5 size-4 shrink-0 text-amber-500" />
      {:else}
        <InfoIcon class="mt-0.5 size-4 shrink-0 text-muted-foreground" />
      {/if}
      <div class="min-w-0 flex-1">
        <p class="text-sm break-words">
          <span class="font-mono font-medium">{message.id}</span>
          <span class={cn("text-muted-foreground")}> {message.message}</span>
        </p>
        {#if location !== null}
          <p class="truncate font-mono text-xs text-muted-foreground">{location}</p>
        {/if}
      </div>
      <Button
        variant="ghost"
        size="icon-sm"
        class="shrink-0"
        aria-label={m.package_copy()}
        onclick={() => void copyMessage(message)}
      >
        <CopyIcon />
      </Button>
    </li>
  {/each}
</ul>
