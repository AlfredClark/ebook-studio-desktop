<script lang="ts">
  // 项目操作栏：单行从左到右为全选 + 搜索 + 排序 + 筛选 + 批量操作 + 新建；纯展示，状态与回调全由调用方传入。
  // 窄栏下横向滚动（滚动条隐藏），下拉内容走 body 移植不受裁剪影响。
  import PlusIcon from "@lucide/svelte/icons/plus";
  import RotateCwIcon from "@lucide/svelte/icons/rotate-cw";
  import SlidersHorizontalIcon from "@lucide/svelte/icons/sliders-horizontal";
  import { Button } from "#components/shadcn-svelte/button";
  import { Checkbox } from "#components/shadcn-svelte/checkbox";
  import * as DropdownMenu from "#components/shadcn-svelte/dropdown-menu";
  import { Input } from "#components/shadcn-svelte/input";
  import {
    Select,
    SelectContent,
    SelectItem,
    SelectTrigger,
    SelectValue,
  } from "#components/shadcn-svelte/select";
  import { m } from "#libs/i18n/paraglide/messages";
  import type { ProjectType } from "#libs/commands/types";
  import type { BatchAction, SortKey, TypeFilter } from "./project-data";

  let {
    query,
    sortKey,
    filter,
    hasChecked,
    refreshing,
    selectAllChecked,
    selectAllIndeterminate,
    selectAllDisabled,
    onQueryChange,
    onSortChange,
    onFilterChange,
    onToggleAll,
    onBatchAction,
    onRefresh,
    onCreate,
  }: {
    query: string;
    sortKey: SortKey;
    filter: TypeFilter;
    hasChecked: boolean;
    refreshing: boolean;
    selectAllChecked: boolean;
    selectAllIndeterminate: boolean;
    selectAllDisabled: boolean;
    onQueryChange: (value: string) => void;
    onSortChange: (value: SortKey) => void;
    onFilterChange: (value: TypeFilter) => void;
    onToggleAll: (checked: boolean) => void;
    onBatchAction: (action: BatchAction) => void;
    onRefresh: () => void;
    onCreate: (type: ProjectType) => void;
  } = $props();

  /** 拼音/注音组词中不提交搜索，组词结束才按最终值过滤，避免逐字跳动。 */
  let composing = $state(false);

  /** 输入法组词时只同步显示不派发，组词结束后按输入框现值提交一次。 */
  function handleSearchInput(value: string): void {
    if (!composing) {
      onQueryChange(value);
    }
  }

  /** 排序下拉候选项：`items` 让触发器在下拉关闭时也能解析出展示文案。 */
  const sortItems: { value: SortKey; label: string }[] = [
    { value: "updated_desc", label: m.projects_sort_updated() },
    { value: "updated_asc", label: m.projects_sort_updated_asc() },
    { value: "created_desc", label: m.projects_sort_created() },
    { value: "created_asc", label: m.projects_sort_created_asc() },
    { value: "name_asc", label: m.projects_sort_name() },
    { value: "name_desc", label: m.projects_sort_name_desc() },
  ];

  /** 分类筛选下拉候选项，同上。 */
  const filterItems: { value: TypeFilter; label: string }[] = [
    { value: "all", label: m.projects_filter_all() },
    { value: "novel", label: m.projects_type_novel() },
    { value: "comic", label: m.projects_type_comic() },
  ];

  /** Bits-ui 给 `string`，经候选表收窄为 `SortKey` 后才提交，无类型断言。 */
  function handleSortChange(value: string): void {
    const next = sortItems.find((item) => item.value === value)?.value;
    if (next) {
      onSortChange(next);
    }
  }

  /** 同上，收窄为 `TypeFilter`。 */
  function handleFilterChange(value: string): void {
    const next = filterItems.find((item) => item.value === value)?.value;
    if (next) {
      onFilterChange(next);
    }
  }
</script>

<div
  class="flex [scrollbar-width:none] items-center gap-2 overflow-x-auto [&::-webkit-scrollbar]:hidden"
