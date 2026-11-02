export default {
  // 1. 前端脚本与组件：Prettier 排版后由 ESLint 校验/自动修复
  "**/*.{js,ts,svelte}": ["prettier --write", "eslint --fix"],

  // 2. 静态资源、样式与 YAML：仅 Prettier 排版
  //    （CI 的 `prettier --check .` 覆盖 .github/**/*.yml 等文件，本地钩子必须保持同一范围）
  "**/*.{json,md,html,css,yml,yaml}": ["prettier --write"],

  // 3. Rust 源码：函数式配置避免参数被追加到 clippy 导致多输入文件名报错
  "**/*.rs": (filenames: string[]) => {
    // 过滤出文件路径，并拼接给 cargo fmt
    const formattedFiles = filenames.join(" ");

    return [
      `cargo fmt --manifest-path Cargo.toml -- ${formattedFiles}`,
      // clippy 作为整工作区检查，必须作为独立命令返回，不得拼接文件名
      "cargo clippy --manifest-path Cargo.toml --workspace --all-targets -- -D warnings",
    ];
  },
};
