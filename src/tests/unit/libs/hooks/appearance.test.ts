import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  COLOR_THEME_STORAGE_KEY,
  DEFAULT_COLOR_THEME,
  DEFAULT_EDITOR_FONT_FAMILY,
  DEFAULT_EDITOR_FONT_SIZE,
  DEFAULT_EDITOR_FONT_WEIGHT,
  DEFAULT_EDITOR_INDENT,
  DEFAULT_FONT_FAMILY,
  DEFAULT_FONT_SIZE,
  DEFAULT_FONT_WEIGHT,
  DEFAULT_LAYOUT,
  EDITOR_FONT_FAMILY_STORAGE_KEY,
  EDITOR_FONT_SIZE_OPTIONS,
  EDITOR_FONT_SIZE_STORAGE_KEY,
  EDITOR_FONT_WEIGHT_STORAGE_KEY,
  EDITOR_INDENT_STORAGE_KEY,
  FONT_FAMILY_STORAGE_KEY,
  FONT_SIZE_OPTIONS,
  FONT_SIZE_STORAGE_KEY,
  FONT_WEIGHT_STORAGE_KEY,
  LAYOUTS,
  LAYOUT_STORAGE_KEY,
  applyAppearance,
  applyColorTheme,
  applyEditorTypography,
  buildFontStack,
  colorThemeState,
  editorTypographyState,
  fontState,
  initAppearance,
  initLayout,
  isColorTheme,
  isEditorFontSize,
  isEditorIndent,
  isFontSize,
  isFontWeight,
  isLayoutName,
  layoutState,
  loadColorTheme,
  loadEditorFontFamily,
  loadEditorFontSize,
  loadEditorFontWeight,
  loadEditorIndent,
  loadFontFamily,
  loadFontSize,
  loadFontWeight,
  loadLayoutName,
  resetAppearance,
  sanitizeFontFamily,
  setColorTheme,
  setEditorFontFamily,
  setEditorFontSize,
  setEditorFontWeight,
  setEditorIndent,
  setFontFamily,
  setFontSize,
  setFontWeight,
  setLayoutName,
} from "#libs/hooks/appearance.svelte";
import {
  DASHBOARD_COLLAPSED_STORAGE_KEY,
  DEFAULT_DASHBOARD_COLLAPSED,
  dashboardState,
} from "#libs/hooks/dashboard.svelte";

// 节点环境无 localStorage，用内存实现替身，保证单测不依赖浏览器
function installLocalStorageMock() {
  const store = new Map<string, string>();
  const mock = {
    getItem: (key: string): string | null => store.get(key) ?? null,
    setItem: (key: string, value: string): void => {
      store.set(key, String(value));
    },
    removeItem: (key: string): void => {
      store.delete(key);
    },
    clear: (): void => {
      store.clear();
    },
  };
  vi.stubGlobal("localStorage", mock);
  return mock;
}

beforeEach(() => {
  installLocalStorageMock();
  layoutState.name = "tabs";
  dashboardState.collapsed = DEFAULT_DASHBOARD_COLLAPSED;
  colorThemeState.name = DEFAULT_COLOR_THEME;
  fontState.family = DEFAULT_FONT_FAMILY;
  fontState.weight = DEFAULT_FONT_WEIGHT;
  fontState.size = DEFAULT_FONT_SIZE;
  editorTypographyState.family = DEFAULT_EDITOR_FONT_FAMILY;
  editorTypographyState.weight = DEFAULT_EDITOR_FONT_WEIGHT;
  editorTypographyState.size = DEFAULT_EDITOR_FONT_SIZE;
  editorTypographyState.indent = DEFAULT_EDITOR_INDENT;
  vi.restoreAllMocks();
});

describe("isLayoutName", () => {
  it("接受合法取值", () => {
    expect(isLayoutName("tabs")).toBe(true);
    expect(isLayoutName("sidebar")).toBe(true);
  });

  it("拒绝非法取值", () => {
    expect(isLayoutName("unknown")).toBe(false);
    expect(isLayoutName(null)).toBe(false);
    expect(isLayoutName(undefined)).toBe(false);
  });
});

