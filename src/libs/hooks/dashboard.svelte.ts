// 仪表盘布局的侧栏折叠偏好：与布局、配色同属纯前端 UI 偏好，落 localStorage，不经 commands 链。
// 独立于 appearance.svelte：布局组件禁止反向导入外观注册表（会形成容器到布局的循环依赖），
// 折叠状态由布局自身持有，重置仍复用 resetAppearance 里的 set 路径。

/** 持久化键名，新增键统一 `app.` 前缀；改名即视为放弃老用户存量 */
export const DASHBOARD_COLLAPSED_STORAGE_KEY = "dashboard-collapsed";

/** 默认展开：首选项，缺失或脏数据一律回落 */
export const DEFAULT_DASHBOARD_COLLAPSED: boolean = false;

// 跨挂载共享的折叠状态：布局按 `{#key}` 重挂载后据此恢复，页面私有状态仍用局部 $state
export const dashboardState = $state<{ collapsed: boolean }>({
  collapsed: DEFAULT_DASHBOARD_COLLAPSED,
});

/** 读取持久化折叠偏好，缺失或非法一律回落默认值 */
export function loadDashboardCollapsed(): boolean {
  try {
    if (typeof localStorage === "undefined") {
      return DEFAULT_DASHBOARD_COLLAPSED;
    }
    const stored = localStorage.getItem(DASHBOARD_COLLAPSED_STORAGE_KEY);
    if (stored !== "true" && stored !== "false") {
      return DEFAULT_DASHBOARD_COLLAPSED;
    }
    return stored === "true";
  } catch {
    // 隐私模式等存储不可用时按默认值渲染，不阻塞首帧
    return DEFAULT_DASHBOARD_COLLAPSED;
  }
}

/** 布局首帧前调用，重复调用无副作用 */
export function initDashboard(): void {
  dashboardState.collapsed = loadDashboardCollapsed();
}

/** 切换折叠，先落盘再改内存，刷新与布局切换都不丢失 */
export function setDashboardCollapsed(next: boolean): void {
  try {
    localStorage.setItem(DASHBOARD_COLLAPSED_STORAGE_KEY, String(next));
  } catch {
    // 落盘失败仍切换内存状态，保证本次会话可用
  }
  dashboardState.collapsed = next;
}
