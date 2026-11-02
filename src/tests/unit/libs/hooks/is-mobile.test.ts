import { describe, expect, it } from "vitest";
import { MediaQuery } from "svelte/reactivity";
import { getSharedIsMobile, IsMobile } from "#libs/hooks/is-mobile.svelte";

// 单测跑 node 环境，`svelte/reactivity` 解析到服务端实现：
// 构造器不触碰 `window.matchMedia`，`current` 恒为回落值。
// 本文件只锁定 SSR 安全与构造契约，真实断点行为由浏览器覆盖。
describe("IsMobile", () => {
  it("继承服务端 MediaQuery，构造不抛错", () => {
    const mobile = new IsMobile();

    expect(mobile).toBeInstanceOf(MediaQuery);
  });

  it("非浏览器环境回落为假，不阻断服务端渲染", () => {
    expect(new IsMobile().current).toBe(false);
  });

  it("接受自定义断点且行为与默认一致", () => {
    const custom = new IsMobile(1024);

    expect(custom).toBeInstanceOf(IsMobile);
    expect(custom.current).toBe(false);
  });

  it("共享单例复用同一实例", () => {
    expect(getSharedIsMobile()).toBe(getSharedIsMobile());
  });
});
