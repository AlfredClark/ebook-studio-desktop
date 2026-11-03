<script lang="ts">
  // 元数据表单（novel/comic 共用）：纯展示 + 本地编辑态；加载/保存/选图由各步骤页负责。
  // 上中下三段：上（封面 | 基本信息 | 出版信息）/ 中（简介横贯）/ 下（重置保存操作条）。
  import CalendarIcon from "@lucide/svelte/icons/calendar";
  import PlusIcon from "@lucide/svelte/icons/plus";
  import XIcon from "@lucide/svelte/icons/x";
  import {
    DateFormatter,
    getLocalTimeZone,
    parseDate,
    type DateValue,
  } from "@internationalized/date";
  import { Button } from "#components/shadcn-svelte/button";
  import { Calendar } from "#components/shadcn-svelte/calendar";
  import { Input } from "#components/shadcn-svelte/input";
  import * as Popover from "#components/shadcn-svelte/popover";
  import { Textarea } from "#components/shadcn-svelte/textarea";
  import type { ProjectMetadata, ProjectType } from "#libs/commands/types";
  import { getLocale } from "#libs/i18n/paraglide/runtime";
  import { m } from "#libs/i18n/paraglide/messages";
  import { toast } from "#libs/utils/toast";
  import { cn } from "#libs/utils/shadcn-svelte";
  import { formatSubjectsInput, parseSubjectsInput } from "./metadata-form-data";

  let {
    metadata,
    coverUrl,
    projectType,
    saving = false,
    importing = false,
    onSave,
    onChooseCover,
    onReset,
  }: {
    metadata: ProjectMetadata;
    coverUrl: string | null;
    projectType: ProjectType;
    saving: boolean;
    importing: boolean;
    onSave: (metadata: ProjectMetadata) => void;
    onChooseCover: () => void;
    /** 重置按钮回调：由步骤页弹确认框，真重置走后端删文件重建。 */
    onReset: () => void;
  } = $props();

  let title = $state("");
  let alternative = $state("");
  let creator = $state("");
  let contributor = $state("");
  let language = $state("");
  let publisher = $state("");
  let description = $state("");
  let subjectsText = $state("");
  let datePublished = $state("");
  let rights = $state("");
  let series = $state("");
  let volume = $state("");
  let source = $state("");
  /** 日历弹窗开关：选中即关。 */
  let pickerOpen = $state(false);
  /** 日历选中值：与字符串日期双向同步（提交只用字符串）。 */
  let pickerValue = $state<DateValue | undefined>(undefined);

  /** 字符串转日历值：空或非法回落未选，不抛错。 */
  function toDateValue(text: string): DateValue | undefined {
    if (text === "") {
      return undefined;
    }
    try {
      return parseDate(text);
    } catch {
      return undefined;
    }
  }

  /** 按钮上的日期文案：按当前语言格式化，无值显示占位。 */
  const formattedDate = $derived.by(() => {
    const value = toDateValue(datePublished);
    if (value === undefined) {
      return "";
    }
    return new DateFormatter(getLocale(), { dateStyle: "medium" }).format(
      value.toDate(getLocalTimeZone()),
    );
  });

  /** 从传入元数据同步本地编辑态：挂载、后端保存返回、重置时共用。 */
  function syncFromMetadata(): void {
    title = metadata.title;
    alternative = metadata.alternative;
    creator = metadata.creator;
    contributor = metadata.contributor;
    language = metadata.language;
    publisher = metadata.publisher;
    description = metadata.description;
    subjectsText = formatSubjectsInput(metadata.subjects);
    datePublished = metadata.date_published ?? "";
    rights = metadata.rights;
    series = metadata.series;
    volume = metadata.volume;
    source = metadata.source;
    pickerValue = toDateValue(datePublished);
  }

  // 后端保存返回新对象时同步回表单（字段值与提交一致，仅更新时间变化）。
  $effect(() => {
    syncFromMetadata();
  });

  /** 标题去空后非空且非保存中才可提交；其余校验以后端为准。 */
  const canSave = $derived(title.trim() !== "" && !saving);

  /** 组装提交对象：必填仅书名（模板与 EPUB3 的硬需，其余皆有后端默认）。 */
  function handleSubmit(event: SubmitEvent): void {
    event.preventDefault();
    if (title.trim() === "") {
      toast.error(m.metadata_title_required());
      return;
    }
    if (saving) {
      return;
    }
    onSave({
      ...metadata,
      title: title.trim(),
      alternative: alternative.trim(),
      creator: creator.trim(),
      contributor: contributor.trim(),
      language: projectType === "novel" ? "zh-CN" : language.trim(),
      publisher: publisher.trim(),
      description: description.trim(),
      subjects: parseSubjectsInput(subjectsText),
      date_published: datePublished === "" ? null : datePublished,
      rights: rights.trim(),
      series: series.trim(),
      volume: volume.trim(),
      source: source.trim(),
    });
  }
