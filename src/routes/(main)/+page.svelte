<script lang="ts">
  // 首页英雄区：应用图标 + 应用名 + 项目简介，在内容区内水平垂直居中，无业务逻辑。
  import AppIcon from "#assets/icons/app-icon.svelte";
  import { m } from "#libs/i18n/paraglide/messages";

  const appName = __APP_TAURI_CONF__.productName;
</script>

<div class="flex h-full w-full flex-col items-center justify-center gap-6 p-6 text-center">
  <AppIcon class="size-12" />
  <h1 class="text-2xl font-medium">{appName}</h1>
  <p class="max-w-md text-sm text-muted-foreground">{m.page_home_intro()}</p>
</div>
