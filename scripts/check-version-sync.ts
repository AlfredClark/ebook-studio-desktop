#!/usr/bin/env node
// 成对维护检查（三项，见 docs/ARCHITECTURE.md 的「成对维护的配置」）：
// 1. `.prettierrc` 的 `importOrderTypeScriptVersion` 必须与 `package.json` 的 `typescript` 版本一致；
// 2. Tauri 前后端插件的 major.minor 必须一致（`pnpm-lock.yaml` 对 `Cargo.lock`）；
// 3. 路径别名的权威定义（`package.json` 的 `imports`）与 TS 侧镜像（`tsconfig.aliases.json` 的 `paths`）必须逐条一致。
// 由 `pnpm check` 自动执行，失配即红灯，避免单边修改后的静默漂移。
// 本脚本只读文件、不写文件；退出码非零即失败。
// 注意：仅使用可擦除的类型语法，Node 24 可直接运行，无需编译。
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root: string = join(dirname(fileURLToPath(import.meta.url)), "..");

function fail(message: string): never {
  console.error(`[check:sync] ${message}`);
  process.exit(1);
}

const prettierrc: { importOrderTypeScriptVersion?: unknown } = JSON.parse(
  readFileSync(join(root, ".prettierrc"), "utf8"),
);
const pkg: { devDependencies?: Record<string, string>; imports?: Record<string, unknown> } =
  JSON.parse(readFileSync(join(root, "package.json"), "utf8"));

const prettierTs = prettierrc.importOrderTypeScriptVersion;
if (typeof prettierTs !== "string" || prettierTs.length === 0) {
  fail("在 .prettierrc 中找不到 importOrderTypeScriptVersion 字段");
}
const pkgTs = pkg.devDependencies?.typescript?.replace(/^[~^>=< ]+/, "");
if (typeof pkgTs !== "string" || pkgTs.length === 0) {
  fail("在 package.json 的 devDependencies 中找不到 typescript 字段");
}
if (prettierTs !== pkgTs) {
  fail(
    `版本漂移：.prettierrc 为 ${String(prettierTs)}，package.json 为 ${String(pkgTs)}，请同步两者`,
  );
}
console.log(`[check:sync] typescript 对齐通过：${String(prettierTs)}`);

checkTauriPluginPairs();
console.log("[check:sync] Tauri 前后端插件对齐通过");

checkAliasPairs();
console.log("[check:sync] 路径别名对齐通过");

/** Npm 包名映射到 cargo 包名；返回 `null` 表示无 Rust 对应端（如 CLI），跳过 */
function toCargoName(npmName: string): string | null {
  if (npmName === "@tauri-apps/api") return "tauri";
  const plugin = npmName.match(/^@tauri-apps\/(plugin-[a-z-]+)$/);
  if (plugin) return `tauri-${plugin[1]}`;
  const standalone = npmName.match(/^(tauri-plugin-[a-z-]+)-api$/);
  if (standalone) return standalone[1];
  return null;
}

/** 取 `major.minor`，供 Tauri 的跨端兼容口径比对（patch 允许不同） */
function majorMinor(version: string): string {
  return version.split(".").slice(0, 2).join(".");
}

/**
 * Tauri 前后端插件对齐：JS 侧读 `pnpm-lock.yaml` 根 importer 的已解析版本， Rust 侧读 `Cargo.lock`
 * 的锁定版本，逐对比较 major.minor。 无 JS 封装的纯后端插件（如 store / autostart）天然只有 Rust 端，直接跳过。
 */
function checkTauriPluginPairs(): void {
  const lock = readFileSync(join(root, "pnpm-lock.yaml"), "utf8");
  const cargoLock = readFileSync(join(root, "Cargo.lock"), "utf8");

  const npmVersions = new Map<string, string>();
  const npmPattern =
    /'((?:@tauri-apps\/(?:api|plugin-[a-z-]+))|(?:tauri-plugin-[a-z-]+-api))':\s*\n\s*specifier: [^\n]*\n\s*version: (\d+\.\d+\.\d+)/g;
  for (const match of lock.matchAll(npmPattern)) {
    npmVersions.set(match[1], match[2]);
  }

  const cargoVersions = new Map<string, string>();
  for (const block of cargoLock.split("[[package]]")) {
    const name = block.match(/\nname = "([^"]+)"/)?.[1];
    const version = block.match(/\nversion = "(\d+\.\d+\.\d+)"/)?.[1];
    if (name && version && !cargoVersions.has(name)) cargoVersions.set(name, version);
  }

  const mismatches: string[] = [];
  for (const [npmName, npmVersion] of npmVersions) {
    const cargoName = toCargoName(npmName);
    if (!cargoName) continue;
    const cargoVersion = cargoVersions.get(cargoName);
    if (!cargoVersion) {
      mismatches.push(`${npmName} 只在 JS 侧声明（${npmVersion}），Cargo.toml 缺少 ${cargoName}`);
      continue;
    }
    if (majorMinor(npmVersion) !== majorMinor(cargoVersion)) {
      mismatches.push(
        `${npmName}（${npmVersion}）与 ${cargoName}（${cargoVersion}）的 major.minor 不一致`,
      );
    }
  }
  if (mismatches.length > 0) {
    fail(
      `Tauri 前后端插件版本漂移：\n  - ${mismatches.join("\n  - ")}\n请两边一起升：pnpm update <npm 包> + cargo update -p <cargo 包>`,
    );
  }
}

/**
 * 路径别名双轨一致性：`package.json` 的 `imports` 是运行时 / 构建的权威定义（Node subpath imports，
 * Vite 原生解析），`tsconfig.aliases.json` 的 `paths` 是 tsc / svelte-check 的只读镜像。 TS 对
 * imports 映射目标按 Node 语义不做隐式扩展名补全，无后缀导入必须经 `paths` 兜底； 两份文件同在仓库根，映射写法逐条直接比较。
 */
function checkAliasPairs(): void {
  const imports = pkg.imports ?? {};
  const aliasConfig: { compilerOptions?: { paths?: Record<string, unknown> } } = JSON.parse(
    readFileSync(join(root, "tsconfig.aliases.json"), "utf8"),
  );
  const paths = aliasConfig.compilerOptions?.paths ?? {};

  const aliasKeys = new Set([
    ...Object.keys(imports).filter((key) => key.startsWith("#")),
    ...Object.keys(paths).filter((key) => key.startsWith("#")),
  ]);

  const mismatches: string[] = [];
  for (const key of Object.keys(paths)) {
    if (!key.startsWith("#")) {
      mismatches.push(`tsconfig.aliases.json 出现非 # 别名的 paths 条目：${key}`);
    }
  }
  for (const key of aliasKeys) {
    const importTarget = imports[key];
    const pathTargets = paths[key];
    if (typeof importTarget !== "string") {
      mismatches.push(`${key} 缺少 package.json 的 imports 定义`);
      continue;
    }
    if (
      !Array.isArray(pathTargets) ||
      pathTargets.length !== 1 ||
      typeof pathTargets[0] !== "string"
    ) {
      mismatches.push(`${key} 在 tsconfig.aliases.json 的 paths 中缺少唯一映射`);
      continue;
    }
    if (importTarget !== pathTargets[0]) {
      mismatches.push(`${key} 映射不一致：imports 为 ${importTarget}，paths 为 ${pathTargets[0]}`);
    }
  }
  if (mismatches.length > 0) {
    fail(
      `路径别名漂移（请同步 package.json 的 imports 与 tsconfig.aliases.json 的 paths）：\n  - ${mismatches.join("\n  - ")}`,
    );
  }
}
