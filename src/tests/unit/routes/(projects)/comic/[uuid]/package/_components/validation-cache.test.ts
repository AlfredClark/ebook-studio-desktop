import { describe, expect, it } from "vitest";
import {
  __clearValidationCacheForTests,
  readCachedValidation,
  writeCachedValidation,
} from "../../../../../../../../routes/(projects)/comic/[uuid]/package/_components/validation-cache";
import type { PackageInfo, ValidationOutcome } from "#libs/commands/types";

function pkgWith(updatedAt: string): PackageInfo {
  return { file_name: "雾镇.epub", size_bytes: 1024, updated_at: updatedAt };
}

function outcomeWith(valid: boolean): ValidationOutcome {
  return { valid, messages: [] };
}

describe("漫画打包校验会话缓存", () => {
  it("写后命中、指纹变化 miss", () => {
    __clearValidationCacheForTests();
    const pkg = pkgWith("2026-11-05T00:00:00Z");
    expect(readCachedValidation("uuid-1", pkg)).toBeNull();
    writeCachedValidation("uuid-1", pkg, outcomeWith(true));
    expect(readCachedValidation("uuid-1", pkg)).toEqual(outcomeWith(true));
    // 重新打包（时间变）指纹即变。
    expect(readCachedValidation("uuid-1", pkgWith("2026-11-05T00:00:01Z"))).toBeNull();
    // 不同项目隔离。
    expect(readCachedValidation("uuid-2", pkg)).toBeNull();
  });

  it("超限淘汰最旧", () => {
    __clearValidationCacheForTests();
    for (let index = 0; index < 25; index += 1) {
      const pkg = pkgWith(`2026-11-05T00:00:${String(index).padStart(2, "0")}Z`);
      writeCachedValidation("uuid-1", pkg, outcomeWith(index % 2 === 0));
    }
    expect(readCachedValidation("uuid-1", pkgWith("2026-11-05T00:00:00Z"))).toBeNull();
    expect(readCachedValidation("uuid-1", pkgWith("2026-11-05T00:00:24Z"))).toEqual(
      outcomeWith(true),
    );
  });
});