describe("loadLayoutName", () => {
  it("缺失时回落默认值", () => {
    expect(loadLayoutName()).toBe("tabs");
  });

  it("读取已持久化的合法值", () => {
    localStorage.setItem(LAYOUT_STORAGE_KEY, "sidebar");
    expect(loadLayoutName()).toBe("sidebar");
  });

  it("脏数据回落默认值", () => {
    localStorage.setItem(LAYOUT_STORAGE_KEY, "unknown");
    expect(loadLayoutName()).toBe("tabs");
  });

  it("读取异常回落默认值", () => {
    const mock = installLocalStorageMock();
    vi.spyOn(mock, "getItem").mockImplementation(() => {
      throw new Error("denied");
    });
    expect(loadLayoutName()).toBe("tabs");
  });
});

describe("setLayoutName", () => {
  it("先落盘再切换内存状态", () => {
    setLayoutName("sidebar");
    expect(localStorage.getItem(LAYOUT_STORAGE_KEY)).toBe("sidebar");
    expect(layoutState.name).toBe("sidebar");
  });

  it("落盘失败仍切换内存状态", () => {
    const mock = installLocalStorageMock();
    vi.spyOn(mock, "setItem").mockImplementation(() => {
      throw new Error("denied");
    });
    setLayoutName("sidebar");
    expect(layoutState.name).toBe("sidebar");
  });

  it("拒绝非法取值且不改状态", () => {
    // @ts-expect-error 故意传入非法值以覆盖运行时分支
    setLayoutName("unknown");
    expect(layoutState.name).toBe("tabs");
    expect(localStorage.getItem(LAYOUT_STORAGE_KEY)).toBeNull();
  });
});

describe("initLayout", () => {
  it("按持久化值初始化", () => {
    localStorage.setItem(LAYOUT_STORAGE_KEY, "sidebar");
    initLayout();
    expect(layoutState.name).toBe("sidebar");
  });
});

describe("LAYOUTS", () => {
  // 动态 import 三个布局组件（含 Svelte 编译 + bits-ui 链），CI 并行压力下可能超过默认 5s。
  it("合法取值均有对应组件", async () => {
    for (const name of ["tabs", "sidebar", "dashboard"] as const) {
      expect(isLayoutName(name)).toBe(true);
      await expect(LAYOUTS[name]()).resolves.toBeDefined();
    }
  }, 30000);
});

describe("sanitizeFontFamily", () => {
  it("保留合法族名", () => {
    expect(sanitizeFontFamily("Microsoft YaHei")).toBe("Microsoft YaHei");
  });

  it("剥离可破坏样式声明的字符", () => {
    expect(sanitizeFontFamily('";color:red;"')).toBe("color:red");
    expect(sanitizeFontFamily("Foo\\Bar")).toBe("FooBar");
  });
});

describe("isFontWeight", () => {
  it("接受整百字重", () => {
    for (const weight of [100, 400, 900]) {
      expect(isFontWeight(weight)).toBe(true);
    }
  });

  it("拒绝非法取值", () => {
    expect(isFontWeight(450)).toBe(false);
    expect(isFontWeight(50)).toBe(false);
    expect(isFontWeight(1000)).toBe(false);
    expect(isFontWeight("400")).toBe(false);
    expect(isFontWeight(null)).toBe(false);
  });
});

describe("loadFontFamily", () => {
  it("缺失时回落系统默认", () => {
    expect(loadFontFamily()).toBe(DEFAULT_FONT_FAMILY);
  });

  it("读取已持久化的族名", () => {
    localStorage.setItem(FONT_FAMILY_STORAGE_KEY, "Serif");
    expect(loadFontFamily()).toBe("Serif");
  });

  it("脏数据回落系统默认", () => {
    localStorage.setItem(FONT_FAMILY_STORAGE_KEY, '";;');
    expect(loadFontFamily()).toBe(DEFAULT_FONT_FAMILY);
  });
});

describe("loadFontWeight", () => {
  it("缺失时回落默认值", () => {
    expect(loadFontWeight()).toBe(DEFAULT_FONT_WEIGHT);
  });

  it("读取已持久化的合法字重", () => {
    localStorage.setItem(FONT_WEIGHT_STORAGE_KEY, "700");
    expect(loadFontWeight()).toBe(700);
  });

  it("脏数据回落默认值", () => {
    localStorage.setItem(FONT_WEIGHT_STORAGE_KEY, "450");
    expect(loadFontWeight()).toBe(DEFAULT_FONT_WEIGHT);
  });
});

