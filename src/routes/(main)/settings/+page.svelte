<script lang="ts">
  // 设置页组装：滚动壳 + 标题 + 通用/外观/快捷键三分组 + 底部恢复默认按钮，无业务逻辑。
  import { ScrollArea } from "#components/shadcn-svelte/scroll-area";
  import { m } from "#libs/i18n/paraglide/messages";
  import AppearanceSettings from "./_components/appearance-settings.svelte";
  import GeneralSettings from "./_components/general-settings.svelte";
  import ResetSettings from "./_components/reset-settings.svelte";
  import ShortcutSettings from "./_components/shortcut-settings.svelte";
</script>

<!-- 页面级纵向滚动：内容超高时在视口内滚动；内层保持水平居中、垂直置顶 -->
<ScrollArea class="h-full w-full">
  <div class="mx-auto flex w-full max-w-2xl flex-col gap-6 px-4 py-6">
    <h1 class="text-2xl font-medium">{m.page_settings_title()}</h1>

    <GeneralSettings />
    <AppearanceSettings />
    <ShortcutSettings />
    <ResetSettings />
  </div>
</ScrollArea>
