import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import FilterPanel from "../../../../../../../../routes/(projects)/novel/[uuid]/edit/_components/filter-panel.svelte";

const runMock = vi.hoisted(() => vi.fn());
const clearMock = vi.hoisted(() => vi.fn());

const baseProps = {
  kind: "keyword" as const,
  keyword: "",
  caseSensitive: false,
  pattern: "",
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
});
