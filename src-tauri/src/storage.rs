use crate::config::StorageConfig;
use crate::error::Result;
use chrono::{Duration, Local, NaiveDate};
use std::fs;
use std::path::{Path, PathBuf};

/// 递归遍历目录（替代 walkdir）
fn walk_dir_recursive(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() {
                files.extend(walk_dir_recursive(&path));
            } else {
                files.push(path);
            }
        }
    }
    files
}

/// 存储管理器
/// 负责清理过期的截图和数据
pub struct StorageManager {
    data_dir: PathBuf,
    config: StorageConfig,
}

impl StorageManager {
    /// 创建存储管理器
    pub fn new(data_dir: &Path, config: StorageConfig) -> Self {
        Self {
            data_dir: data_dir.to_path_buf(),
            config,
        }
    }

    /// 更新配置
    pub fn update_config(&mut self, config: StorageConfig) {
        self.config = config;
    }

    /// 执行清理任务
    pub fn cleanup(&self) -> Result<CleanupResult> {
        // 1. 清理过期的截图文件
        let mut screenshots_deleted = self.cleanup_old_screenshots()?;

        // 2. 检查存储空间是否超限
        let current_size = self.calculate_storage_size()?;
        let total_size_mb = current_size as f64 / 1024.0 / 1024.0;

        // 如果超过限制，继续删除最旧的数据
        if current_size > (self.config.storage_limit_mb as u64 * 1024 * 1024) {
            screenshots_deleted += self.cleanup_oldest_until_under_limit()?;
        }

        log::info!(
            "存储清理完成: 删除 {screenshots_deleted} 个截图, 当前占用 {total_size_mb:.1} MB"
        );

        Ok(CleanupResult {
            screenshots_deleted,
            total_size_mb,
        })
    }

    /// 清理过期截图
    fn cleanup_old_screenshots(&self) -> Result<u32> {
        let screenshots_dir = self.data_dir.join("screenshots");
        if !screenshots_dir.exists() {
            return Ok(0);
        }

        let cutoff_date = Local::now().date_naive()
            - Duration::days(self.config.screenshot_retention_days as i64);
        let mut deleted_count = 0u32;

        // 遍历日期目录
        for entry in fs::read_dir(&screenshots_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                // 目录名格式: YYYY-MM-DD
                if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                    if let Ok(date) = NaiveDate::parse_from_str(dir_name, "%Y-%m-%d") {
                        if date < cutoff_date {
                            // 删除整个目录
                            match fs::remove_dir_all(&path) {
                                Ok(_) => {
                                    // 统计删除的文件数
                                    deleted_count += 1;
                                    log::info!("已删除过期截图目录: {dir_name}");
                                }
                                Err(e) => {
                                    log::warn!("删除目录失败 {dir_name}: {e}");
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(deleted_count)
    }

    /// 当存储超限时，删除最旧的数据直到低于限制
    fn cleanup_oldest_until_under_limit(&self) -> Result<u32> {
        let screenshots_dir = self.data_dir.join("screenshots");
        if !screenshots_dir.exists() {
            return Ok(0);
        }

        let limit_bytes = self.config.storage_limit_mb as u64 * 1024 * 1024;
        let mut deleted_count = 0u32;

        // 收集所有日期目录并排序
        let mut date_dirs: Vec<_> = fs::read_dir(&screenshots_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .filter_map(|e| {
                e.path()
                    .file_name()
                    .and_then(|n| n.to_str())
                    .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
                    .map(|date| (date, e.path()))
            })
            .collect();

        // 按日期升序排序（最旧的在前）
        date_dirs.sort_by_key(|(date, _)| *date);

        // 删除最旧的目录直到低于限制
        for (date, path) in date_dirs {
            let current_size = self.calculate_storage_size()?;
            if current_size < limit_bytes {
                break;
            }

            match fs::remove_dir_all(&path) {
                Ok(_) => {
                    deleted_count += 1;
                    log::info!("存储超限，已删除最旧目录: {date}");
                }
                Err(e) => {
                    log::warn!("删除目录失败 {date}: {e}");
                }
            }
        }

        Ok(deleted_count)
    }

    /// 计算当前存储占用大小（字节）
    fn calculate_storage_size(&self) -> Result<u64> {
        let screenshots_dir = self.data_dir.join("screenshots");
        if !screenshots_dir.exists() {
            return Ok(0);
        }

        let total_size: u64 = walk_dir_recursive(&screenshots_dir)
            .iter()
            .filter_map(|p| fs::metadata(p).ok())
            .map(|m| m.len())
            .sum();

        Ok(total_size)
    }

    /// 获取存储统计信息
    pub fn get_stats(&self) -> Result<StorageStats> {
        let screenshots_dir = self.data_dir.join("screenshots");

        let mut stats = StorageStats::default();

        if !screenshots_dir.exists() {
            return Ok(stats);
        }

        // 遍历统计
        for path in walk_dir_recursive(&screenshots_dir) {
            if let Ok(metadata) = fs::metadata(&path) {
                stats.total_files += 1;
                stats.total_size_bytes += metadata.len();
            }
        }

        stats.total_size_mb = stats.total_size_bytes as f64 / 1024.0 / 1024.0;
        stats.storage_limit_mb = self.config.storage_limit_mb;
        stats.retention_days = self.config.screenshot_retention_days;

        Ok(stats)
    }
}

/// 清理结果
#[derive(Debug, Default)]
pub struct CleanupResult {
    pub screenshots_deleted: u32,
    pub total_size_mb: f64,
}

/// 存储统计信息
#[derive(Debug, Default, serde::Serialize)]
pub struct StorageStats {
    pub total_files: u64,
    pub total_size_bytes: u64,
    pub total_size_mb: f64,
    pub storage_limit_mb: u32,
    pub retention_days: u32,
}
