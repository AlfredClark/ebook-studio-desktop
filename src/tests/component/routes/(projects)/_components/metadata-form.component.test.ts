import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import type { ProjectMetadata } from "../../../../../libs/commands/types";
import MetadataForm from "../../../../../routes/(projects)/_components/metadata-form.svelte";

// jsdom 缺少浮动定位所需的浏览器 API，bits-ui 弹窗用得到，仅在本文件内就地补齐。
if (!Element.prototype.hasPointerCapture) {
  Element.prototype.hasPointerCapture = () => false;
  Element.prototype.setPointerCapture = () => {};
  Element.prototype.releasePointerCapture = () => {};
}
if (!Element.prototype.scrollIntoView) {
  Element.prototype.scrollIntoView = () => {};
}
if (typeof ResizeObserver === "undefined") {
  globalThis.ResizeObserver = class {
    observe(): void {}
    unobserve(): void {}
    disconnect(): void {}
  } as unknown as typeof ResizeObserver;
}

// 表单纯展示：不调后端命令，回调 mock 即可覆盖提交路径。
const saveMock = vi.hoisted(() => vi.fn());
const chooseMock = vi.hoisted(() => vi.fn());
const resetMock = vi.hoisted(() => vi.fn());

const baseMetadata: ProjectMetadata = {
  schema_version: 2,
  identifier: "abc-uuid",
  title: "雾镇",
  alternative: "",
  creator: "作者",
  contributor: "",
  language: "zh-CN",
  publisher: "",
  description: [],
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

  it("简介回显为换行文本且提交时去空去首尾空白", async () => {
    const user = userEvent.setup();
    renderForm({ ...baseMetadata, description: ["第一行", "第二行"] }, "comic");

    const description = screen.getByLabelText("Description") as HTMLTextAreaElement;
    expect(description.value).toBe("第一行\n第二行");

    await user.clear(description);
    await user.type(description, "  第一行  {enter}{enter}  第二行  ");
    await user.click(screen.getByRole("button", { name: "Save" }));

    expect(saveMock).toHaveBeenCalledOnce();
    expect(saveMock.mock.calls[0][0].description).toEqual(["第一行", "第二行"]);
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

  it("输入作者名后版权自动填充", async () => {
    const user = userEvent.setup();
    renderForm({ ...baseMetadata, creator: "", rights: "" }, "comic");

    await user.type(screen.getByLabelText("Creator"), "雾镇");

    expect((screen.getByLabelText("Rights") as HTMLInputElement).value).toBe(
      "Copyright © 雾镇. All rights reserved.",
    );
  });

  it("手动修改过的版权不受作者联动覆盖", async () => {
    const user = userEvent.setup();
    renderForm({ ...baseMetadata, creator: "", rights: "" }, "comic");

    const creator = screen.getByLabelText("Creator");
    const rights = screen.getByLabelText("Rights") as HTMLInputElement;
    await user.type(creator, "雾镇");
    expect(rights.value).toBe("Copyright © 雾镇. All rights reserved.");

    await user.clear(rights);
    await user.type(rights, "自定义版权");
    await user.type(creator, "甲");

    expect(rights.value).toBe("自定义版权");
  });

  it("空标题禁用保存", async () => {
    const user = userEvent.setup();
    renderForm({ ...baseMetadata, title: "雾镇" }, "comic");

    await user.clear(screen.getByLabelText("Title"));

    expect((screen.getByRole("button", { name: "Save" }) as HTMLButtonElement).disabled).toBe(true);
    expect(saveMock).not.toHaveBeenCalled();
  });

  it("日历选日期后触发器展示并随保存提交，同步回填不回滚用户选择", async () => {
    // 回归：同步 effect 曾订阅本地 datePublished，选中即被回滚为空，表现为"选择后无变化"。
    const user = userEvent.setup();
    renderForm({ ...baseMetadata, date_published: null }, "novel");

    // 触发器经 label 关联命名（内容文案是占位符，读不到选中值）。
    await fireEvent.click(screen.getByRole("button", { name: "Publication date" }));
    let day15: HTMLElement | undefined;
    await waitFor(
      () => {
        const days = [...document.querySelectorAll("div[data-bits-day]")];
        day15 = days.find((el) => el.textContent?.trim() === "15") as HTMLElement | undefined;
        expect(day15 != null).toBe(true);
      },
      { timeout: 8000 },
    );
    await user.click(day15 as HTMLElement);

    // 选中即落本地态：清除按钮出现，提交携带 YYYY-MM-15。
    await waitFor(
      () => {
        expect(screen.queryByRole("button", { name: "Clear date" })).not.toBeNull();
      },
      { timeout: 8000 },
    );
    // 清除按钮叠在输入框内部：与触发器同属一个 relative 容器，而非并排兄弟。
    const trigger = screen.getByRole("button", { name: "Publication date" });
    const clear = screen.getByRole("button", { name: "Clear date" });
    expect(clear.closest(".relative")).not.toBeNull();
    expect(clear.closest(".relative")).toBe(trigger.closest(".relative"));
    await user.click(screen.getByRole("button", { name: "Save" }));
    expect(saveMock).toHaveBeenCalledOnce();
    const submitted = saveMock.mock.calls[0][0] as ProjectMetadata;
    expect(submitted.date_published).toMatch(/^\d{4}-\d{2}-15$/);
  });
});