describe("setFontFamily", () => {
  it("先落盘再切换内存状态", () => {
    setFontFamily("Serif");
    expect(localStorage.getItem(FONT_FAMILY_STORAGE_KEY)).toBe("Serif");
    expect(fontState.family).toBe("Serif");
  });

  it("拒绝空取值且不改状态", () => {
    setFontFamily("   ");
    expect(fontState.family).toBe(DEFAULT_FONT_FAMILY);
    expect(localStorage.getItem(FONT_FAMILY_STORAGE_KEY)).toBeNull();
  });
});

describe("setFontWeight", () => {
  it("先落盘再切换内存状态", () => {
    setFontWeight(700);
    expect(localStorage.getItem(FONT_WEIGHT_STORAGE_KEY)).toBe("700");
    expect(fontState.weight).toBe(700);
  });

  it("预览模式只改内存不落盘", () => {
    setFontWeight(700, false);
    expect(localStorage.getItem(FONT_WEIGHT_STORAGE_KEY)).toBeNull();
    expect(fontState.weight).toBe(700);
  });

  it("拒绝非法取值且不改状态", () => {
    setFontWeight(450);
    expect(fontState.weight).toBe(DEFAULT_FONT_WEIGHT);
    expect(localStorage.getItem(FONT_WEIGHT_STORAGE_KEY)).toBeNull();
  });
});

describe("isFontSize", () => {
  it("接受步长倍数字号", () => {
    for (const size of [75, 100, 125]) {
      expect(isFontSize(size)).toBe(true);
    }
  });

  it("拒绝非法取值", () => {
    expect(isFontSize(77)).toBe(false);
    expect(isFontSize(70)).toBe(false);
    expect(isFontSize(130)).toBe(false);
    expect(isFontSize("100")).toBe(false);
    expect(isFontSize(null)).toBe(false);
  });
});

describe("loadFontSize", () => {
  it("缺失时回落默认值", () => {
    expect(loadFontSize()).toBe(DEFAULT_FONT_SIZE);
  });

  it("读取已持久化的合法字号", () => {
    localStorage.setItem(FONT_SIZE_STORAGE_KEY, "110");
    expect(loadFontSize()).toBe(110);
  });

  it("脏数据回落默认值", () => {
    localStorage.setItem(FONT_SIZE_STORAGE_KEY, "77");
    expect(loadFontSize()).toBe(DEFAULT_FONT_SIZE);
  });
});

describe("setFontSize", () => {
  it("先落盘再切换内存状态", () => {
    setFontSize(110);
    expect(localStorage.getItem(FONT_SIZE_STORAGE_KEY)).toBe("110");
    expect(fontState.size).toBe(110);
  });

  it("拒绝非法取值且不改状态", () => {
    setFontSize(77);
    expect(fontState.size).toBe(DEFAULT_FONT_SIZE);
    expect(localStorage.getItem(FONT_SIZE_STORAGE_KEY)).toBeNull();
  });
});

describe("FONT_SIZE_OPTIONS", () => {
  it("与校验区间同源", () => {
    expect(FONT_SIZE_OPTIONS).toEqual([75, 80, 85, 90, 95, 100, 105, 110, 115, 120, 125]);
    for (const size of FONT_SIZE_OPTIONS) {
      expect(isFontSize(size)).toBe(true);
    }
  });
});

describe("buildFontStack", () => {
  it("系统默认返回默认栈", () => {
    expect(buildFontStack(DEFAULT_FONT_FAMILY)).toContain("Geist Variable");
  });

  it("选中族名优先并带回退", () => {
    const stack = buildFontStack("Serif");
    expect(stack.indexOf('"Serif"')).toBe(0);
    expect(stack).toContain("Geist Variable");
  });
});

