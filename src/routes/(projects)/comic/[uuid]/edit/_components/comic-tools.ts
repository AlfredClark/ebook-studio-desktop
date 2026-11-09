// 漫画编辑页图片工具选择：切页返回时，有在跑的长时间作业则直接打开对应工具。
// 各作业经项目锁互斥，同项目至多其一在跑；顺序仅防万一。
import type { ComicTool } from "./comic-tool-panel.svelte";

/** 各工具是否有作业在跑（调用方读 hook 快照传入）。 */
export type RunningJobs = {
  convert: boolean;
  compress: boolean;
  resize: boolean;
};

/** 有在跑作业时返回对应工具，否则返回空（调用方保持当前选择）。 */
export function toolWithRunningJob(jobs: RunningJobs): ComicTool | null {
  if (jobs.convert) {
    return "convert";
  }
  if (jobs.compress) {
    return "compress";
  }
  if (jobs.resize) {
    return "resize";
  }
  return null;
}
