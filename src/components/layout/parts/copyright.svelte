<script lang="ts">
  import { m } from "#libs/i18n/paraglide/messages";
  import { cn } from "#libs/utils/shadcn-svelte";

  // 年份取运行时当前年，构建产物跨年也不会过期；作者以 package.json 为唯一来源
  const copyrightYear = new Date().getFullYear().toString();

  let { class: className = "" }: { class?: string } = $props();
</script>

<span class={cn("text-center", className)}>
  {m.footer_copyright({ year: copyrightYear, author: __APP_PKG__.author })}
</span>
