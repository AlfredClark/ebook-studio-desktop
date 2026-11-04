<script lang="ts">
  // 右栏筛选器（哑组件）：顶部类型下拉固定，中间参数区按类型切换，
  // 底部筛选 + 清除贴底；脏态或空输入时筛选不可用。
  import { Button } from "#components/shadcn-svelte/button";
  import { Checkbox } from "#components/shadcn-svelte/checkbox";
  import { Input } from "#components/shadcn-svelte/input";
  import {
    Select,
    SelectContent,
    SelectItem,
    SelectTrigger,
    SelectValue,
  } from "#components/shadcn-svelte/select";
  import { m } from "#libs/i18n/paraglide/messages";

  let {
    kind = $bindable("keyword"),
    keyword = $bindable(""),
    caseSensitive = $bindable(false),
    pattern = $bindable(""),
    filtering = false,
    canFilter = false,
    hasOutcome = false,
    onRun,
    onClear,
  }: {
    kind: "keyword" | "regex";
    keyword: string;
    caseSensitive: boolean;
    pattern: string;
    filtering: boolean;
    canFilter: boolean;
    hasOutcome: boolean;
    onRun: () => void;
    onClear: () => void;
  } = $props();

  /** 类型下拉候选项：触发器关闭时靠 `items` 解析展示文案。 */
  const kindItems = [
    { value: "keyword", label: m.edit_filter_kind_keyword() },
    { value: "regex", label: m.edit_filter_kind_regex() },
  ];

  /** 下拉给字符串，经候选项收窄后才提交，无类型断言。 */
  function handleKindChange(value: string): void {
    if (value === "keyword" || value === "regex") {
      kind = value;
    }
  }
</script>

<section aria-label={m.edit_filter_title()} class="flex h-full min-h-0 flex-col gap-4 p-4">
  <h2 class="text-sm font-medium">{m.edit_filter_title()}</h2>
  <div class="flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto">
    <Select type="single" value={kind} items={kindItems} onValueChange={handleKindChange}>
      <SelectTrigger class="w-full" aria-label={m.edit_filter_kind_label()}>
        <SelectValue />
      </SelectTrigger>
      <SelectContent>
        <SelectItem value="keyword">{m.edit_filter_kind_keyword()}</SelectItem>
        <SelectItem value="regex">{m.edit_filter_kind_regex()}</SelectItem>
      </SelectContent>
    </Select>
    <div class="shrink-0 border-t"></div>
    {#if kind === "keyword"}
      <div class="flex flex-col gap-1.5">
        <label for="edit-filter-keyword" class="text-sm font-medium">
          {m.edit_filter_kind_keyword()}
        </label>
        <Input
          id="edit-filter-keyword"
          type="text"
          value={keyword}
          oninput={(event) => (keyword = event.currentTarget.value)}
          placeholder={m.edit_filter_keyword_placeholder()}
        />
        <label class="flex cursor-pointer items-center gap-2 text-sm text-muted-foreground">
          <Checkbox
            checked={caseSensitive}
            onCheckedChange={(value) => (caseSensitive = value === true)}
          />
          {m.edit_filter_case_sensitive()}
        </label>
      </div>
    {:else}
      <div class="flex flex-col gap-1.5">
        <label for="edit-filter-pattern" class="text-sm font-medium">
          {m.edit_filter_kind_regex()}
        </label>
        <Input
          id="edit-filter-pattern"
          type="text"
          value={pattern}
          oninput={(event) => (pattern = event.currentTarget.value)}
          placeholder={m.edit_filter_pattern_placeholder()}
        />
        <p class="text-xs text-muted-foreground">{m.edit_filter_pattern_hint()}</p>
      </div>
    {/if}
  </div>
  <div class="flex shrink-0 gap-2 border-t pt-3">
    <Button class="min-w-0 flex-1" size="sm" disabled={!canFilter} onclick={onRun}>
      {filtering ? m.edit_filter_running() : m.edit_filter_run()}
    </Button>
    <Button variant="outline" size="sm" disabled={!hasOutcome} onclick={onClear}>
      {m.edit_filter_clear()}
    </Button>
  </div>
</section>
