//! 漫画文件命令：`sources/images/` 目录导入、状态查询与顺序解析的薄封装，具体文件操作在 `features::comic_files`。

use crate::cores::types::CommandResult;
use crate::features::comic_files::{
    self, AppendOutcome, ComicImageInfo, ComicImagesStatus, ComicOrphans, ComicOutline,
    ComicVolumeInput,
};
use crate::features::comic_images::{
    self, ComicCompressStatus, ComicConvertStatus, ComicResizeStatus, CompressOptions,
    CompressOutcome, CompressPreview, ConvertOptions, ConvertPreview, ImageTargetFormat,
    ResizeOptions, ResizeOutcome, ResizePreview, SingleImageOptions, SinglePreview,
};

/// 查询漫画原稿状态；缺目录回落空态，不建文件。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn get_comic_images_status(
    app: tauri::AppHandle,
    id: String,
) -> CommandResult<ComicImagesStatus> {
    Ok(comic_files::images_status(&app, &id)?)
}

/// 导入漫画目录：整体复制为 `sources/images`，返回最新状态。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn import_comic_images(
    app: tauri::AppHandle,
    id: String,
    source_path: String,
) -> CommandResult<ComicImagesStatus> {
    Ok(comic_files::import_images(&app, &id, &source_path)?)
}

/// 解析漫画顺序：扫描 `sources/images` 后落盘 `content.json`，返回识别大纲（图片二进制不出 IPC）。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn parse_comic_images(app: tauri::AppHandle, id: String) -> CommandResult<ComicOutline> {
    Ok(comic_files::parse_images(&app, &id)?)
}

/// 读取已落盘的识别大纲：缺失、损坏或版本不符回落空，不建文件。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn get_comic_content_outline(
    app: tauri::AppHandle,
    id: String,
) -> CommandResult<Option<ComicOutline>> {
    Ok(comic_files::content_outline(&app, &id)?)
}

/// 查询漫画图片信息：绝对路径供前端转 `asset:`，大小供标题栏展示；非图片或缺失直接报错。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn get_comic_image_path(
    app: tauri::AppHandle,
    id: String,
    path: String,
) -> CommandResult<ComicImageInfo> {
    Ok(comic_files::image_path(&app, &id, &path)?)
}

/// 全量更新漫画分话结构：标题可改、话可增删排序，页须全量覆盖，落盘后返回新大纲。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn update_comic_volumes(
    app: tauri::AppHandle,
    id: String,
    volumes: Vec<ComicVolumeInput>,
) -> CommandResult<ComicOutline> {
    Ok(comic_files::update_volumes(&app, &id, volumes)?)
}

/// 预览图片格式转换：按源格式分组计数，供确认弹窗逐行展示（图片二进制不出 IPC）。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn preview_comic_image_conversion(
    app: tauri::AppHandle,
    id: String,
    target: ImageTargetFormat,
    force: bool,
) -> CommandResult<ConvertPreview> {
    Ok(comic_images::preview_conversion(&app, &id, target, force)?)
}

/// 查询图片转换进度：切回页面时回填用；从未开始返回空闲零值。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn get_comic_convert_status(id: String) -> CommandResult<ComicConvertStatus> {
    Ok(comic_images::convert_status_of(&id))
}

/// 转换全量图片格式：事务性两阶段（暂存区编码→批量替换），成功后更新 `content.json`
/// 并清空下游 `build/` 与 `dist/`；阻塞池执行并推送逐文件进度，前端据此渲染进度。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub async fn convert_comic_images(
    app: tauri::AppHandle,
    id: String,
    target: ImageTargetFormat,
    options: ConvertOptions,
) -> CommandResult<ComicOutline> {
    comic_images::convert_images(&app, &id, target, &options).await
}

/// 取消图片作业：转换/压缩/尺寸调整互斥，任一在跑即置旗，编码循环在文件边界处中断；
/// 返回是否有在跑作业，无作业时不做任何事。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn cancel_comic_images(app: tauri::AppHandle, id: String) -> CommandResult<bool> {
    Ok(comic_images::cancel_images(&app, &id)?)
}

/// 预览图片压缩：只读 `content.json` + 文件大小，不解码图片。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn preview_comic_image_compression(
    app: tauri::AppHandle,
    id: String,
) -> CommandResult<CompressPreview> {
    Ok(comic_images::preview_compression(&app, &id)?)
}