</script>

<form class="flex h-full min-h-0 w-full flex-col gap-4 p-6" onsubmit={handleSubmit}>
  <div class="grid min-h-0 grid-cols-1 gap-6 lg:grid-cols-[180px_minmax(0,1fr)_minmax(0,1fr)]">
    <section class="flex min-h-0 flex-col gap-3">
      <h2 class="text-sm font-semibold">{m.metadata_cover_section()}</h2>
      <button
        type="button"
        disabled={importing}
        onclick={onChooseCover}
        aria-label={coverUrl !== null ? m.metadata_cover_change() : m.metadata_cover_choose()}
        class="relative flex aspect-[2/3] w-full items-center justify-center overflow-hidden rounded-md border-2 border-dashed border-muted-foreground/40 bg-muted/30 transition-colors hover:border-muted-foreground/70 disabled:cursor-wait disabled:opacity-60"
      >
        {#if coverUrl !== null}
          <img
            src={coverUrl}
            alt={m.metadata_cover_preview_alt()}
            class="h-full w-full object-cover"
          />
          <span
            class="absolute inset-x-0 bottom-0 bg-black/55 px-2 py-1 text-center text-[11px] text-white"
          >
            {m.metadata_cover_hint()}
          </span>
        {:else}
          <span class="flex flex-col items-center gap-2 p-3 text-center">
            <PlusIcon class="size-10 text-muted-foreground" />
            <span class="text-[11px] text-muted-foreground">{m.metadata_cover_hint()}</span>
          </span>
        {/if}
      </button>
      <div class="mt-auto">
        <Button
          type="button"
          variant="outline"
          disabled={importing}
          onclick={onChooseCover}
          class="w-full"
        >
          {importing
            ? m.metadata_cover_importing()
            : coverUrl !== null
              ? m.metadata_cover_change()
              : m.metadata_cover_choose()}
        </Button>
      </div>
    </section>

    <section class="flex min-h-0 flex-col gap-3">
      <h2 class="text-sm font-semibold">{m.metadata_basic_section()}</h2>
      <div class="flex flex-col gap-1.5">
        <label for="metadata-identifier" class="text-sm font-medium"
          >{m.metadata_identifier_label()}</label
        >
        <Input id="metadata-identifier" value={metadata.identifier} readonly disabled />
      </div>
      <div class="flex flex-col gap-1.5">
        <label for="metadata-title" class="text-sm font-medium">{m.metadata_title_label()}</label>
        <Input
          id="metadata-title"
          bind:value={title}
          type="text"
          maxlength={200}
          autocomplete="off"
        />
      </div>
      <div class="flex flex-col gap-1.5">
        <label for="metadata-alternative" class="text-sm font-medium"
          >{m.metadata_alternative_label()}</label
        >
        <Input
          id="metadata-alternative"
          bind:value={alternative}
          type="text"
          maxlength={200}
          autocomplete="off"
        />
      </div>
      <div class="grid grid-cols-1 gap-3 sm:grid-cols-2">
        <div class="flex flex-col gap-1.5">
          <label for="metadata-creator" class="text-sm font-medium"
            >{m.metadata_creator_label()}</label
          >
          <Input
            id="metadata-creator"
            bind:value={creator}
            type="text"
            maxlength={100}
            autocomplete="off"
          />
        </div>
        <div class="flex flex-col gap-1.5">
          <label for="metadata-contributor" class="text-sm font-medium"
            >{m.metadata_contributor_label()}</label
          >
          <Input
            id="metadata-contributor"
            bind:value={contributor}
            type="text"
            maxlength={100}
            autocomplete="off"
          />
        </div>
      </div>
      <div class="flex flex-col gap-1.5">
        <label for="metadata-language" class="text-sm font-medium"
          >{m.metadata_language_label()}</label
        >
        {#if projectType === "novel"}
          <Input id="metadata-language" value="zh-CN" disabled />
        {:else}
          <Input
            id="metadata-language"
            bind:value={language}
            type="text"
            maxlength={35}
            placeholder="zh-CN"
            autocomplete="off"
          />
        {/if}
      </div>
    </section>

    <section class="flex min-h-0 flex-col gap-3">
      <h2 class="text-sm font-semibold">{m.metadata_publish_section()}</h2>
      <div class="grid grid-cols-1 gap-3 sm:grid-cols-2">
        <div class="flex flex-col gap-1.5">
          <label for="metadata-publisher" class="text-sm font-medium"
            >{m.metadata_publisher_label()}</label
          >
          <Input
            id="metadata-publisher"
            bind:value={publisher}
            type="text"
            maxlength={100}
            autocomplete="off"
          />
        </div>
        <div class="flex flex-col gap-1.5">
          <label for="metadata-date" class="text-sm font-medium"
            >{m.metadata_date_published_label()}</label
          >
          <div class="flex gap-1.5">
            <Popover.Root bind:open={pickerOpen}>
              <Popover.Trigger id="metadata-date" class="min-w-0 flex-1">
                {#snippet child({ props })}
                  <Button
                    {...props}
                    variant="outline"
                    class={cn(
                      "w-full justify-start text-left font-normal",
                      datePublished === "" && "text-muted-foreground",
                    )}
                  >
                    <CalendarIcon class="me-2 size-4" />
                    {datePublished === "" ? m.metadata_date_placeholder() : formattedDate}
                  </Button>
                {/snippet}
              </Popover.Trigger>
              <Popover.Content class="w-auto p-0" align="start">
                <Calendar
                  type="single"
                  bind:value={pickerValue}
                  captionLayout="dropdown"
                  initialFocus
                  onValueChange={(value) => {
                    datePublished = value === undefined ? "" : value.toString();
                    pickerOpen = false;
                  }}
                />
              </Popover.Content>
            </Popover.Root>
            {#if datePublished !== ""}
              <Button
                type="button"
                variant="ghost"
                size="icon"
                aria-label={m.metadata_date_clear()}
                onclick={() => {
                  datePublished = "";
                  pickerValue = undefined;
                }}
              >
                <XIcon class="size-4" />
              </Button>
            {/if}
          </div>
        </div>
      </div>
      <div class="grid grid-cols-1 gap-3 sm:grid-cols-2">
        <div class="flex flex-col gap-1.5">
          <label for="metadata-series" class="text-sm font-medium"
            >{m.metadata_series_label()}</label
          >
          <Input
            id="metadata-series"
            bind:value={series}
            type="text"
            maxlength={100}
            autocomplete="off"
          />
        </div>
        <div class="flex flex-col gap-1.5">
          <label for="metadata-volume" class="text-sm font-medium"
            >{m.metadata_volume_label()}</label
          >
          <Input
            id="metadata-volume"
            bind:value={volume}
            type="text"
            maxlength={50}
            autocomplete="off"
          />
        </div>
      </div>
      <div class="flex flex-col gap-1.5">
        <label for="metadata-rights" class="text-sm font-medium">{m.metadata_rights_label()}</label>
        <Input
          id="metadata-rights"
          bind:value={rights}
          type="text"
          maxlength={500}
          autocomplete="off"
        />
      </div>
      <div class="flex flex-col gap-1.5">
        <label for="metadata-source" class="text-sm font-medium">{m.metadata_source_label()}</label>
        <Input
          id="metadata-source"
          bind:value={source}
          type="text"
          maxlength={500}
          autocomplete="off"
        />
      </div>
      <div class="flex flex-col gap-1.5">
        <label for="metadata-subjects" class="text-sm font-medium"
          >{m.metadata_subjects_label()}</label
        >
        <Input
          id="metadata-subjects"
          bind:value={subjectsText}
          type="text"
          maxlength={1000}
          placeholder={m.metadata_subjects_placeholder()}
          autocomplete="off"
        />
      </div>
    </section>
  </div>

  <div class="flex min-h-0 flex-1 flex-col gap-1.5">
    <label for="metadata-description" class="text-sm font-medium"
      >{m.metadata_description_label()}</label
    >
    <Textarea
      id="metadata-description"
      bind:value={description}
      maxlength={5000}
      class="min-h-0 flex-1 resize-none"
    />
  </div>

  <div class="grid grid-cols-2 gap-2">
    <Button type="button" variant="outline" disabled={saving} onclick={onReset}>
      {m.metadata_reset()}
    </Button>
    <Button type="submit" disabled={!canSave}>
      {saving ? m.metadata_saving() : m.metadata_save()}
    </Button>
  </div>
</form>
