// release 构建不在 Windows 弹出多余的控制台窗口
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
// 启用 clippy 全部与 pedantic 组检查（警告级别）
#![warn(clippy::all, clippy::pedantic)]
// 豁免 Tauri 模板中常见但过于严苛的模块名重复检查
#![allow(clippy::module_name_repetitions)]

fn main() {
    // 二进制入口只做转发，桌面 / 移动端（mobile_entry_point）与测试共用同一份 run()
    tauri_desktop_template_lib::run();
}
