<script lang="ts">
  // 右栏筛选器（哑组件）：顶部类型下拉固定，中间参数区按类型切换，
  // 底部筛选 + 清除贴底；脏态或空输入时筛选不可用。
  import { Button } from "#components/shadcn-svelte/button";
  import { Checkbox } from "#components/shadcn-svelte/checkbox";
  import { Input } from "#components/shadcn-svelte/input";
  import type { CleanupPreset, PairKind, PunctPreset } from "#libs/commands/types";
  import {
    Root as RadioGroupRoot,
    Item as RadioGroupItem,
  } from "#components/shadcn-svelte/radio-group";
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
    cleanupPresets = $bindable([]),
    punctPresets = $bindable([]),
    pairScope = $bindable("Chapter"),
    pairPresets = $bindable([]),
    filtering = false,
    canFilter = false,
    hasOutcome = false,
    onRun,
    onClear,
  }: {
    kind: "keyword" | "regex" | "cleanup" | "punct" | "pairs";
    keyword: string;
    caseSensitive: boolean;
    pattern: string;
    cleanupPresets: CleanupPreset[];
    punctPresets: PunctPreset[];
    pairScope: "Line" | "Chapter";
    pairPresets: PairKind[];
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
    { value: "cleanup", label: m.edit_filter_kind_cleanup() },
    { value: "punct", label: m.edit_filter_kind_punct() },
    { value: "pairs", label: m.edit_filter_kind_pairs() },
  ];

  /** 清理分组：组头复用现有四字标签，叶子固定顺序排布。 */
  const cleanupGroups: { header: string; leaves: { value: CleanupPreset; label: string }[] }[] = [
    {
      header: m.edit_cleanup_preset_url(),
      leaves: [
        { value: "UrlFull", label: m.edit_cleanup_url_full() },
        { value: "UrlDomain", label: m.edit_cleanup_url_domain() },
      ],
    },
    {
      header: m.edit_cleanup_preset_promo(),
      leaves: [
        { value: "PromoRelease", label: m.edit_cleanup_promo_release() },
        { value: "PromoSite", label: m.edit_cleanup_promo_site() },
        { value: "PromoVote", label: m.edit_cleanup_promo_vote() },
        { value: "PromoGroup", label: m.edit_cleanup_promo_group() },
      ],
    },
    {
      header: m.edit_cleanup_preset_contact(),
      leaves: [
        { value: "ContactQq", label: m.edit_cleanup_contact_qq() },
        { value: "ContactWechat", label: m.edit_cleanup_contact_wechat() },
        { value: "ContactGroup", label: m.edit_cleanup_contact_group() },
      ],
    },
  ];

  /** 清理叶子的全局固定顺序：规格按此拼装，对比 `stale` 时不因勾选顺序抖动。 */
  const cleanupLeafOrder: CleanupPreset[] = cleanupGroups.flatMap((group) =>
    group.leaves.map((leaf) => leaf.value),
  );

  /** 标点分组：组头复用四字标签思路（宽度规范/隐形字符/异形连标），叶子固定顺序排布。 */
  const punctGroups: { header: string; leaves: { value: PunctPreset; label: string }[] }[] = [
    {
      header: m.edit_punct_group_width(),
      leaves: [
        { value: "HalfPunct", label: m.edit_punct_half() },
        { value: "FullSymbol", label: m.edit_punct_full() },
      ],
    },
    {
      header: m.edit_punct_group_hidden(),
      leaves: [
        { value: "ZeroWidth", label: m.edit_punct_zero() },
        { value: "WeirdSpace", label: m.edit_punct_space() },
      ],
    },
    {
      header: m.edit_punct_group_special(),
      leaves: [
        { value: "SpecialSym", label: m.edit_punct_symbol() },
        { value: "RepeatPunct", label: m.edit_punct_repeat() },
      ],
    },
  ];

  /** 标点叶子的全局固定顺序：规格按此拼装，对比 `stale` 时不因勾选顺序抖动。 */
  const punctLeafOrder: PunctPreset[] = punctGroups.flatMap((group) =>
    group.leaves.map((leaf) => leaf.value),
  );

  /** 成对分组：组头复用四字标签思路，叶子固定顺序排布。 */
  const pairGroups: { header: string; leaves: { value: PairKind; label: string }[] }[] = [
    {
      header: m.edit_pairs_group_round(),
      leaves: [
        { value: "Round", label: m.edit_pairs_round() },
        { value: "Square", label: m.edit_pairs_square() },
        { value: "Brace", label: m.edit_pairs_brace() },
      ],
    },
    {
      header: m.edit_pairs_group_special(),
      leaves: [
        { value: "TortBlack", label: m.edit_pairs_tort_black() },
        { value: "TortWhite", label: m.edit_pairs_tort_white() },
      ],
    },
    {
      header: m.edit_pairs_group_book(),
      leaves: [
        { value: "BookDouble", label: m.edit_pairs_book_double() },
        { value: "BookSingle", label: m.edit_pairs_book_single() },
      ],
    },
    {
      header: m.edit_pairs_group_quote(),
      leaves: [
        { value: "DQuote", label: m.edit_pairs_dquote() },
        { value: "SQuote", label: m.edit_pairs_squote() },
        { value: "CQuoteS", label: m.edit_pairs_cquote_s() },
        { value: "CQuoteD", label: m.edit_pairs_cquote_d() },
      ],
    },
    {
      header: m.edit_pairs_group_ascii(),
      leaves: [
        { value: "AsciiS", label: m.edit_pairs_ascii_s() },
        { value: "AsciiD", label: m.edit_pairs_ascii_d() },
        { value: "RevQuote", label: m.edit_pairs_rev() },
      ],
    },
  ];

  /** 成对叶子的全局固定顺序：规格按此拼装，对比 `stale` 时不因勾选顺序抖动。 */
  const pairLeafOrder: PairKind[] = pairGroups.flatMap((group) =>
    group.leaves.map((leaf) => leaf.value),
  );

  /** 下拉给字符串，经候选项收窄后才提交，无类型断言。 */
  function handleKindChange(value: string): void {
    if (
      value === "keyword" ||
      value === "regex" ||
      value === "cleanup" ||
      value === "punct" ||
      value === "pairs"
    ) {
      kind = value;
    }
  }

  /** 范围单选：bits-ui 给字符串，经候选项收窄为后端枚举名后提交。 */
  function handleScopeChange(value: string): void {
    if (value === "Line" || value === "Chapter") {
      pairScope = value;
    }
  }

  /** 叶子勾选：增删后按全局固定顺序归一，避免规格顺序抖动。 */
  function toggleLeaf<T>(current: T[], order: T[], item: T, checked: boolean): T[] {
    return checked
      ? order.filter((value) => value === item || current.includes(value))
      : current.filter((value) => value !== item);
  }

  /** 组头切换：全开态点一下全关，其余点一下全开，始终按全局固定顺序归一。 */
  function toggleGroup<T>(current: T[], order: T[], values: T[], checked: boolean): T[] {
    if (checked) {
      const rest = current.filter((item) => !values.includes(item));
      return order.filter((item) => values.includes(item) || rest.includes(item));
    }
    return current.filter((item) => !values.includes(item));
  }

  /** 组半选：部分叶子开，`indeterminate` 与 `checked` 是两个独立 prop。 */
  function isGroupIndeterminate<T>(current: T[], values: T[]): boolean {
    const on = values.filter((value) => current.includes(value));
    return on.length > 0 && on.length < values.length;
  }

  /** 组全选：叶子全开。半选时 `checked` 取假，半选态由 `indeterminate` 独立表达。 */
  function isGroupChecked<T>(current: T[], values: T[]): boolean {
    return values.length > 0 && values.every((value) => current.includes(value));
  }

  /** 清理叶子勾选：增删后按全局固定顺序归一。 */
  function handleCleanupLeafChange(preset: CleanupPreset, checked: boolean): void {
    cleanupPresets = toggleLeaf(cleanupPresets, cleanupLeafOrder, preset, checked);
  }

  /** 清理组头切换：全开态点一下全关，其余点一下全开。 */
  function handleCleanupGroupChange(values: CleanupPreset[], checked: boolean): void {
    cleanupPresets = toggleGroup(cleanupPresets, cleanupLeafOrder, values, checked);
  }

  /** 标点叶子勾选：增删后按全局固定顺序归一。 */
  function handlePunctLeafChange(preset: PunctPreset, checked: boolean): void {
    punctPresets = toggleLeaf(punctPresets, punctLeafOrder, preset, checked);
  }

  /** 标点组头切换：全开态点一下全关，其余点一下全开。 */
  function handlePunctGroupChange(values: PunctPreset[], checked: boolean): void {
    punctPresets = toggleGroup(punctPresets, punctLeafOrder, values, checked);
  }

  /** 成对叶子勾选：增删后按全局固定顺序归一。 */
  function handlePairLeafChange(preset: PairKind, checked: boolean): void {
    pairPresets = toggleLeaf(pairPresets, pairLeafOrder, preset, checked);
  }

  /** 成对组头切换：全开态点一下全关，其余点一下全开。 */
  function handlePairGroupChange(values: PairKind[], checked: boolean): void {
    pairPresets = toggleGroup(pairPresets, pairLeafOrder, values, checked);
  }
