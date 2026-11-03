import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import type { ProjectMetadata } from "../../../../../libs/commands/types";
import MetadataForm from "../../../../../routes/(projects)/_components/metadata-form.svelte";

// 表单纯展示：不调后端命令，回调 mock 即可覆盖提交路径。
const saveMock = vi.hoisted(() => vi.fn());
const chooseMock = vi.hoisted(() => vi.fn());
const resetMock = vi.hoisted(() => vi.fn());

const baseMetadata: ProjectMetadata = {
  schema_version: 1,
  identifier: "abc-uuid",
  title: "雾镇",
  alternative: "",
  creator: "作者",
  contributor: "",
  language: "zh-CN",
  publisher: "",
  description: "",
  subjects: ["悬疑"],
  date_published: null,
  rights: "",
  series: "",
  volume: "",
  source: "",
  cover: null,
  updated_at: "2026-01-01T00:00:00Z",
};

function renderForm(metadata: ProjectMetadata = baseMetadata, projectType: "novel" | "comic") {
  render(MetadataForm, {
    props: {
      metadata,
      coverUrl: null,
      projectType,
      saving: false,
      importing: false,
      onSave: saveMock,
      onChooseCover: chooseMock,
      onReset: resetMock,
    },
  });
}

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

describe("元数据表单", () => {
  it("回显传入的字段值", async () => {
    renderForm(baseMetadata, "comic");

    expect((screen.getByLabelText("Identifier") as HTMLInputElement).value).toBe("abc-uuid");
    expect((screen.getByLabelText("Title") as HTMLInputElement).value).toBe("雾镇");
    expect((screen.getByLabelText("Subjects") as HTMLTextAreaElement).value).toBe("悬疑");
  });

  it("小说锁定语言输入", () => {
    renderForm(baseMetadata, "novel");

    expect((screen.getByLabelText("Language") as HTMLInputElement).disabled).toBe(true);
  });

  it("漫画语言可编辑", () => {
    renderForm(baseMetadata, "comic");

    expect((screen.getByLabelText("Language") as HTMLInputElement).disabled).toBe(false);
  });

  it("提交时回传解析后的主题词", async () => {
    const user = userEvent.setup();
    renderForm(baseMetadata, "comic");

    const subjects = screen.getByLabelText("Subjects");
    await user.clear(subjects);
    await user.type(subjects, "悬疑，推理");
    await user.click(screen.getByRole("button", { name: "Save" }));

    expect(saveMock).toHaveBeenCalledOnce();
    expect(saveMock.mock.calls[0][0].subjects).toEqual(["悬疑", "推理"]);
  });

  it("重置走确认回调而非直接回填", async () => {
    const user = userEvent.setup();
    renderForm(baseMetadata, "comic");

    const title = screen.getByLabelText("Title");
    await user.clear(title);
    await user.click(screen.getByRole("button", { name: "Reset" }));

    expect(resetMock).toHaveBeenCalledOnce();
    expect(saveMock).not.toHaveBeenCalled();
  });

  it("空标题禁用保存", async () => {
    const user = userEvent.setup();
    renderForm({ ...baseMetadata, title: "雾镇" }, "comic");

    await user.clear(screen.getByLabelText("Title"));

    expect((screen.getByRole("button", { name: "Save" }) as HTMLButtonElement).disabled).toBe(true);
    expect(saveMock).not.toHaveBeenCalled();
  });
});
