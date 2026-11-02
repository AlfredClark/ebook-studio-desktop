import type { UserConfig } from "@commitlint/types";

const Configuration: UserConfig = {
  extends: ["@commitlint/config-conventional"],
  rules: {
    // 对应 cliff.toml 中 commit_parsers 解析的提交类型
    "type-enum": [
      2,
      "always",
      [
        "feat", // 🚀 Features
        "fix", // 🐛 Bug Fixes
        "doc", // 📚 Documentation (兼容 cliff 的 ^doc)
        "docs", // 📚 Documentation (标准格式)
        "perf", // ⚡ Performance
        "refactor", // 🚜 Refactor
        "style", // 🎨 Styling
        "test", // 🧪 Testing
        "chore", // ⚙️ Miscellaneous Tasks
        "ci", // ⚙️ Miscellaneous Tasks
        "build", // ⚙️ Miscellaneous Tasks
        "revert", // ◀️ Revert
      ],
    ],
    // 强制 type 为小写
    "type-case": [2, "always", "lower-case"],
    // 禁止空 type
    "type-empty": [2, "never"],
    // 禁止空描述
    "subject-empty": [2, "never"],
    // 关闭对 subject 大小写的硬性限制，契合 Tera 模板的 upper_first 过滤器
    "subject-case": [0],
  },
};

export default Configuration;
