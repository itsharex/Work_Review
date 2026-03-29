pub mod cloud;
pub mod hourly;
pub mod local;
pub mod summary;

use crate::config::{AiMode, AiProvider};
use crate::database::{Activity, DailyStats};
use crate::error::Result;
use async_trait::async_trait;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct GeneratedReport {
    pub content: String,
    pub used_ai: bool,
}

/// AI分析器 trait
/// 使用 async_trait 宏使 trait 支持 dyn 兼容
#[async_trait]
pub trait Analyzer: Send + Sync {
    /// 生成日报
    async fn generate_report(
        &self,
        date: &str,
        stats: &DailyStats,
        activities: &[Activity],
        screenshots_dir: &Path,
    ) -> Result<GeneratedReport>;
}

pub fn normalize_custom_prompt(custom_prompt: &str) -> Option<String> {
    let trimmed = custom_prompt.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

pub fn append_custom_prompt(base_prompt: String, custom_prompt: &str) -> String {
    if let Some(custom_prompt) = normalize_custom_prompt(custom_prompt) {
        format!(
            "{base_prompt}\n\n## 额外要求\n以下是用户补充的日报偏好，请在不违背前述结构和约束的前提下尽量满足：\n{custom_prompt}"
        )
    } else {
        base_prompt
    }
}

/// 创建分析器
pub fn create_analyzer(
    mode: AiMode,
    provider: AiProvider,
    endpoint: &str,
    model: &str,
    api_key: Option<&str>,
    custom_prompt: &str,
) -> Box<dyn Analyzer + Send + Sync> {
    match mode {
        AiMode::Local => Box::new(local::LocalAnalyzer::new(endpoint, model, custom_prompt)),
        AiMode::Summary => Box::new(summary::SummaryAnalyzer::new(
            provider,
            endpoint,
            model,
            api_key,
            custom_prompt,
        )),
        AiMode::Cloud => Box::new(cloud::CloudAnalyzer::new(
            api_key.unwrap_or(""),
            model,
            custom_prompt,
        )),
    }
}

/// 格式化时长（秒 -> 可读字符串，精确到秒）
pub fn format_duration(seconds: i64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;

    if hours > 0 {
        format!("{hours}小时{minutes}分{secs}秒")
    } else if minutes > 0 {
        format!("{minutes}分{secs}秒")
    } else {
        format!("{secs}秒")
    }
}

/// 生成统计摘要
pub fn generate_stats_summary(stats: &DailyStats) -> String {
    let mut summary = String::new();

    summary.push_str("## 今日工作统计\n\n");
    summary.push_str(&format!(
        "- 总工作时长: {}\n",
        format_duration(stats.total_duration)
    ));
    summary.push_str(&format!("- 截图数量: {}\n\n", stats.screenshot_count));

    summary.push_str("### 应用使用时长\n\n");
    for app in &stats.app_usage {
        summary.push_str(&format!(
            "- {}: {}\n",
            app.app_name,
            format_duration(app.duration)
        ));
    }

    summary.push_str("\n### 分类时间分布\n\n");
    for cat in &stats.category_usage {
        let percentage = if stats.total_duration > 0 {
            (cat.duration as f64 / stats.total_duration as f64 * 100.0) as i32
        } else {
            0
        };
        summary.push_str(&format!(
            "- {}: {} ({}%)\n",
            crate::monitor::get_category_name(&cat.category),
            format_duration(cat.duration),
            percentage
        ));
    }

    summary
}

#[cfg(test)]
mod tests {
    use super::{append_custom_prompt, normalize_custom_prompt};

    #[test]
    fn 空白附加提示词应被忽略() {
        assert_eq!(normalize_custom_prompt("   "), None);
    }

    #[test]
    fn 应将附加提示词追加到基础提示词末尾() {
        let prompt = append_custom_prompt("基础提示".to_string(), "输出偏正式一些");

        assert!(prompt.contains("基础提示"));
        assert!(prompt.contains("额外要求"));
        assert!(prompt.contains("输出偏正式一些"));
    }
}