</script>

<section aria-label={m.edit_filter_title()} class="flex h-full min-h-0 flex-col gap-4 p-4">
  <h2 class="text-sm font-medium">{m.edit_filter_title()}</h2>
  <Select type="single" value={kind} items={kindItems} onValueChange={handleKindChange}>
    <SelectTrigger class="w-full shrink-0" aria-label={m.edit_filter_kind_label()}>
      <SelectValue />
    </SelectTrigger>
    <SelectContent>
      {#each kindItems as item (item.value)}
        <SelectItem value={item.value}>{item.label}</SelectItem>
      {/each}
    </SelectContent>
  </Select>
  <div class="shrink-0 border-t"></div>
  <div class="flex min-h-0 min-w-0 flex-1 flex-col gap-4 overflow-x-hidden overflow-y-auto">
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
    {:else if kind === "cleanup"}
      <div class="flex flex-col gap-3" role="group" aria-label={m.edit_filter_kind_cleanup()}>
        {#each cleanupGroups as group (group.header)}
          {@const values = group.leaves.map((leaf) => leaf.value)}
          <div class="flex flex-col gap-1.5">
            <label class="flex cursor-pointer items-center gap-2 text-sm font-medium">
              <Checkbox
                checked={isGroupChecked(cleanupPresets, values)}
                indeterminate={isGroupIndeterminate(cleanupPresets, values)}
                onCheckedChange={(value) => handleCleanupGroupChange(values, value === true)}
              />
              {group.header}
            </label>
            <div class="flex flex-col gap-1.5 pl-6">
              {#each group.leaves as leaf (leaf.value)}
                <label class="flex cursor-pointer items-center gap-2 text-sm text-muted-foreground">
                  <Checkbox
                    checked={cleanupPresets.includes(leaf.value)}
                    onCheckedChange={(value) => handleCleanupLeafChange(leaf.value, value === true)}
                  />
                  {leaf.label}
                </label>
              {/each}
            </div>
          </div>
        {/each}
      </div>
    {:else if kind === "punct"}
      <div class="flex flex-col gap-3" role="group" aria-label={m.edit_filter_kind_punct()}>
        {#each punctGroups as group (group.header)}
          {@const values = group.leaves.map((leaf) => leaf.value)}
          <div class="flex flex-col gap-1.5">
            <label class="flex cursor-pointer items-center gap-2 text-sm font-medium">
              <Checkbox
                checked={isGroupChecked(punctPresets, values)}
                indeterminate={isGroupIndeterminate(punctPresets, values)}
                onCheckedChange={(value) => handlePunctGroupChange(values, value === true)}
              />
              {group.header}
            </label>
            <div class="flex flex-col gap-1.5 pl-6">
              {#each group.leaves as leaf (leaf.value)}
                <label class="flex cursor-pointer items-center gap-2 text-sm text-muted-foreground">
                  <Checkbox
                    checked={punctPresets.includes(leaf.value)}
                    onCheckedChange={(value) => handlePunctLeafChange(leaf.value, value === true)}
                  />
                  {leaf.label}
                </label>
              {/each}
            </div>
          </div>
        {/each}
      </div>
    {:else if kind === "pairs"}
      <div class="flex flex-col gap-1.5">
        <p id="edit-pairs-scope-label" class="text-sm font-medium">
          {m.edit_pairs_scope_label()}
        </p>
        <RadioGroupRoot
          value={pairScope}
          onValueChange={handleScopeChange}
          class="flex flex-col gap-1.5"
          aria-labelledby="edit-pairs-scope-label"
        >
          <div class="flex items-center gap-2">
            <RadioGroupItem value="Line" aria-labelledby="edit-pairs-scope-line-label" />
            <span id="edit-pairs-scope-line-label" class="text-sm text-muted-foreground">
              {m.edit_pairs_scope_line()}
            </span>
          </div>
          <div class="flex items-center gap-2">
            <RadioGroupItem value="Chapter" aria-labelledby="edit-pairs-scope-chapter-label" />
            <span id="edit-pairs-scope-chapter-label" class="text-sm text-muted-foreground">
              {m.edit_pairs_scope_chapter()}
            </span>
          </div>
        </RadioGroupRoot>
      </div>
      <div class="flex flex-col gap-3" role="group" aria-label={m.edit_filter_kind_pairs()}>
        {#each pairGroups as group (group.header)}
          {@const values = group.leaves.map((leaf) => leaf.value)}
          <div class="flex flex-col gap-1.5">
            <label class="flex cursor-pointer items-center gap-2 text-sm font-medium">
              <Checkbox
                checked={isGroupChecked(pairPresets, values)}
                indeterminate={isGroupIndeterminate(pairPresets, values)}
                onCheckedChange={(value) => handlePairGroupChange(values, value === true)}
              />
              {group.header}
            </label>
            <div class="flex flex-col gap-1.5 pl-6">
              {#each group.leaves as leaf (leaf.value)}
                <label class="flex cursor-pointer items-center gap-2 text-sm text-muted-foreground">
                  <Checkbox
                    checked={pairPresets.includes(leaf.value)}
                    onCheckedChange={(value) => handlePairLeafChange(leaf.value, value === true)}
                  />
                  {leaf.label}
                </label>
              {/each}
            </div>
          </div>
        {/each}
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
