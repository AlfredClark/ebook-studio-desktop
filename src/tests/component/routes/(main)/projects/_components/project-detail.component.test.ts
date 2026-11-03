import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import type { ProjectInfo } from "../../../../../../libs/commands/types";
import ProjectDetail from "../../../../../../routes/(main)/projects/_components/project-detail.svelte";

const getOverviewMock = vi.hoisted(() => vi.fn());

vi.mock("#libs/commands", () => ({
  default: { getProjectOverview: getOverviewMock },
}));

vi.mock("@tauri-apps/api/core", () => ({
  convertFileSrc: (path: string) => `https://asset.localhost/${path}`,
}));

type FakeResult =
  { status: "ok"; data: unknown } | { status: "error"; error: { kind: string; message: string } };

/** 与 `EnhancedCommand` 行为一致的替身：只实现本组件用到的 `result()` */
function fakeCommand(result: FakeResult): { result: () => Promise<FakeResult> } {
  return { result: () => Promise.resolve(result) };
}

const project: ProjectInfo = {
  id: "abc-uuid",
  name: "雾镇",
  type: "novel",
  created_at: "2026-01-01T00:00:00Z",
  updated_at: "2026-01-02T00:00:00Z",
};

beforeEach(() => {
  vi.clearAllMocks();
});

afterEach(() => {
  cleanup();
});

describe("项目详情", () => {
  it("无选中走空态", () => {
    render(ProjectDetail, { props: { project: undefined } });

    expect(screen.getByText("Select a project to view details")).not.toBeNull();
    expect(getOverviewMock).not.toHaveBeenCalled();
  });

  it("有元数据时展示封面与著录，附加属性默认折叠", async () => {
    const user = userEvent.setup();
    getOverviewMock.mockReturnValue(
      fakeCommand({
        status: "ok",
        data: {
          metadata: {
            identifier: "abc-uuid",
            title: "雾镇·卷一",
            creator: "作者",
            contributor: "",
            language: "zh-CN",
            publisher: "某社",
            description: "",
            subjects: [],
            date_published: "2026-01-01",
            rights: "",
            series: "",
            volume: "",
            source: "",
            cover: "sources/cover.png",
            schema_version: 1,
            updated_at: "2026-01-02T00:00:00Z",
          },
          cover_path: "/data/covers/cover.png",
        },
      }),
    );
    render(ProjectDetail, { props: { project } });

    await screen.findByText("雾镇·卷一");
    expect(screen.getByText("作者")).not.toBeNull();
    const trigger = screen.getByRole("button", { name: "More info" });
    expect(trigger.getAttribute("aria-expanded")).toBe("false");

    await user.click(trigger);

    expect(trigger.getAttribute("aria-expanded")).toBe("true");
    expect(await screen.findByText("某社")).not.toBeNull();
    const img = screen.getByAltText("Cover preview") as HTMLImageElement;
    expect(img.src).toContain("asset.localhost");
  });

  it("无元数据回落默认封面与创建时间", async () => {
    getOverviewMock.mockReturnValue(
      fakeCommand({ status: "ok", data: { metadata: null, cover_path: null } }),
    );
    render(ProjectDetail, { props: { project } });

    // 标题同时出现在标题栏与默认封面框内
    expect(screen.getAllByText("雾镇")).toHaveLength(2);
    expect(screen.getByText("Created")).not.toBeNull();
    expect(screen.queryByAltText("Cover preview")).toBeNull();
  });
});