describe("applyAppearance", () => {
  it("无文档环境直接返回", () => {
    expect(() => applyAppearance()).not.toThrow();
  });

  it("把内存态写入根元素变量", () => {
    const store = new Map<string, string>();
    const style = {
      setProperty: (key: string, value: string): void => {
        store.set(key, value);
      },
      getPropertyValue: (key: string): string => store.get(key) ?? "",
    };
    vi.stubGlobal("document", { documentElement: { style } });
    try {
      fontState.family = "Serif";
      fontState.weight = 700;
      fontState.size = 110;
      applyAppearance();
      expect(style.getPropertyValue("--app-font-family")).toContain('"Serif"');
      expect(style.getPropertyValue("--app-font-weight")).toBe("700");
      expect(style.getPropertyValue("--app-font-size")).toBe("110%");
    } finally {
      vi.unstubAllGlobals();
    }
  });
});

describe("initAppearance", () => {
  it("按持久化值初始化并应用", () => {
    localStorage.setItem(FONT_FAMILY_STORAGE_KEY, "Serif");
    localStorage.setItem(FONT_WEIGHT_STORAGE_KEY, "700");
    localStorage.setItem(FONT_SIZE_STORAGE_KEY, "110");
    localStorage.setItem(EDITOR_FONT_FAMILY_STORAGE_KEY, "Mono");
    localStorage.setItem(EDITOR_FONT_WEIGHT_STORAGE_KEY, "500");
    localStorage.setItem(EDITOR_FONT_SIZE_STORAGE_KEY, "18");
    localStorage.setItem(EDITOR_INDENT_STORAGE_KEY, "0");
    localStorage.setItem(COLOR_THEME_STORAGE_KEY, "ocean");
    initAppearance();
    expect(fontState.family).toBe("Serif");
    expect(fontState.weight).toBe(700);
    expect(fontState.size).toBe(110);
    expect(editorTypographyState.family).toBe("Mono");
    expect(editorTypographyState.weight).toBe(500);
    expect(editorTypographyState.size).toBe(18);
    expect(editorTypographyState.indent).toBe(0);
    expect(colorThemeState.name).toBe("ocean");
  });
});

describe("isColorTheme", () => {
  it("只接受取值元组内的语义名", () => {
    for (const name of [
      "neutral",
      "ocean",
      "teal",
      "emerald",
      "amber",
      "tangerine",
      "rose",
      "amethyst",
      "nord",
      "dracula",
      "tokyo-night",
    ]) {
      expect(isColorTheme(name)).toBe(true);
    }
    for (const value of ["default", "blue", "green", "violet", "dark", "", null, undefined, 0]) {
      expect(isColorTheme(value)).toBe(false);
    }
  });
});

describe("loadColorTheme", () => {
  it("缺失时回落默认配色", () => {
    expect(loadColorTheme()).toBe(DEFAULT_COLOR_THEME);
  });

  it("合法存量原样返回", () => {
    localStorage.setItem(COLOR_THEME_STORAGE_KEY, "amethyst");
    expect(loadColorTheme()).toBe("amethyst");
  });

  it("更名前的旧取值视为脏数据回落默认配色", () => {
    localStorage.setItem(COLOR_THEME_STORAGE_KEY, "blue");
    expect(loadColorTheme()).toBe(DEFAULT_COLOR_THEME);
  });

  it("脏数据回落默认配色", () => {
    localStorage.setItem(COLOR_THEME_STORAGE_KEY, "neon");
    expect(loadColorTheme()).toBe(DEFAULT_COLOR_THEME);
  });
});

describe("applyColorTheme", () => {
  it("无文档环境直接返回", () => {
    expect(() => applyColorTheme()).not.toThrow();
  });

  it("配色属性避开 mode-watcher 占用的 data-theme", () => {
    // mode-watcher 挂载即把 data-theme 置空，配色须经独立属性生效，否则重启即被覆盖
    const attrs = new Map<string, string>();
    vi.stubGlobal("document", {
      documentElement: {
        setAttribute: (key: string, value: string): void => {
          attrs.set(key, value);
        },
        removeAttribute: (key: string): void => {
          attrs.delete(key);
        },
      },
    });
    try {
      colorThemeState.name = "ocean";
      applyColorTheme();
      expect(attrs.get("data-color-theme")).toBe("ocean");
      expect(attrs.has("data-theme")).toBe(false);
    } finally {
      vi.unstubAllGlobals();
    }
  });

  it("默认配色删属性回落样式表，非默认写属性", () => {
    const attrs = new Map<string, string>();
    const root = {
      setAttribute: (key: string, value: string): void => {
        attrs.set(key, value);
      },
      removeAttribute: (key: string): void => {
        attrs.delete(key);
      },
      getAttribute: (key: string): string | null => attrs.get(key) ?? null,
    };
    vi.stubGlobal("document", { documentElement: root });
    try {
      colorThemeState.name = "rose";
      applyColorTheme();
      expect(root.getAttribute("data-color-theme")).toBe("rose");
      colorThemeState.name = DEFAULT_COLOR_THEME;
      applyColorTheme();
      expect(root.getAttribute("data-color-theme")).toBeNull();
    } finally {
      vi.unstubAllGlobals();
    }
  });
});

