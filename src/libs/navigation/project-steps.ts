// 项目步骤注册表：slug 与 `(projects)` 路由段一一对应，数组顺序即面包屑顺序。
import { m } from "#libs/i18n/paraglide/messages";

/** 步骤清单，顺序即渲染顺序；`as const` 保留 slug 字面量类型，注册错路由会在编译期报错 */
export const PROJECT_STEPS = [
  { slug: "metadata", label: m.project_step_metadata },
  { slug: "files", label: m.project_step_files },
  { slug: "edit", label: m.project_step_edit },
  { slug: "build", label: m.project_step_build },
  { slug: "package", label: m.project_step_package },
  { slug: "export", label: m.project_step_export },
] as const;

/** 单个步骤 */
export type ProjectStep = (typeof PROJECT_STEPS)[number];

/** 已登记的步骤 slug */
export type ProjectStepSlug = ProjectStep["slug"];

/** 按路径末段匹配当前步骤；未命中（如未知路径）回落首步 */
export function resolveProjectStep(pathname: string): ProjectStep {
  const slug = pathname.split("/").filter(Boolean).at(-1);
  return PROJECT_STEPS.find((step) => step.slug === slug) ?? PROJECT_STEPS[0];
}
