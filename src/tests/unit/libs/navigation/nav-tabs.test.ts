import { describe, expect, it } from "vitest";
import { NAV_TABS, resolveNavCurrent, resolveNavTab } from "#libs/navigation/nav-tabs";

describe("导航标签注册表", () => {
  it("首页固定在最前且路径不重复", () => {
    const paths = NAV_TABS.map((tab) => tab.path);

    expect(paths[0]).toBe("/");
    expect(new Set(paths).size).toBe(paths.length);
  });

  it("按路径命中标签", () => {
    for (const tab of NAV_TABS) {
      expect(resolveNavTab(tab.path)).toBe(tab);
    }
  });

  it("未登记的路径不选中任何标签", () => {
    expect(resolveNavTab("/unknown")).toBeUndefined();
  });

  it("导航项状态：当前页带 aria-current，其余不带", () => {
    for (const tab of NAV_TABS) {
      expect(resolveNavCurrent(tab.path, tab.path)).toBe("page");
    }
  });

  it("导航项状态：未登记的路径不选中任何项", () => {
    for (const tab of NAV_TABS) {
      expect(resolveNavCurrent(tab.path, "/unknown")).toBeUndefined();
    }
  });

  it("每个标签都有文案与图标", () => {
    for (const tab of NAV_TABS) {
      // 节点环境无 window，Paraglide 回落 baseLocale，取到英文文案
      expect(tab.label()).toBeTruthy();
      expect(tab.icon).toBeTruthy();
    }
  });
});
