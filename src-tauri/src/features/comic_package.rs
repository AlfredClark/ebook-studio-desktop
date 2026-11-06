//! 漫画打包：`build/` 目录树 → `<uuid>/dist/书名.epub`，随后用 `epubveri` 校验。
//!
//! 打包与校验逻辑与小说同构（OCF 容器落盘 + 通用 EPUB3 检查），复用
//! `features::novel_package` 的通用实现；本模块只定漫画的进度事件名与入口。

use super::novel_package::{self, PackageOutcome};

/// 打包进度事件名：前端据此订阅 `zip` / `validate` 两阶段进度；改名时两端同步。
/// 与小说的 `novel-package-progress` 区分，避免跨类型串扰。
pub const PACKAGE_PROGRESS_EVENT: &str = "comic-package-progress";

/// 打包漫画（`AppHandle` 入口）：落盘后自动校验一次；CPU 密集段进阻塞池并推送进度。
pub async fn package_comic(app: &tauri::AppHandle, id: &str) -> anyhow::Result<PackageOutcome> {
    novel_package::package_with_progress(app, id, PACKAGE_PROGRESS_EVENT).await
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::super::comic_build;
    use super::super::comic_files;
    use super::super::novel_package;
    use super::super::projects;
    use super::super::projects::ProjectType;

    /// 用例隔离的项目：建真实漫画项目目录，跑完清理。
    fn test_project(case: &str) -> (PathBuf, PathBuf) {
        let root = std::env::temp_dir().join(format!(
            "ebook-studio-comic-package-test-{}-{case}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("create test root");
        let info =
            projects::create_project_in(&root, "雾镇", ProjectType::Comic).expect("create project");
        let dir = root.join(&info.id);
        (root, dir)
    }

    fn cleanup(root: &Path) {
        let _ = std::fs::remove_dir_all(root);
    }

    /// 建一个已构建的漫画项目：导入解析后构建，返回项目目录。
    fn built_project(case: &str, files: &[&str]) -> (PathBuf, PathBuf) {
        let (root, dir) = test_project(case);
        let source = root.join(format!("{case}-src"));
        let _ = std::fs::remove_dir_all(&source);
        for file in files {
            let path = source.join(file);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).expect("create source parent");
            }
            // 标准 1x1 PNG 真字节：epubveri 嗅探内容与声明类型，假头过不了。
            std::fs::write(
                &path,
                [
                    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49,
                    0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06,
                    0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44,
                    0x41, 0x54, 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0D,
                    0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42,
                    0x60, 0x82,
                ],
            )
            .expect("write source image");
        }
        comic_files::import_dir_in(&dir, &source).expect("import");
        comic_files::parse_images_in(&dir).expect("parse");
        let template_base =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates/EPUB33-COMIC");
        comic_build::build_in(
            &dir,
            &template_base,
            &super::super::novel_export::BuildTitleOptions::default(),
        )
        .expect("build");
        (root, dir)
    }

    #[test]
    fn packages_comic_build_and_validates() {
        let (root, dir) = built_project("pack", &["第1话/01.png", "第1话/02.png"]);
        // 无产物时状态为空，校验直接报错。
        assert!(novel_package::status_in(&dir).expect("status").is_none());
        let outcome = novel_package::write_epub_in(&dir, None).expect("package");
        assert!(
            Path::new(&outcome.file_name)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("epub"))
        );
        assert!(dir.join("dist").join(&outcome.file_name).is_file());
        let reread = novel_package::status_in(&dir)
            .expect("status")
            .expect("status");
        assert_eq!(reread.file_name, outcome.file_name);
        let validation = novel_package::validate_in(&dir).expect("validate");
        assert!(
            validation.valid,
            "own template output must validate: {:?}",
            validation.messages
        );
        // 流式页不带 viewport meta：HTM_060b 噪声不得出现。
        assert!(
            validation
                .messages
                .iter()
                .all(|message| message.id != "HTM_060b"),
            "unexpected viewport usage messages: {:?}",
            validation.messages
        );
        novel_package::clear_dist_in(&dir).expect("clear");
        assert!(!dir.join("dist").exists());
        assert!(novel_package::status_in(&dir).expect("status").is_none());
        cleanup(&root);
    }
}
