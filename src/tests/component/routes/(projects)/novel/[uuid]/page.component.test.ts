import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import NovelProjectPage from "../../../../../../routes/(projects)/novel/[uuid]/+page.svelte";

// 裸项目路由守卫：挂载即重定向到默认第一步，不留历史。
const gotoMock = vi.hoisted(() => vi.fn());

vi.mock("$app/navigation", () => ({ goto: gotoMock }));

vi.mock("$app/state", () => ({
  page: {
    url: new URL("http://localhost/novel/abc-uuid"),
    params: { uuid: "abc-uuid" },
  },
}));

afterEach(() => {
  cleanup();
});

describe("裸项目路由", () => {
  it("挂载即重定向到元数据步骤", () => {
    render(NovelProjectPage);

    expect(gotoMock).toHaveBeenCalledWith("/novel/abc-uuid/metadata", { replaceState: true });
  });
});
