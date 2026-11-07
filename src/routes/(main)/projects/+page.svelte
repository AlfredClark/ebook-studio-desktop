<script lang="ts">
  // 项目页：左右分栏 + 操作栏/列表/详情组装；项目数据来自后端命令，进页拉取一次，
  // 创建/删除用命令返回值直接改本地态，页内不再重拉。
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { resolve } from "$app/paths";
  import { Button } from "#components/shadcn-svelte/button";
  import * as Resizable from "#components/shadcn-svelte/resizable";
  import { Skeleton } from "#components/shadcn-svelte/skeleton";
  import ConfirmDialog from "#components/common/confirm-dialog.svelte";
  import commands from "#libs/commands";
  import type { ProjectInfo, ProjectType } from "#libs/commands/types";
  import { m } from "#libs/i18n/paraglide/messages";
  import { getLocale } from "#libs/i18n/paraglide/runtime";
  import { storeProjectEntry } from "#libs/navigation/project-entry";
  import { toast } from "#libs/utils/toast";
  import {
    computeSelectAllState,
    filterAndSortProjects,
    removeProjects,
    toggleAllChecked,
    toggleCheckedId,
    type BatchAction,
    type SortKey,
    type TypeFilter,
  } from "./_components/project-data";
  import CreateProjectDialog from "#components/common/create-project-dialog.svelte";
  import ProjectDetail from "./_components/project-detail.svelte";
  import ProjectsList from "./_components/projects-list.svelte";
  import ProjectsToolbar from "./_components/projects-toolbar.svelte";

  let projects = $state<ProjectInfo[]>([]);
  let activeId = $state<string | undefined>(undefined);
  let checkedIds = $state<string[]>([]);
  let query = $state("");
  let sortKey = $state<SortKey>("updated_desc");
  let filter = $state<TypeFilter>("all");
  let loading = $state(true);
  let loadFailed = $state(false);
  let dialogOpen = $state(false);
  let dialogType = $state<ProjectType>("novel");
  let creating = $state(false);
  let deleteConfirmOpen = $state(false);
  /** 待删除 id 集合：批量操作取已勾选，行删除按钮取单行；确认框计数与执行都读它。 */
  let pendingDeleteIds = $state<string[]>([]);
  /** 行内改名态：正在编辑的行 id（同一时间最多一行）与提交中标志。 */
  let editingId = $state<string | null>(null);
  let renaming = $state(false);

  // 可见列表：筛选与排序的派生结果；项目名固定用户输入，不随语言重算。
  const visibleProjects = $derived(
    filterAndSortProjects(projects, { query, sortKey, filter, locale: getLocale() }),
  );
  const visibleIds = $derived(visibleProjects.map((project) => project.id));
  const selectAll = $derived(computeSelectAllState(visibleIds, checkedIds));

  // 右栏只预览可见列表中的选中项；筛掉或删掉后切空态但保留 activeId，不自动跳选。
  const activeProject = $derived(visibleProjects.find((project) => project.id === activeId));

  /** 选中项持久化键：新增键统一 `app.` 前缀，返回/刷新后仍选中上次项目。 */
  const ACTIVE_ID_KEY = "app.projects.activeId";

  /** 读存量选中；无痕/禁用存储时回落空，不阻断加载。 */
  function readStoredActiveId(): string | undefined {
    try {
      return localStorage.getItem(ACTIVE_ID_KEY) ?? undefined;
    } catch {
      // 存储不可用时视为无存量
      return undefined;
    }
  }

  /** 写存量选中；失败静默跳过，不影响主流程。 */
  function storeActiveId(id: string | undefined): void {
    try {
      if (id === undefined) {
        localStorage.removeItem(ACTIVE_ID_KEY);
      } else {
        localStorage.setItem(ACTIVE_ID_KEY, id);
      }
    } catch {
      // 存储不可用时跳过持久化
    }
  }

  /** 进页拉取一次：成功落本地态并补默认选中，失败走空列表 + 错误态。 */
  async function loadProjects(): Promise<void> {
    loading = true;
    loadFailed = false;
    const result = await commands.listProjects().result();
    loading = false;
    if (result.status === "error") {
      loadFailed = true;
      toast.error(m.projects_list_load_failed());
      return;
    }
    projects = result.data;
    // 勾选只保留仍存在的 id；选中优先恢复存量，缺失（删除/首次）时默认首个可见项。
    checkedIds = checkedIds.filter((id) => projects.some((project) => project.id === id));
    const storedId = readStoredActiveId();
    if (storedId !== undefined && projects.some((project) => project.id === storedId)) {
      activeId = storedId;
    } else {
      activeId = filterAndSortProjects(projects, { query, sortKey, filter }).at(0)?.id;
    }
    storeActiveId(activeId);
  }

  /** 行选中：落本地态并同步存量，返回后再进页可恢复。 */
  function handleSelect(id: string): void {
    activeId = id;
    storeActiveId(id);
  }

  /** 新建菜单选类型后开弹窗，类型决定弹窗标题。 */
  function handleCreateMenu(type: ProjectType): void {
    dialogType = type;
    dialogOpen = true;
  }

  /** 弹窗确认：后端建目录落盘，成功插顶并自动选中，失败保持打开可改。 */
  async function handleCreateConfirm(name: string): Promise<void> {
    creating = true;
    const result = await commands.createProject(name, dialogType).result();
    creating = false;
    if (result.status === "error") {
      toast.error(m.projects_toast_create_failed());
      return;
    }
    projects = [result.data, ...projects];
    activeId = result.data.id;
    storeActiveId(result.data.id);
    // 新项目在当前筛选/搜索下不可见时回落到全部，避免"建了却看不到"。
    const nowVisible = filterAndSortProjects(projects, { query, sortKey, filter }).some(
      (project) => project.id === result.data.id,
    );
    if (!nowVisible) {
      query = "";
      filter = "all";
    }
    dialogOpen = false;
    toast.success(m.projects_toast_created({ name: result.data.name }));
  }

  /** 批量菜单：目前仅删除，记下已勾选后弹确认框。 */
  function handleBatchAction(action: BatchAction): void {
    if (action === "delete" && checkedIds.length > 0) {
      pendingDeleteIds = [...checkedIds];
      deleteConfirmOpen = true;
    }
  }

  /** 行删除按钮：记下单行后走同一确认框。 */
  function handleRowDelete(id: string): void {
    pendingDeleteIds = [id];
    deleteConfirmOpen = true;
  }

  /** 确认删除：后端永久递归删目录，用返回数同步本地态；勾选只剔除已删项。 */
  async function handleDeleteConfirm(): Promise<void> {
    const ids = pendingDeleteIds;
    if (ids.length === 0) {
      return;
    }
    const result = await commands.deleteProjects(ids).result();
    if (result.status === "error") {
      toast.error(m.projects_toast_delete_failed());
      return;
    }
    const deleted = new Set(ids);
    projects = removeProjects(projects, ids);
    checkedIds = checkedIds.filter((id) => !deleted.has(id));
    if (editingId !== null && deleted.has(editingId)) {
      editingId = null;
    }
    toast.success(m.projects_toast_deleted({ count: result.data }));
  }

  /** 行内改名确认：后端回写成功才退出编辑态，失败保持编辑可改。 */
  async function handleRenameConfirm(id: string, name: string): Promise<void> {
    renaming = true;
    const result = await commands.renameProject(id, name).result();
    renaming = false;
    if (result.status === "error") {
      toast.error(m.projects_toast_rename_failed());
      return;
    }
    projects = projects.map((project) => (project.id === id ? result.data : project));
    editingId = null;
    toast.success(m.projects_toast_renamed({ name: result.data.name }));
  }

  onMount(() => {
    void loadProjects();
  });