describe("setColorTheme", () => {
  it("非法取值直接拒绝", () => {
    const attrs = new Map<string, string>();
    vi.stubGlobal("document", {
      documentElement: {
        setAttribute: (key: string, value: string): void => {
          attrs.set(key, value);
        },
        removeAttribute: (key: string): void => {
          attrs.delete(key);
        },
      },
    });
    try {
      setColorTheme("neon" as never);
      expect(colorThemeState.name).toBe(DEFAULT_COLOR_THEME);
      expect(localStorage.getItem(COLOR_THEME_STORAGE_KEY)).toBeNull();
      expect(attrs.size).toBe(0);
    } finally {
      vi.unstubAllGlobals();
    }
  });

  it("先落盘再改内存并即时应用", () => {
    const attrs = new Map<string, string>();
    vi.stubGlobal("document", {
      documentElement: {
        setAttribute: (key: string, value: string): void => {
          attrs.set(key, value);
        },
        removeAttribute: (key: string): void => {
          attrs.delete(key);
        },
      },
    });
    try {
      setColorTheme("teal");
      expect(localStorage.getItem(COLOR_THEME_STORAGE_KEY)).toBe("teal");
      expect(colorThemeState.name).toBe("teal");
      expect(attrs.get("data-color-theme")).toBe("teal");
    } finally {
      vi.unstubAllGlobals();
    }
  });
});

describe("resetAppearance", () => {
  it("十项偏好全部回到默认值并落盘", () => {
    layoutState.name = "sidebar";
    dashboardState.collapsed = true;
    colorThemeState.name = "rose";
    fontState.family = "Serif";
    fontState.weight = 700;
    fontState.size = 120;
    editorTypographyState.family = "Mono";
    editorTypographyState.weight = 500;
    editorTypographyState.size = 20;
    editorTypographyState.indent = 0;

    resetAppearance();

    expect(layoutState.name).toBe(DEFAULT_LAYOUT);
    expect(dashboardState.collapsed).toBe(DEFAULT_DASHBOARD_COLLAPSED);
    expect(colorThemeState.name).toBe(DEFAULT_COLOR_THEME);
    expect(fontState.family).toBe(DEFAULT_FONT_FAMILY);
    expect(fontState.weight).toBe(DEFAULT_FONT_WEIGHT);
    expect(fontState.size).toBe(DEFAULT_FONT_SIZE);
    expect(editorTypographyState.family).toBe(DEFAULT_EDITOR_FONT_FAMILY);
    expect(editorTypographyState.weight).toBe(DEFAULT_EDITOR_FONT_WEIGHT);
    expect(editorTypographyState.size).toBe(DEFAULT_EDITOR_FONT_SIZE);
    expect(editorTypographyState.indent).toBe(DEFAULT_EDITOR_INDENT);
    expect(localStorage.getItem(LAYOUT_STORAGE_KEY)).toBe(DEFAULT_LAYOUT);
    expect(localStorage.getItem(DASHBOARD_COLLAPSED_STORAGE_KEY)).toBe(
      String(DEFAULT_DASHBOARD_COLLAPSED),
    );
    expect(localStorage.getItem(COLOR_THEME_STORAGE_KEY)).toBe(DEFAULT_COLOR_THEME);
    expect(localStorage.getItem(FONT_FAMILY_STORAGE_KEY)).toBe(DEFAULT_FONT_FAMILY);
    expect(localStorage.getItem(FONT_WEIGHT_STORAGE_KEY)).toBe(String(DEFAULT_FONT_WEIGHT));
    expect(localStorage.getItem(FONT_SIZE_STORAGE_KEY)).toBe(String(DEFAULT_FONT_SIZE));
    expect(localStorage.getItem(EDITOR_FONT_FAMILY_STORAGE_KEY)).toBe(DEFAULT_EDITOR_FONT_FAMILY);
    expect(localStorage.getItem(EDITOR_FONT_WEIGHT_STORAGE_KEY)).toBe(
      String(DEFAULT_EDITOR_FONT_WEIGHT),
    );
    expect(localStorage.getItem(EDITOR_FONT_SIZE_STORAGE_KEY)).toBe(
      String(DEFAULT_EDITOR_FONT_SIZE),
    );
    expect(localStorage.getItem(EDITOR_INDENT_STORAGE_KEY)).toBe(String(DEFAULT_EDITOR_INDENT));
  });
});

