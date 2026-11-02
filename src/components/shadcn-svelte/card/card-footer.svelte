<script lang="ts">
  import type { WithElementRef } from "#libs/utils/shadcn-svelte.js";
  import type { HTMLAttributes } from "svelte/elements";
  import { cn } from "#libs/utils/shadcn-svelte.js";

  let {
    ref = $bindable(null),
    class: className,
    children,
    ...restProps
  }: WithElementRef<HTMLAttributes<HTMLDivElement>> = $props();
</script>

<div
  bind:this={ref}
  data-slot="card-footer"
  class={cn("flex items-center rounded-b-xl border-t bg-muted/50 p-(--card-spacing)", className)}
  {...restProps}
>
  {@render children?.()}
</div>