/// 查询图片压缩进度：切回页面时回填用；从未开始返回空闲零值，备份位实时计算。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn get_comic_compress_status(
    app: tauri::AppHandle,
    id: String,
) -> CommandResult<ComicCompressStatus> {
    let dir = crate::features::projects::resolve_project_dir(&app, &id)?;
    Ok(comic_images::compress_status_of(&dir, &id))
}

/// 压缩全量图片：原地重编码（缩放 + 灰度 + 按源格式重编码），路径不变；
/// 成功后清空下游 `build/` 与 `dist/`；阻塞池执行并推送逐文件进度。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub async fn compress_comic_images(
    app: tauri::AppHandle,
    id: String,
    options: CompressOptions,
) -> CommandResult<CompressOutcome> {
    comic_images::compress_images(&app, &id, &options).await
}

/// 撤销压缩备份：把备份原图拷回原位，一次性有效，无备份直接报错。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn restore_comic_compress_backup(
    app: tauri::AppHandle,
    id: String,
) -> CommandResult<ComicOutline> {
    Ok(comic_images::restore_backup(&app, &id)?)
}

/// 清除压缩备份：只删备份目录，当前图片不动；无备份返回假。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn clear_comic_compress_backup(app: tauri::AppHandle, id: String) -> CommandResult<bool> {
    Ok(comic_images::clear_backup(&app, &id)?)
}

/// 预览图片尺寸调整：只读 `content.json` + 图片头（不全解码），返回总数、跳过数与总字节。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn preview_comic_image_resize(
    app: tauri::AppHandle,
    id: String,
    options: ResizeOptions,
) -> CommandResult<ResizePreview> {
    Ok(comic_images::preview_resize(&app, &id, &options)?)
}

/// 查询图片尺寸调整进度：切回页面时回填用；从未开始返回空闲零值，备份位实时计算。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn get_comic_resize_status(
    app: tauri::AppHandle,
    id: String,
) -> CommandResult<ComicResizeStatus> {
    let dir = crate::features::projects::resolve_project_dir(&app, &id)?;
    Ok(comic_images::resize_status_of(&dir, &id))
}

/// 调整全量图片尺寸：原地重编码（重排版 + 按源格式重编码），路径与格式不变；
/// 成功后清空下游 `build/` 与 `dist/`；阻塞池执行并推送逐文件进度。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub async fn resize_comic_images(
    app: tauri::AppHandle,
    id: String,
    options: ResizeOptions,
) -> CommandResult<ResizeOutcome> {
    comic_images::resize_images(&app, &id, &options).await
}

/// 预览单图处理：只读源文件，真编码写预览槽位，不碰源文件与下游；
/// 批量作业进行中直接拒绝，避免读到写一半的文件。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn preview_single_comic_image(
    app: tauri::AppHandle,
    id: String,
    path: String,
    options: SingleImageOptions,
) -> CommandResult<SinglePreview> {
    Ok(comic_images::preview_single(&app, &id, &path, &options)?)
}

/// 应用单图处理：变换后原子替换源文件，路径与格式不变；
/// 成功后清空下游 `build/` 与 `dist/`；与批量作业共用项目锁。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub async fn apply_single_comic_image(
    app: tauri::AppHandle,
    id: String,
    path: String,
    options: SingleImageOptions,
) -> CommandResult<ComicOutline> {
    comic_images::apply_single(&app, &id, &path, &options).await
}

/// 扫描孤儿图片：`content.json` 未引用的落盘图片，删页残留从这里清理。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn scan_comic_orphans(app: tauri::AppHandle, id: String) -> CommandResult<ComicOrphans> {
    Ok(comic_files::scan_orphans(&app, &id)?)
}

/// 删除孤儿图片：只删扫描命中的无引用图片，返回刷新后的识别大纲。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn delete_comic_orphans(app: tauri::AppHandle, id: String) -> CommandResult<ComicOutline> {
    Ok(comic_files::delete_orphans(&app, &id)?)
}

/// 增量追加图片：只收源目录顶层图片，按自然排序续号存入 `images/append/`；
/// 不碰 `content.json`（下次解析合并），忙时拒绝。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn append_comic_images(
    app: tauri::AppHandle,
    id: String,
    source_path: String,
) -> CommandResult<AppendOutcome> {
    Ok(comic_files::append_images(&app, &id, &source_path)?)
}