describe("isEditorFontSize", () => {
  it("只接受离散档内的字号", () => {
    for (const size of EDITOR_FONT_SIZE_OPTIONS) {
      expect(isEditorFontSize(size)).toBe(true);
    }
  });

  it("拒绝非法取值", () => {
    expect(isEditorFontSize(17)).toBe(false);
    expect(isEditorFontSize(100)).toBe(false);
    expect(isEditorFontSize("16")).toBe(false);
    expect(isEditorFontSize(null)).toBe(false);
  });
});

describe("isEditorIndent", () => {
  it("接受区间内的整数字符数", () => {
    for (const indent of [0, 2, 4]) {
      expect(isEditorIndent(indent)).toBe(true);
    }
  });

  it("拒绝非法取值", () => {
    expect(isEditorIndent(2.5)).toBe(false);
    expect(isEditorIndent(-1)).toBe(false);
    expect(isEditorIndent(5)).toBe(false);
    expect(isEditorIndent("2")).toBe(false);
    expect(isEditorIndent(null)).toBe(false);
  });
});

describe("loadEditorFontFamily", () => {
  it("缺失时回落系统默认", () => {
    expect(loadEditorFontFamily()).toBe(DEFAULT_EDITOR_FONT_FAMILY);
  });

  it("读取已持久化的合法族名", () => {
    localStorage.setItem(EDITOR_FONT_FAMILY_STORAGE_KEY, "Mono");
    expect(loadEditorFontFamily()).toBe("Mono");
  });

  it("脏数据回落系统默认", () => {
    localStorage.setItem(EDITOR_FONT_FAMILY_STORAGE_KEY, '";;');
    expect(loadEditorFontFamily()).toBe(DEFAULT_EDITOR_FONT_FAMILY);
  });
});

describe("loadEditorFontWeight", () => {
  it("缺失时回落默认值", () => {
    expect(loadEditorFontWeight()).toBe(DEFAULT_EDITOR_FONT_WEIGHT);
  });

  it("读取已持久化的合法字重", () => {
    localStorage.setItem(EDITOR_FONT_WEIGHT_STORAGE_KEY, "500");
    expect(loadEditorFontWeight()).toBe(500);
  });

  it("脏数据回落默认值", () => {
    localStorage.setItem(EDITOR_FONT_WEIGHT_STORAGE_KEY, "450");
    expect(loadEditorFontWeight()).toBe(DEFAULT_EDITOR_FONT_WEIGHT);
  });
});

describe("loadEditorFontSize", () => {
  it("缺失时回落默认值", () => {
    expect(loadEditorFontSize()).toBe(DEFAULT_EDITOR_FONT_SIZE);
  });

  it("读取已持久化的合法字号", () => {
    localStorage.setItem(EDITOR_FONT_SIZE_STORAGE_KEY, "18");
    expect(loadEditorFontSize()).toBe(18);
  });

  it("非离散档脏数据回落默认值", () => {
    localStorage.setItem(EDITOR_FONT_SIZE_STORAGE_KEY, "17");
    expect(loadEditorFontSize()).toBe(DEFAULT_EDITOR_FONT_SIZE);
  });
});

describe("loadEditorIndent", () => {
  it("缺失时回落默认值", () => {
    expect(loadEditorIndent()).toBe(DEFAULT_EDITOR_INDENT);
  });

  it("读取已持久化的合法缩进", () => {
    localStorage.setItem(EDITOR_INDENT_STORAGE_KEY, "0");
    expect(loadEditorIndent()).toBe(0);
  });

  it("脏数据回落默认值", () => {
    localStorage.setItem(EDITOR_INDENT_STORAGE_KEY, "9");
    expect(loadEditorIndent()).toBe(DEFAULT_EDITOR_INDENT);
  });
});