</script>

<!-- 全幅页面：不套 max-w-2xl 单列约束，宽高占满布局 main 内容区 -->
<div class="flex h-full min-h-0 w-full min-w-0">
  <h1 class="sr-only">{m.page_projects_title()}</h1>
  <Resizable.PaneGroup direction="horizontal">
    <Resizable.Pane class="min-w-0">
      <div class="flex h-full min-h-0 flex-col gap-3 p-4">
        <ProjectsToolbar
          {query}
          {sortKey}
          {filter}
          hasChecked={checkedIds.length > 0}
          refreshing={loading}
          selectAllChecked={selectAll.checked}
          selectAllIndeterminate={selectAll.indeterminate}
          selectAllDisabled={selectAll.disabled || loading}
          onQueryChange={(value) => {
            // 可见条件变化会藏起编辑行，编辑态一起清掉，避免无出口残留。
            editingId = null;
            query = value;
          }}
          onSortChange={(value) => (sortKey = value)}
          onFilterChange={(value) => {
            editingId = null;
            filter = value;
          }}
          onToggleAll={(checked) =>
            (checkedIds = toggleAllChecked(checkedIds, visibleIds, checked))}
          onBatchAction={handleBatchAction}
          onRefresh={() => void loadProjects()}
          onCreate={handleCreateMenu}
        />
        {#if loading}
          <div
            class="flex min-h-0 flex-1 flex-col gap-1 p-1"
            aria-busy="true"
            aria-label={m.projects_list_loading()}
          >
            {#each [0, 1, 2, 3, 4] as index (index)}
              <Skeleton class="h-10 w-full" />
            {/each}
          </div>
        {:else if loadFailed}
          <div
            class="flex min-h-0 flex-1 flex-col items-center justify-center gap-3 p-6 text-center"
          >
            <p class="text-sm text-muted-foreground">{m.projects_list_load_failed()}</p>
            <Button variant="outline" size="sm" onclick={() => void loadProjects()}>
              {m.projects_list_retry()}
            </Button>
          </div>
        {:else if projects.length === 0}
          <div
            class="flex min-h-0 flex-1 flex-col items-center justify-center gap-3 p-6 text-center"
          >
            <p class="text-sm font-medium">{m.projects_empty_title()}</p>
            <p class="text-sm text-muted-foreground">{m.projects_empty_description()}</p>
            <div class="flex items-center gap-2">
              <Button size="sm" onclick={() => handleCreateMenu("novel")}>
                {m.page_home_new_novel()}
              </Button>
              <Button size="sm" variant="outline" onclick={() => handleCreateMenu("comic")}>
                {m.page_home_new_comic()}
              </Button>
            </div>
          </div>
        {:else}
          <ProjectsList
            projects={visibleProjects}
            {activeId}
            {checkedIds}
            {editingId}
            {renaming}
            onSelect={handleSelect}
            onToggle={(id, checked) => (checkedIds = toggleCheckedId(checkedIds, id, checked))}
            onEditStart={(id) => (editingId = id)}
            onEditCancel={() => (editingId = null)}
            onEditConfirm={(id, name) => void handleRenameConfirm(id, name)}
            onDelete={handleRowDelete}
            onOpen={(id) => {
              const project = projects.find((item) => item.id === id);
              if (project) {
                storeProjectEntry("/projects");
                void goto(resolve(`/${project.type}/${project.id}/metadata`));
              }
            }}
          />
        {/if}
      </div>
    </Resizable.Pane>
    <Resizable.Handle withHandle aria-label={m.page_projects_resize_label()} />
    <Resizable.Pane defaultSize={38.2} minSize={33} maxSize={50} class="min-w-0">
      <ProjectDetail project={activeProject} />
    </Resizable.Pane>
  </Resizable.PaneGroup>
</div>

<CreateProjectDialog
  bind:open={dialogOpen}
  type={dialogType}
  {creating}
  onConfirm={(name) => void handleCreateConfirm(name)}
/>
<ConfirmDialog
  bind:open={deleteConfirmOpen}
  title={m.projects_delete_confirm_title()}
  description={m.projects_delete_confirm_description({ count: pendingDeleteIds.length })}
  cancelLabel={m.projects_create_cancel()}
  confirmLabel={m.projects_delete_confirm_ok()}
  variant="destructive"
  onConfirm={() => void handleDeleteConfirm()}
/>
