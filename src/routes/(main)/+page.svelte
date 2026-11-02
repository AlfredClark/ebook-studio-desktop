<script lang="ts">
  // 首页英雄区：技术栈图标 + 应用名 + 项目简介，在内容区内水平垂直居中，无业务逻辑。
  import SvelteIcon from "#assets/icons/svelte-icon.svelte";
  import TauriIcon from "#assets/icons/tauri-icon.svelte";
  import ViteIcon from "#assets/icons/vite-icon.svelte";
  import { m } from "#libs/i18n/paraglide/messages";

  const appName = __APP_TAURI_CONF__.productName;

  /** 技术栈图标：内联组件随构建打包，不发静态图片请求；初始化时随模板品牌一并清除 */
  const stackIcons = [TauriIcon, SvelteIcon, ViteIcon];
</script>

<div class="flex h-full w-full flex-col items-center justify-center gap-6 p-6 text-center">
  <div class="flex items-center justify-center gap-6">
    {#each stackIcons as Icon (Icon)}
      <Icon />
    {/each}
  </div>
  <h1 class="text-2xl font-medium">{appName}</h1>
  <p class="max-w-md text-sm text-muted-foreground">{m.page_home_intro()}</p>
</div>
