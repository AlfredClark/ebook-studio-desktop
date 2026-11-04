import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import type { NovelTextStatus } from "../../../../../../../../libs/commands/types";
import TxtSourceCard from "../../../../../../../../routes/(projects)/novel/[uuid]/files/_components/txt-source-card.svelte";

// 哑组件：只断展示与回调，不调后端命令。
const chooseMock = vi.hoisted(() => vi.fn());

const emptyStatus: NovelTextStatus = { exists: false, size_bytes: 0, updated_at: null };
const filledStatus: NovelTextStatus = {
  exists: true,
  size_bytes: 2048,
  updated_at: "2026-02-01T00:00:00Z",
};

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

describe("原文卡片", () => {
  it("缺文件时显示空态与选择按钮", () => {
    render(TxtSourceCard, {
      props: { status: emptyStatus, importing: false, onChoose: chooseMock },
    });

    expect(screen.getByText("No source text imported yet")).not.toBeNull();
    expect(screen.getByRole("button", { name: "Choose TXT…" })).not.toBeNull();
  });

  it("已导入时显示大小与更新时间", () => {
    render(TxtSourceCard, {
      props: { status: filledStatus, importing: false, onChoose: chooseMock },
    });

    expect(screen.getByText("Size")).not.toBeNull();
    expect(screen.getByText("2.0 KB")).not.toBeNull();
    expect(screen.getByRole("button", { name: "Choose again…" })).not.toBeNull();
  });

  it("导入中禁用按钮，点击走回调", async () => {
    const user = userEvent.setup();
    render(TxtSourceCard, {
      props: { status: emptyStatus, importing: true, onChoose: chooseMock },
    });

    expect((screen.getByRole("button", { name: "Importing…" }) as HTMLButtonElement).disabled).toBe(
      true,
    );
    await user.click(screen.getByRole("button", { name: "Importing…" }));
    expect(chooseMock).not.toHaveBeenCalled();
  });

  it("空闲点击触发选择回调", async () => {
    const user = userEvent.setup();
    render(TxtSourceCard, {
      props: { status: emptyStatus, importing: false, onChoose: chooseMock },
    });

    await user.click(screen.getByRole("button", { name: "Choose TXT…" }));
    expect(chooseMock).toHaveBeenCalledOnce();
  });
});
