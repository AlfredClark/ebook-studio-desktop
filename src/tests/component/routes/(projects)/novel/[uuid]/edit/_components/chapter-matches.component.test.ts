import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import type { LineMatch } from "../../../../../../../../libs/commands/types";
import ChapterMatches from "../../../../../../../../routes/(projects)/novel/[uuid]/edit/_components/chapter-matches.svelte";

const jumpMock = vi.hoisted(() => vi.fn());

const matches: LineMatch[] = [
  { line: 2, spans: [{ start: 0, len: 2 }] },
  { line: 3, spans: [{ start: 1, len: 1 }] },
];

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

describe("命中列表", () => {
  it("渲染行号与正文预览，点击跳到对应行", async () => {
    const user = userEvent.setup();
    render(ChapterMatches, {
      props: { matches, content: "第一行\n正文甲\n正文乙丙", stale: false, onJump: jumpMock },
    });

    expect(screen.getByText("Matches in this chapter (2)")).not.toBeNull();
    expect(screen.getByText("正文甲")).not.toBeNull();
    expect(screen.getByText("Line 2")).not.toBeNull();

    await user.click(screen.getByText("正文甲"));
    expect(jumpMock).toHaveBeenCalledOnce();
    expect(jumpMock).toHaveBeenCalledWith(2);
  });

  it("过期时挂基于落盘版本的提示", () => {
    render(ChapterMatches, {
      props: { matches, content: "第一行\n正文甲\n正文乙丙", stale: true, onJump: jumpMock },
    });

    expect(screen.getByText("Text edited, results are based on the saved version")).not.toBeNull();
  });
});
