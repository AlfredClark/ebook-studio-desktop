<script lang="ts">
  // 分卷分章参数卡（哑组件）：两个正则输入框，值由步骤页持有（bind），本步不做编译校验。
  import { Input } from "#components/shadcn-svelte/input";
  import CardSection from "#components/common/card-section.svelte";
  import { m } from "#libs/i18n/paraglide/messages";
  import { isValidPattern } from "./novel-text-params";

  let {
    volumePattern = $bindable(),
    chapterPattern = $bindable(),
  }: {
    volumePattern: string;
    chapterPattern: string;
  } = $props();

  const volumeValid = $derived(isValidPattern(volumePattern));
  const chapterValid = $derived(isValidPattern(chapterPattern));
</script>

<CardSection title={m.files_params_section()} description={m.files_params_description()}>
  <div class="flex flex-col gap-1.5 py-3">
    <label for="novel-volume-pattern" class="text-sm font-medium">{m.files_volume_label()}</label>
    <Input
      id="novel-volume-pattern"
      bind:value={volumePattern}
      type="text"
      maxlength={500}
      autocomplete="off"
      spellcheck={false}
    />
    {#if !volumeValid}
      <p class="text-sm text-destructive">{m.files_pattern_invalid()}</p>
    {/if}
  </div>
  <div class="flex flex-col gap-1.5 py-3">
    <label for="novel-chapter-pattern" class="text-sm font-medium">{m.files_chapter_label()}</label>
    <Input
      id="novel-chapter-pattern"
      bind:value={chapterPattern}
      type="text"
      maxlength={500}
      autocomplete="off"
      spellcheck={false}
    />
    {#if !chapterValid}
      <p class="text-sm text-destructive">{m.files_pattern_invalid()}</p>
    {/if}
  </div>
  <p class="py-3 text-sm break-words text-muted-foreground">{m.files_params_hint()}</p>
</CardSection>
