<script lang="ts">
  // 裸项目路由守卫：无步骤段即默认第一步，重定向到元数据步骤，不留历史。
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import { resolve } from "$app/paths";

  onMount(() => {
    void goto(resolve(`/novel/${page.params.uuid}/metadata`), { replaceState: true });
  });
</script>
