<script lang="ts">
  // 平台信息分组：挂载即拉系统信息，失败时在分组内保留**常驻**错误态；零 props。
  import type { SystemInfo } from "#libs/commands/types";
  import CardRow from "#components/common/card-row.svelte";
  import CardSection from "#components/common/card-section.svelte";
  import commands from "#libs/commands";
  import { reportCommandFailure } from "#libs/commands/cores";
  import { m } from "#libs/i18n/paraglide/messages";
  import { toast } from "#libs/utils/toast";

  /** 运行平台信息；加载失败留空，行内回落占位 */
  let sysInfo = $state<SystemInfo | null>(null);
  /** 拉取失败标记：toast 几秒后自行消失，不能作为失败的唯一信号，故在分组内留常驻态 */
  let failed = $state(false);

  // 挂载即拉一次平台信息，失败 toast 并在行内转为常驻错误态
  $effect(() => {
    void commands
      .getSystemInfo()
      .success((info) => {
        sysInfo = info;
      })
      .failed((failure) => {
        failed = true;
        reportCommandFailure("[about] failed to load system info", failure);
        toast.error(m.about_system_failed());
      });
  });
</script>

<CardSection title={m.about_platform_info_title()}>
  {#if failed}
    <!-- 常驻错误态：四行"…"占位看着像还在加载，会把"失败"读成"未完成" -->
    <p class="py-3 text-sm text-destructive">{m.about_system_failed()}</p>
  {:else}
    <CardRow label={m.about_platform()} value={sysInfo?.platform ?? "…"} />

    <CardRow label={m.about_os_version()} value={sysInfo?.os_version ?? "…"} />

    <CardRow label={m.about_arch()} value={sysInfo?.arch ?? "…"} />

    <CardRow label={m.about_hostname()} value={sysInfo?.hostname ?? "…"} />
  {/if}
</CardSection>
