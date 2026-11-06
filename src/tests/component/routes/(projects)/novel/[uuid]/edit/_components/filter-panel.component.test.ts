import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import type {
  CleanupPreset,
  PairKind,
  PunctPreset,
} from "../../../../../../../../libs/commands/types";
import FilterPanel from "../../../../../../../../routes/(projects)/novel/[uuid]/edit/_components/filter-panel.svelte";

const runMock = vi.hoisted(() => vi.fn());
const clearMock = vi.hoisted(() => vi.fn());

const baseProps = {
  kind: "keyword" as const,
  keyword: "",
  caseSensitive: false,
  pattern: "",
  cleanupPresets: [] as CleanupPreset[],
  punctPresets: [] as PunctPreset[],
  pairScope: "Chapter" as const,
  pairPresets: [] as PairKind[],
  filtering: false,
  canFilter: false,
  hasOutcome: false,
  onRun: runMock,
  onClear: clearMock,
};

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

describe("筛选面板", () => {
  it("关键词模式显示输入与区分大小写，空输入时筛选禁用", () => {
    render(FilterPanel, { props: baseProps });

    expect(screen.getByPlaceholderText("Enter keyword…")).not.toBeNull();
    expect(screen.getByText("Case sensitive")).not.toBeNull();
    expect(screen.queryByPlaceholderText("Enter regex (Rust syntax)…")).toBeNull();
    expect((screen.getByRole("button", { name: "Filter" }) as HTMLButtonElement).disabled).toBe(
      true,
    );
    expect((screen.getByRole("button", { name: "Clear" }) as HTMLButtonElement).disabled).toBe(
      true,
    );
  });

  it("正则模式显示 pattern 输入与语法提示", () => {
    render(FilterPanel, { props: { ...baseProps, kind: "regex" } });

    expect(screen.getByPlaceholderText("Enter regex (Rust syntax)…")).not.toBeNull();
    expect(screen.getByText("Rust regex syntax, matched per line, e.g. 第.*章")).not.toBeNull();
    expect(screen.queryByPlaceholderText("Enter keyword…")).toBeNull();
  });

  it("有输入且可用时点击筛选走回调，有结果时可清除", async () => {
    const user = userEvent.setup();
    render(FilterPanel, {
      props: { ...baseProps, keyword: "正文", canFilter: true, hasOutcome: true },
    });

    const run = screen.getByRole("button", { name: "Filter" });
    expect((run as HTMLButtonElement).disabled).toBe(false);
    await user.click(run);
    expect(runMock).toHaveBeenCalledOnce();

    const clear = screen.getByRole("button", { name: "Clear" });
    expect((clear as HTMLButtonElement).disabled).toBe(false);
    await user.click(clear);
    expect(clearMock).toHaveBeenCalledOnce();
  });

  it("筛选中按钮显示进行态", () => {
    render(FilterPanel, { props: { ...baseProps, keyword: "正文", filtering: true } });

    expect(screen.getByRole("button", { name: "Filtering…" })).not.toBeNull();
  });

  it("垃圾清理模式显示分组与三态组头，叶子勾选即时生效", async () => {
    const user = userEvent.setup();
    render(FilterPanel, { props: { ...baseProps, kind: "cleanup", cleanupPresets: ["UrlFull"] } });

    const group = screen.getByRole("checkbox", { name: "URLs" });
    const full = screen.getByRole("checkbox", { name: "Full URLs" });
    const domain = screen.getByRole("checkbox", { name: "Domain names" });
    expect(group.getAttribute("data-state")).toBe("indeterminate");
    expect(full.getAttribute("data-state")).toBe("checked");
    expect(domain.getAttribute("data-state")).toBe("unchecked");

    // 叶子勾选即时生效
    await user.click(domain);
    expect(domain.getAttribute("data-state")).toBe("checked");
    expect(group.getAttribute("data-state")).toBe("checked");

    // 组头全开态点一下全关
    await user.click(group);
    expect(full.getAttribute("data-state")).toBe("unchecked");
    expect(domain.getAttribute("data-state")).toBe("unchecked");

    // 组头全关态点一下全开
    await user.click(group);
    expect(full.getAttribute("data-state")).toBe("checked");
    expect(domain.getAttribute("data-state")).toBe("checked");
  });

  it("标点符号模式显示分组与三态组头，叶子勾选即时生效", async () => {
    const user = userEvent.setup();
    render(FilterPanel, { props: { ...baseProps, kind: "punct", punctPresets: ["HalfPunct"] } });

    const group = screen.getByRole("checkbox", { name: "Width norms" });
    const half = screen.getByRole("checkbox", { name: "Half-width punct" });
    const full = screen.getByRole("checkbox", { name: "Full-width forms" });
    expect(group.getAttribute("data-state")).toBe("indeterminate");
    expect(half.getAttribute("data-state")).toBe("checked");
    expect(full.getAttribute("data-state")).toBe("unchecked");

    // 叶子勾选即时生效
    await user.click(full);
    expect(full.getAttribute("data-state")).toBe("checked");
    expect(group.getAttribute("data-state")).toBe("checked");

    // 组头全开态点一下全关
    await user.click(group);
    expect(half.getAttribute("data-state")).toBe("unchecked");
    expect(full.getAttribute("data-state")).toBe("unchecked");
  });

  it("标点闭合模式显示范围单选与成对分组，叶子勾选即时生效", async () => {
    const user = userEvent.setup();
    render(FilterPanel, {
      props: { ...baseProps, kind: "pairs", pairScope: "Chapter", pairPresets: ["Round"] },
    });

    // 范围单选默认跨行（英文 locale）
    const lineScope = screen.getByRole("radio", { name: "Per line" });
    const chapterScope = screen.getByRole("radio", { name: "Cross line" });
    expect(chapterScope.getAttribute("data-state")).toBe("checked");
    expect(lineScope.getAttribute("data-state")).toBe("unchecked");
    await user.click(lineScope);
    expect(lineScope.getAttribute("data-state")).toBe("checked");

    const group = screen.getByRole("checkbox", { name: "Round brackets" });
    const round = screen.getByRole("checkbox", { name: "Round pairs" });
    const square = screen.getByRole("checkbox", { name: "Square pairs" });
    const brace = screen.getByRole("checkbox", { name: "Brace pairs" });
    expect(group.getAttribute("data-state")).toBe("indeterminate");
    expect(round.getAttribute("data-state")).toBe("checked");
    expect(square.getAttribute("data-state")).toBe("unchecked");

    // 叶子勾选即时生效
    await user.click(square);
    expect(square.getAttribute("data-state")).toBe("checked");
    expect(group.getAttribute("data-state")).toBe("indeterminate");
    await user.click(brace);
    expect(brace.getAttribute("data-state")).toBe("checked");
    expect(group.getAttribute("data-state")).toBe("checked");

    // 组头全开态点一下全关
    await user.click(group);
    expect(round.getAttribute("data-state")).toBe("unchecked");
    expect(square.getAttribute("data-state")).toBe("unchecked");
    expect(brace.getAttribute("data-state")).toBe("unchecked");
  });
});