describe("setEditorFontFamily", () => {
  it("先落盘再切换内存状态", () => {
    setEditorFontFamily("Mono");
    expect(localStorage.getItem(EDITOR_FONT_FAMILY_STORAGE_KEY)).toBe("Mono");
    expect(editorTypographyState.family).toBe("Mono");
  });

  it("拒绝空取值且不改状态", () => {
    setEditorFontFamily("   ");
    expect(editorTypographyState.family).toBe(DEFAULT_EDITOR_FONT_FAMILY);
    expect(localStorage.getItem(EDITOR_FONT_FAMILY_STORAGE_KEY)).toBeNull();
  });
});

describe("setEditorFontWeight", () => {
  it("先落盘再切换内存状态", () => {
    setEditorFontWeight(500);
    expect(localStorage.getItem(EDITOR_FONT_WEIGHT_STORAGE_KEY)).toBe("500");
    expect(editorTypographyState.weight).toBe(500);
  });

  it("预览模式只改内存不落盘", () => {
    setEditorFontWeight(500, false);
    expect(localStorage.getItem(EDITOR_FONT_WEIGHT_STORAGE_KEY)).toBeNull();
    expect(editorTypographyState.weight).toBe(500);
  });

  it("拒绝非法取值且不改状态", () => {
    setEditorFontWeight(450);
    expect(editorTypographyState.weight).toBe(DEFAULT_EDITOR_FONT_WEIGHT);
    expect(localStorage.getItem(EDITOR_FONT_WEIGHT_STORAGE_KEY)).toBeNull();
  });
});

describe("setEditorFontSize", () => {
  it("先落盘再切换内存状态", () => {
    setEditorFontSize(18);
    expect(localStorage.getItem(EDITOR_FONT_SIZE_STORAGE_KEY)).toBe("18");
    expect(editorTypographyState.size).toBe(18);
  });

  it("拒绝非离散档取值且不改状态", () => {
    setEditorFontSize(17);
    expect(editorTypographyState.size).toBe(DEFAULT_EDITOR_FONT_SIZE);
    expect(localStorage.getItem(EDITOR_FONT_SIZE_STORAGE_KEY)).toBeNull();
  });
});

describe("setEditorIndent", () => {
  it("先落盘再切换内存状态", () => {
    setEditorIndent(0);
    expect(localStorage.getItem(EDITOR_INDENT_STORAGE_KEY)).toBe("0");
    expect(editorTypographyState.indent).toBe(0);
  });

  it("预览模式只改内存不落盘", () => {
    setEditorIndent(4, false);
    expect(localStorage.getItem(EDITOR_INDENT_STORAGE_KEY)).toBeNull();
    expect(editorTypographyState.indent).toBe(4);
  });

  it("拒绝非法取值且不改状态", () => {
    setEditorIndent(9);
    expect(editorTypographyState.indent).toBe(DEFAULT_EDITOR_INDENT);
    expect(localStorage.getItem(EDITOR_INDENT_STORAGE_KEY)).toBeNull();
  });
});

describe("applyEditorTypography", () => {
  it("无文档环境直接返回", () => {
    expect(() => applyEditorTypography()).not.toThrow();
  });

  it("把内存态写入根元素变量", () => {
    const store = new Map<string, string>();
    const style = {
      setProperty: (key: string, value: string): void => {
        store.set(key, String(value));
      },
      getPropertyValue: (key: string): string => store.get(key) ?? "",
    };
    vi.stubGlobal("document", { documentElement: { style } });
    try {
      editorTypographyState.family = "Mono";
      editorTypographyState.weight = 500;
      editorTypographyState.size = 18;
      editorTypographyState.indent = 0;
      applyEditorTypography();
      expect(style.getPropertyValue("--app-editor-font-family")).toContain('"Mono"');
      expect(style.getPropertyValue("--app-editor-font-weight")).toBe("500");
      expect(style.getPropertyValue("--app-editor-font-size")).toBe("18px");
      expect(style.getPropertyValue("--app-editor-indent")).toBe("0");
    } finally {
      vi.unstubAllGlobals();
    }
  });
});
