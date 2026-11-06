<script lang="ts">
  // 漫画导出步骤页：待接入前先按解析产物引导（无产物回文件步骤，有产物提示待接入）。
  import { page } from "$app/state";
  import { m } from "#libs/i18n/paraglide/messages";
  import ComicStepGuide from "../../_components/comic-step-guide.svelte";

  const uuid = $derived(page.params.uuid ?? "");
</script>

<h1 class="sr-only">{m.project_step_export()}</h1>
<ComicStepGuide {uuid} />
