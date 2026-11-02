<script lang="ts">
  // 字体选择器：系统字体下拉（搜索 + 懒加载 + 逐项字体预览），状态走外观 hook，对外零 props。
  // 插件仅提供候选列表，偏好本身存前端 localStorage；插件不可用时禁用并提示，不阻断页面。
  import { tick } from "svelte";
  import { getSystemFonts } from "tauri-plugin-system-fonts-api";
  import ChevronsUpDownIcon from "@lucide/svelte/icons/chevrons-up-down";
  import type { SystemFont } from "tauri-plugin-system-fonts-api";
  import {
    buildFontStack,
    DEFAULT_FONT_FAMILY,
    fontState,
    sanitizeFontFamily,
    setFontFamily,
  } from "#libs/hooks/appearance.svelte";
  import { Button } from "#components/shadcn-svelte/button";
  import {
    Input as CommandInput,
    Item as CommandItem,
    List as CommandList,
    Root as CommandRoot,
  } from "#components/shadcn-svelte/command";
  import {
    Content as PopoverContent,
    Root as PopoverRoot,
    Trigger as PopoverTrigger,
  } from "#components/shadcn-svelte/popover";
  import CardRow from "#components/common/card-row.svelte";
  import { m } from "#libs/i18n/paraglide/messages";

  /** 系统字体族候选：插件变体数组按族名去重排序，失败时为空并禁用下拉。 */
  let systemFontFamilies = $state<string[]>([]);
  let loadingFonts = $state(false);
  // 是否已拉取过：首次打开弹窗前不付全量 IPC，触发器保持可用
  let fontsLoaded = $state(false);

  /** 懒加载窗口：每页挂载数，搜索或打开弹窗时重置，避免几百项一次挂载。 */
  const FONT_LIST_PAGE_SIZE = 80;
  let fontPopoverOpen = $state(false);
  let fontQuery = $state("");
  let visibleFontCount = $state(FONT_LIST_PAGE_SIZE);
  let fontTriggerRef = $state<HTMLButtonElement | null>(null);
  let fontListRef: HTMLElement | null = $state(null);
  let fontSentinel: HTMLDivElement | null = $state(null);

  /** 下拉渲染顺序：已存取值即便被卸载仍保留一行，避免触发器闪空。 */
  const orderedFontFamilies = $derived(
    [fontState.family, ...systemFontFamilies]
      .filter((family) => family && family !== DEFAULT_FONT_FAMILY)
      .filter((family, index, all) => all.indexOf(family) === index)
      .sort((a, b) => a.localeCompare(b)),
  );

  // 查询串归一化一次，多族名复用，避免循环内重复计算
  const normalizedQuery = $derived(normalizeFontQuery(fontQuery));

  /** 搜索过滤：归一化后子串匹配族名，空格不影响查询。 */
  const filteredFontFamilies = $derived(
    orderedFontFamilies.filter((family) => normalizeFontQuery(family).includes(normalizedQuery)),
  );

  /** 懒加载当前页：常驻 DOM 封顶，触底哨兵再追加。 */
  const visibleFontFamilies = $derived(filteredFontFamilies.slice(0, visibleFontCount));

  /** 触发器展示文案：跟随已存取值，关闭态同样可解析。 */
  const selectedFontLabel = $derived(
    fontState.family === DEFAULT_FONT_FAMILY
      ? m.settings_font_family_option_system()
      : fontState.family,
  );

  // 插件不可用（浏览器预览等无 Tauri 环境）时回落空列表，不阻断页面；
  // 未拉取前按正常描述展示，拉取后为空才提示不可用
  const fontFamilyDescription = $derived(
    fontsLoaded && !loadingFonts && systemFontFamilies.length === 0
      ? m.settings_font_family_unavailable()
      : m.settings_font_family_description(),
  );

  /** 字体即时生效，仅前端 `localStorage` 持久化，不经过后端。 */
  // 选中后关闭弹窗并回焦触发器，方便继续键盘导航
  function handleFontFamilySelect(family: string): void {
    setFontFamily(family);
    fontPopoverOpen = false;
    tick().then(() => {
      fontTriggerRef?.focus();
    });
  }

  /** 归一化搜索串：去空白后小写，族名与关键词空格差异不影响匹配。 */
  function normalizeFontQuery(value: string): string {
    return value.replace(/\s+/g, "").toLowerCase();
  }

  /** 从插件变体数组提取族名：按 `name` 去重排序，族名先清洗再使用。 */
  function extractFontFamilies(fonts: SystemFont[]): string[] {
    const families: string[] = [];
    for (const font of fonts) {
      const family = sanitizeFontFamily(font.name || font.fontName);
      if (family && family !== DEFAULT_FONT_FAMILY && !families.includes(family)) {
        families.push(family);
      }
    }
    return families.sort((a, b) => a.localeCompare(b));
  }

  // 首次打开弹窗才拉取系统字体：进设置页不付全量 IPC；失败回落空列表并禁用
  function handleFontPopoverOpenChange(open: boolean): void {
    if (!open || fontsLoaded) {
      return;
    }
    fontsLoaded = true;
    loadingFonts = true;
    void (async () => {
      try {
        systemFontFamilies = extractFontFamilies(await getSystemFonts());
      } catch {
        systemFontFamilies = [];
      } finally {
        loadingFonts = false;
      }
    })();
  }

  // 搜索变化时重置窗口，避免过滤后出现空洞
  $effect(() => {
    if (fontQuery.length >= 0) {
      visibleFontCount = FONT_LIST_PAGE_SIZE;
    }
  });

  // 弹窗打开时重置搜索与窗口，每次都是全量首屏
  $effect(() => {
    if (fontPopoverOpen) {
      fontQuery = "";
      visibleFontCount = FONT_LIST_PAGE_SIZE;
    }
  });

  // 哨兵触底追加一页；弹窗关闭卸载后引用置空，观察自动清理
  $effect(() => {
    const sentinel = fontSentinel;
    const root = fontListRef;
    if (!fontPopoverOpen || !sentinel || !root) return;
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((entry) => entry.isIntersecting)) {
          visibleFontCount += FONT_LIST_PAGE_SIZE;
        }
      },
      { root },
    );
    observer.observe(sentinel);
    return () => observer.disconnect();
  });
