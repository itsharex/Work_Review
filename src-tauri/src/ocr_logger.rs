// OCR 日志保存模块
// 将每日 OCR 内容保存到文本文件，用于后续分析和总结
// 当前为预留模块

#![allow(dead_code)]

use crate::error::Result;
use chrono::Local;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

/// OCR 日志管理器
pub struct OcrLogger {
    /// 日志目录
    log_dir: PathBuf,
}

impl OcrLogger {
    /// 创建新的 OCR 日志管理器
    pub fn new(data_dir: &Path) -> Self {
        let log_dir = data_dir.join("ocr_logs");
        // 确保目录存在
        let _ = fs::create_dir_all(&log_dir);

        Self { log_dir }
    }

    /// 追加 OCR 内容到今日日志文件
    pub fn append_ocr(&self, app_name: &str, window_title: &str, ocr_text: &str) -> Result<()> {
        if ocr_text.trim().is_empty() {
            return Ok(());
        }

        let today = Local::now().format("%Y-%m-%d").to_string();
        let log_file = self.log_dir.join(format!("{today}.txt"));

        let time = Local::now().format("%H:%M:%S").to_string();
        let entry = format!(
            "[{}] {}\n窗口: {}\n内容: {}\n---\n",
            time,
            app_name,
            window_title,
            ocr_text.trim()
        );

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file)?;

        file.write_all(entry.as_bytes())?;

        Ok(())
    }

    /// 读取指定日期的 OCR 日志
    pub fn read_log(&self, date: &str) -> Result<String> {
        let log_file = self.log_dir.join(format!("{date}.txt"));
        if log_file.exists() {
            Ok(fs::read_to_string(log_file)?)
        } else {
            Ok(String::new())
        }
    }

    /// 获取今日 OCR 日志路径
    pub fn get_today_log_path(&self) -> PathBuf {
        let today = Local::now().format("%Y-%m-%d").to_string();
        self.log_dir.join(format!("{today}.txt"))
    }
}
