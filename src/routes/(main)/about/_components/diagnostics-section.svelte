<script lang="ts">
  // 诊断分组：打开日志/配置目录 + 复制系统信息，各行独立命令、失败 toast，零 props。
  import { Button } from "#components/shadcn-svelte/button";
  import CardRow from "#components/common/card-row.svelte";
  import CardSection from "#components/common/card-section.svelte";
  import commands from "#libs/commands";
  import { m } from "#libs/i18n/paraglide/messages";
  import { toast } from "#libs/utils/toast";

  /** 命令在途时禁用三按钮，避免重复打开多个文件管理器窗口 */
  let busy = $state(false);

  /** 打开日志目录；失败 toast，成功无提示（文件管理器已弹出） */
  async function handleOpenLogDir(): Promise<void> {
    if (busy) return;
    busy = true;
    try {
      await commands
        .openLogDir()
        .failed(() => {
          toast.error(m.about_diagnostics_open_failed());
        })
        .result();
    } finally {
      busy = false;
    }
  }

  /** 打开配置目录；语义同上 */
  async function handleOpenConfigDir(): Promise<void> {
    if (busy) return;
    busy = true;
    try {
      await commands
        .openConfigDir()
        .failed(() => {
          toast.error(m.about_diagnostics_open_failed());
        })
        .result();
    } finally {
      busy = false;
    }
  }

  /** 复制系统信息；成功与失败均 toast（复制无可见落点，需明确反馈） */
  async function handleCopySystemInfo(): Promise<void> {
    if (busy) return;
    busy = true;
    try {
      const result = await commands
        .copySystemInfo()
        .failed(() => {
          toast.error(m.about_diagnostics_copy_failed());
        })
        .result();
      if (result.status === "ok") toast.success(m.about_diagnostics_copy_success());
    } finally {
      busy = false;
    }
  }
</script>

<CardSection title={m.about_diagnostics_title()} description={m.about_diagnostics_description()}>
  <CardRow
    label={m.about_diagnostics_log_label()}
    description={m.about_diagnostics_log_description()}
  >
    {#snippet control()}
      <Button variant="outline" disabled={busy} onclick={() => void handleOpenLogDir()}>
        {m.about_open()}
      </Button>
    {/snippet}
  </CardRow>

  <CardRow
    label={m.about_diagnostics_config_label()}
    description={m.about_diagnostics_config_description()}
  >
    {#snippet control()}
      <Button variant="outline" disabled={busy} onclick={() => void handleOpenConfigDir()}>
        {m.about_open()}
      </Button>
    {/snippet}
  </CardRow>

  <CardRow
    label={m.about_diagnostics_copy_label()}
    description={m.about_diagnostics_copy_description()}
  >
    {#snippet control()}
      <Button variant="outline" disabled={busy} onclick={() => void handleCopySystemInfo()}>
        {m.about_diagnostics_copy()}
      </Button>
    {/snippet}
  </CardRow>
</CardSection>