>
  <Checkbox
    checked={selectAllChecked}
    indeterminate={selectAllIndeterminate}
    disabled={selectAllDisabled}
    onCheckedChange={(value) => onToggleAll(value === true)}
    aria-label={m.projects_toolbar_select_all()}
    class="shrink-0"
  />
  <Input
    value={query}
    oninput={(event) => handleSearchInput(event.currentTarget.value)}
    oncompositionstart={() => (composing = true)}
    oncompositionend={(event) => {
      composing = false;
      onQueryChange(event.currentTarget.value);
    }}
    type="search"
    placeholder={m.projects_toolbar_search_placeholder()}
    aria-label={m.projects_toolbar_search_label()}
    class="w-32 min-w-0 flex-1"
  />
  <Select type="single" value={sortKey} items={sortItems} onValueChange={handleSortChange}>
    <SelectTrigger class="w-32 shrink-0" aria-label={m.projects_toolbar_sort_label()}>
      <SelectValue />
    </SelectTrigger>
    <SelectContent>
      <SelectItem value="updated_desc">{m.projects_sort_updated()}</SelectItem>
      <SelectItem value="updated_asc">{m.projects_sort_updated_asc()}</SelectItem>
      <SelectItem value="created_desc">{m.projects_sort_created()}</SelectItem>
      <SelectItem value="created_asc">{m.projects_sort_created_asc()}</SelectItem>
      <SelectItem value="name_asc">{m.projects_sort_name()}</SelectItem>
      <SelectItem value="name_desc">{m.projects_sort_name_desc()}</SelectItem>
    </SelectContent>
  </Select>
  <Select type="single" value={filter} items={filterItems} onValueChange={handleFilterChange}>
    <SelectTrigger class="w-28 shrink-0" aria-label={m.projects_toolbar_filter_label()}>
      <SelectValue />
    </SelectTrigger>
    <SelectContent>
      <SelectItem value="all">{m.projects_filter_all()}</SelectItem>
      <SelectItem value="novel">{m.projects_type_novel()}</SelectItem>
      <SelectItem value="comic">{m.projects_type_comic()}</SelectItem>
    </SelectContent>
  </Select>
  <Button
    variant="ghost"
    size="icon-sm"
    class="shrink-0"
    disabled={refreshing}
    onclick={onRefresh}
    aria-label={m.projects_toolbar_refresh()}
  >
    <RotateCwIcon class={refreshing ? "animate-spin" : ""} />
  </Button>
  {#if hasChecked}
    <DropdownMenu.Root>
      <DropdownMenu.Trigger>
        {#snippet child({ props })}
          <Button
            {...props}
            variant="outline"
            size="sm"
            class="shrink-0"
            aria-label={m.projects_toolbar_batch()}
          >
            <SlidersHorizontalIcon />{m.projects_toolbar_batch()}
          </Button>
        {/snippet}
      </DropdownMenu.Trigger>
      <DropdownMenu.Content align="end">
        <DropdownMenu.Item variant="destructive" onSelect={() => onBatchAction("delete")}>
          {m.projects_batch_delete()}
        </DropdownMenu.Item>
      </DropdownMenu.Content>
    </DropdownMenu.Root>
  {/if}
  <DropdownMenu.Root>
    <DropdownMenu.Trigger>
      {#snippet child({ props })}
        <Button {...props} size="sm" class="shrink-0" aria-label={m.projects_toolbar_create()}>
          <PlusIcon />{m.projects_toolbar_create()}
        </Button>
      {/snippet}
    </DropdownMenu.Trigger>
    <DropdownMenu.Content align="end">
      <DropdownMenu.Item onSelect={() => onCreate("novel")}>
        {m.projects_type_novel()}
      </DropdownMenu.Item>
      <DropdownMenu.Item onSelect={() => onCreate("comic")}>
        {m.projects_type_comic()}
      </DropdownMenu.Item>
    </DropdownMenu.Content>
  </DropdownMenu.Root>
</div>
