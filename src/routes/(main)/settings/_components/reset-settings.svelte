<script lang="ts">
  // 恢复默认设置：danger-zone 分组卡（标题 + 范围说明）+ 全宽 destructive 按钮 + 二次确认弹窗。
  // 卡片负责事前说清重置范围，弹窗负责确认这一次点击；语言回落走 alignLocale 原地生效而非整页重载
  // ——重载会吃掉成功 toast，界面语言也会变得毫无征兆。
  import { setMode } from "mode-watcher";
  import { resetAppearance } from "#libs/hooks/appearance.svelte";
  import { resetConfig } from "#libs/hooks/config.svelte";
  import CardSection from "#components/common/card-section.svelte";
  import ConfirmDialog from "#components/common/confirm-dialog.svelte";
  import { Button } from "#components/shadcn-svelte/button";
  import { alignLocale } from "#libs/i18n/locale.svelte";
  import { m } from "#libs/i18n/paraglide/messages";
  import { baseLocale, getLocale } from "#libs/i18n/paraglide/runtime";
  import { toast } from "#libs/utils/toast";

  /** 确认弹窗开关 */
  let open = $state(false);
  /** 重置在途时禁用按钮，避免重复提交 */
  let busy = $state(false);

  /** 后端重置会把 locale 拉回默认值（= Paraglide baseLocale），当前语言非默认时范围里必须提前讲清 */
  const resetsLocale = $derived(getLocale() !== baseLocale);

  /** 范围说明：仅当真的会连带换语言时换成含语言后果的那份；不做字符串拼接——空格与标点归各语言自己管 */
  const scope = $derived(
    resetsLocale
      ? m.settings_reset_description_locale({ language: m.settings_language_option_en() })
      : m.settings_reset_description(),
  );

  /** 确认恢复：弹窗已由通用组件关闭；前端三件套即时生效，后端失败仅 toast（前端已重置的不回滚） */
  async function handleConfirm(): Promise<void> {
    if (busy) return;
    busy = true;
    try {
      resetAppearance();
      setMode("system");
      const after = await resetConfig();
      if (after === null) {
        toast.error(m.settings_reset_failed());
        return;
      }
      // 先对齐语言再报成功：toast 与重置后的界面语言一致，且不被重载吞掉
      alignLocale(after.locale);
      toast.success(m.settings_reset_success());
    } finally {
      busy = false;
    }
  }
</script>

<CardSection title={m.settings_reset_title()} description={scope}>
  <div class="py-3">
    <Button variant="destructive" class="w-full" disabled={busy} onclick={() => (open = true)}>
      {m.settings_reset_button()}
    </Button>
  </div>
</CardSection>

<ConfirmDialog
  bind:open
  title={m.settings_reset_confirm_title()}
  description={m.settings_reset_confirm_description()}
  cancelLabel={m.settings_close_confirm_cancel()}
  confirmLabel={m.settings_reset_confirm_ok()}
  variant="destructive"
  onConfirm={() => void handleConfirm()}
/>
