import { afterEach, describe, expect, it } from "vitest";
import { cleanup, render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import ParseParamsCard from "../../../../../../../../routes/(projects)/novel/[uuid]/files/_components/parse-params-card.svelte";
import {
  DEFAULT_CHAPTER_PATTERN,
  DEFAULT_VOLUME_PATTERN,
} from "../../../../../../../../routes/(projects)/novel/[uuid]/files/_components/novel-text-params";

afterEach(() => {
  cleanup();
});

describe("分卷分章参数卡", () => {
  it("回显传入的表达式", () => {
    render(ParseParamsCard, {
      props: { volumePattern: DEFAULT_VOLUME_PATTERN, chapterPattern: DEFAULT_CHAPTER_PATTERN },
    });

    expect((screen.getByLabelText("Volume title regex") as HTMLInputElement).value).toBe(
      DEFAULT_VOLUME_PATTERN,
    );
    expect((screen.getByLabelText("Chapter title regex") as HTMLInputElement).value).toBe(
      DEFAULT_CHAPTER_PATTERN,
    );
  });

  it("输入框可编辑", async () => {
    const user = userEvent.setup();
    render(ParseParamsCard, {
      props: { volumePattern: DEFAULT_VOLUME_PATTERN, chapterPattern: DEFAULT_CHAPTER_PATTERN },
    });

    const volume = screen.getByLabelText("Volume title regex");
    await user.clear(volume);
    await user.type(volume, "^卷 (.+)$");

    expect((volume as HTMLInputElement).value).toBe("^卷 (.+)$");
  });

  it("坏正则显示无效提示", async () => {
    const user = userEvent.setup();
    render(ParseParamsCard, {
      props: { volumePattern: DEFAULT_VOLUME_PATTERN, chapterPattern: DEFAULT_CHAPTER_PATTERN },
    });

    expect(screen.queryByText("Invalid regex, please check the syntax")).toBeNull();

    const volume = screen.getByLabelText("Volume title regex");
    await user.clear(volume);
    await user.type(volume, "^(unclosed");

    expect(screen.getByText("Invalid regex, please check the syntax")).not.toBeNull();
  });
});
