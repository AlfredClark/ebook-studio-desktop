<script lang="ts">
  // 应用图标：Ebook Studio 标识（epub 折角箭头的单色 currentColor 版），随构建打包；
  // 与 `src-tauri/app-icon.svg` 同图案（源自 epub.svg），明暗主题自适应，
  // 纯装饰，`aria-hidden` 不进入可访问名称。
  import { cn } from "#libs/utils/shadcn-svelte";

  let { class: className = "" }: { class?: string } = $props();
</script>

<svg
  xmlns="http://www.w3.org/2000/svg"
  viewBox="0 0 64 64"
  fill="none"
  aria-hidden="true"
  class={cn("size-5", className)}
>
  <svg x="9" y="9" width="46" height="46" viewBox="0 0 1024 1024">
    <path
      d="M512 793.664L230.464 512 512 230.464l93.952 93.824L418.176 512 512 605.888l281.6-281.6-245.312-245.248a51.2 51.2 0 0 0-72.512 0L79.04 475.776a51.2 51.2 0 0 0 0 72.512l396.736 396.736a51.2 51.2 0 0 0 72.512 0l396.736-396.736a51.392 51.392 0 0 0 0-72.512l-57.6-57.536z"
      fill="currentColor"
    />
  </svg>
</svg>
