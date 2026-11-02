#!/usr/bin/env node
// 版本号统一升级：以 package.json 的 version 为唯一真值来源，同步写入
// tauri.conf.json 与 src-tauri/Cargo.toml（与 pnpm release 的 bumpp 三件套保持一致）。
// 本脚本只改文件，不提交、不打 tag；发版提交仍走 pnpm release。
// 用法：node scripts/bump-version.ts <patch|minor|major|x.y.z>
// 注意：仅使用可擦除的类型语法，Node 24 可直接运行，无需编译。
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root: string = join(dirname(fileURLToPath(import.meta.url)), "..");

/** 需要同步版本号的文件：package.json 必须排第一（真值来源） */
const VERSION_FILES: readonly string[] = [
  "package.json",
  "src-tauri/tauri.conf.json",
  "src-tauri/Cargo.toml",
];

interface VersionFile {
  filename: string;
  content: string;
}

/** 从文件内容中提取版本号；三种文件格式各异，逐个匹配 */
function extractVersion(filename: string, content: string): string {
  const pattern =
    filename === "src-tauri/Cargo.toml" ? /^version = "([^"]+)"/m : /"version": "([^"]+)"/;
  const matched: RegExpMatchArray | null = content.match(pattern);
  if (!matched) throw new Error(`在 ${filename} 中找不到版本号字段`);
  return matched[1];
}

/** 按等级递增纯数字 semver（x.y.z），预发布后缀不支持，有需要走 pnpm release */
function bumpVersion(version: string, level: "patch" | "minor" | "major"): string {
  const matched: RegExpMatchArray | null = version.match(/^(\d+)\.(\d+)\.(\d+)$/);
  if (!matched) throw new Error(`当前版本 ${version} 非纯数字 semver，请手动处理`);
  let [major, minor, patch]: number[] = matched.slice(1).map(Number);
  if (level === "major") {
    major += 1;
    minor = 0;
    patch = 0;
  } else if (level === "minor") {
    minor += 1;
    patch = 0;
  } else {
    patch += 1;
  }
  return `${major}.${minor}.${patch}`;
}

/** 解析命令行参数：升级等级或显式指定的目标版本号 */
function resolveTarget(current: string, arg: string | undefined): string {
  if (arg === "patch" || arg === "minor" || arg === "major") return bumpVersion(current, arg);
  if (arg !== undefined && /^\d+\.\d+\.\d+$/.test(arg)) {
    if (arg === current) throw new Error(`目标版本 ${arg} 与当前版本一致，无需升级`);
    return arg;
  }
  throw new Error(`未知参数 ${arg}，仅支持 patch/minor/major 或显式版本号 x.y.z`);
}

/** 原地替换版本号，保持原文件缩进与换行不动 */
function replaceVersion(filename: string, content: string, from: string, to: string): string {
  if (filename === "src-tauri/Cargo.toml") {
    return content.replace(/^version = "[^"]+"/m, `version = "${to}"`);
  }
  return content.replace(`"version": "${from}"`, `"version": "${to}"`);
}

const contents: VersionFile[] = VERSION_FILES.map((filename) => ({
  filename,
  content: readFileSync(join(root, filename), "utf8"),
}));

// 三处当前版本必须一致，否则说明有人手改漏了，先报错停下
const versions: Set<string> = new Set(
  contents.map(({ filename, content }) => extractVersion(filename, content)),
);
if (versions.size !== 1) {
  throw new Error(`三处版本号不一致：${[...versions].join(", ")}，请先手动对齐`);
}
const from: string = [...versions][0];
const target: string = resolveTarget(from, process.argv[2]);

for (const { filename, content } of contents) {
  writeFileSync(join(root, filename), replaceVersion(filename, content, from, target));
}
console.log(`版本已同步：${from} → ${target}`);