</script>

<CardRow label={m.settings_font_family_label()} description={fontFamilyDescription}>
  {#snippet control()}
    <PopoverRoot bind:open={fontPopoverOpen} onOpenChange={handleFontPopoverOpenChange}>
      <PopoverTrigger bind:ref={fontTriggerRef}>
        {#snippet child({ props })}
          <Button
            {...props}
            variant="outline"
            role="combobox"
            aria-expanded={fontPopoverOpen}
            aria-label={m.settings_font_family_label()}
            disabled={loadingFonts || (fontsLoaded && systemFontFamilies.length === 0)}
            class="w-(--control-column-width) justify-between"
          >
            <span class="truncate">
              {loadingFonts ? m.settings_font_family_loading() : selectedFontLabel}
            </span>
            <ChevronsUpDownIcon class="opacity-50" />
          </Button>
        {/snippet}
      </PopoverTrigger>
      <PopoverContent class="max-w-xl min-w-48 p-0" align="end">
        <CommandRoot shouldFilter={false}>
          <CommandInput
            bind:value={fontQuery}
            placeholder={m.settings_font_family_search()}
            aria-label={m.settings_font_family_search()}
          />
          <CommandList bind:ref={fontListRef}>
            <CommandItem
              value={DEFAULT_FONT_FAMILY}
              onSelect={() => handleFontFamilySelect(DEFAULT_FONT_FAMILY)}
            >
              {m.settings_font_family_option_system()}
            </CommandItem>
            {#each visibleFontFamilies as family (family)}
              <CommandItem value={family} onSelect={() => handleFontFamilySelect(family)}>
                <span style="font-family: {buildFontStack(family)};">{family}</span>
              </CommandItem>
            {/each}
            {#if filteredFontFamilies.length === 0}
              <div class="px-2 py-6 text-center text-sm text-muted-foreground">
                {m.settings_font_family_empty()}
              </div>
            {:else if visibleFontFamilies.length < filteredFontFamilies.length}
              <div bind:this={fontSentinel} aria-hidden="true"></div>
            {/if}
          </CommandList>
        </CommandRoot>
      </PopoverContent>
    </PopoverRoot>
  {/snippet}
</CardRow>
