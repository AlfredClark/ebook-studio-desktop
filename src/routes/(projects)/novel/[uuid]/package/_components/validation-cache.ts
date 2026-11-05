// 打包校验会话缓存：指纹 → 校验结果；SPA 模块单例常驻，重载即清空。
// 缓存的是“对该 dist 文件的结论”：文件不变则结论恒有效，重新打包指纹即变。
import type { PackageInfo, ValidationOutcome } from "#libs/commands/types";

/** 缓存上限：超限删最旧，纯卫生项。 */
const MAX_ENTRIES = 20;

const cache = new Map<string, ValidationOutcome>();

/** 缓存键：项目 + 文件名 + 大小 + 打包时间。 */
function fingerprint(uuid: string, pkg: PackageInfo): string {
  return `${uuid}/${pkg.file_name}/${pkg.size_bytes}/${pkg.updated_at}`;
}

/** 读缓存：未命中回 `null`，调用方走后端校验。 */
export function readCachedValidation(uuid: string, pkg: PackageInfo): ValidationOutcome | null {
  return cache.get(fingerprint(uuid, pkg)) ?? null;
}

/** 写缓存：打包与校验成功后调用。 */
export function writeCachedValidation(
  uuid: string,
  pkg: PackageInfo,
  validation: ValidationOutcome,
): void {
  cache.set(fingerprint(uuid, pkg), validation);
  while (cache.size > MAX_ENTRIES) {
    const oldest = cache.keys().next();
    if (oldest.done) {
      break;
    }
    cache.delete(oldest.value);
  }
}

/** 清空缓存：仅测试用，生产会话重置靠模块重载。 */
export function __clearValidationCacheForTests(): void {
  cache.clear();
}
