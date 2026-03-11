use crate::analysis::{format_duration, Analyzer};
use crate::database::{Activity, DailyStats};
use crate::error::{AppError, Result};
use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use reqwest::Client;
use serde_json::json;
use std::path::Path;
use std::time::Duration;

/// 本地多模态分析器
/// 使用 Ollama 运行本地多模态模型（如 LLaVA）
pub struct LocalAnalyzer {
    host: String,
    model: String,
    client: Client,
}

impl LocalAnalyzer {
    pub fn new(host: &str, model: &str) -> Self {
        // 创建带超时配置的 HTTP 客户端
        let client = Client::builder()
            .timeout(Duration::from_secs(120)) // Ollama 模型推理可能较慢，设置2分钟超时
            .connect_timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            host: host.to_string(),
            model: model.to_string(),
            client,
        }
    }

    /// 使用 Ollama 生成日报内容
    async fn generate_with_ollama(
        &self,
        date: &str,
        stats: &DailyStats,
        activities: &[Activity],
    ) -> Result<String> {
        // 构建提示词
        let apps_list = stats
            .app_usage
            .iter()
            .take(5)
            .map(|a| format!("{}: {}", a.app_name, format_duration(a.duration)))
            .collect::<Vec<_>>()
            .join(", ");

        let urls_list = stats
            .domain_usage
            .iter()
            .take(3)
            .map(|d| d.domain.clone())
            .collect::<Vec<_>>()
            .join(", ");

        // 提取关键词
        let keywords: Vec<String> = activities
            .iter()
            .filter_map(|a| a.ocr_text.as_ref())
            .flat_map(|text| {
                text.split(|c: char| !c.is_alphanumeric() && c != '-')
                    .filter(|w| w.len() > 3)
                    .take(3)
                    .map(|s| s.to_string())
            })
            .take(20)
            .collect();

        let prompt = format!(
            r#"你是一位风趣幽默的工作效率分析师。请根据以下打工人今日的工作数据，生成一份有温度的工作分析。

## 今日数据
- 日期：{} 
- 总工作时长：{} 
- 使用的应用：{}
- 访问的网站：{}
- 从屏幕内容提取的关键词：{}

## 要求
请用轻松友好的语气，生成以下内容（必须包含这三个部分）：

## 🎯 今日工作内容
（根据应用、网站和关键词，推断用户今天主要做了什么工作，2-4 句话）

## 📊 专注度分析
（分析用户的工作专注度，是否频繁切换应用，给出一个简短评价）

## 💡 明日建议
（给一条接地气的效率改进建议，可以带点幽默）

注意：直接输出内容，不要有任何额外说明。"#,
            date,
            format_duration(stats.total_duration),
            apps_list,
            if urls_list.is_empty() {
                "无".to_string()
            } else {
                urls_list
            },
            if keywords.is_empty() {
                "无".to_string()
            } else {
                keywords.join("、")
            }
        );

        // 调用 Ollama API
        let response = self
            .client
            .post(format!("{}/api/generate", self.host))
            .json(&json!({
                "model": self.model,
                "prompt": prompt,
                "stream": false,
                "options": {
                    "temperature": 0.2,  // 降低 temperature 提高输出稳定性
                    "seed": 42  // 使用固定种子提高一致性
                }
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(AppError::Analysis(format!(
                "Ollama API 错误: {}",
                response.status()
            )));
        }

        let result: serde_json::Value = response.json().await?;
        let ai_content = result["response"].as_str().unwrap_or("").to_string();

        if ai_content.is_empty() {
            return Err(AppError::Analysis("Ollama 返回空内容".to_string()));
        }

        Ok(ai_content)
    }

    /// 分析单张截图（保留供视觉模式使用）
    #[allow(dead_code)]
    async fn analyze_screenshot(&self, screenshot_path: &Path) -> Result<String> {
        // 读取截图并转换为 Base64
        let image_data = tokio::fs::read(screenshot_path).await?;
        let image_base64 = BASE64_STANDARD.encode(&image_data);

        // 调用 Ollama API
        let response = self.client
            .post(format!("{}/api/generate", self.host))
            .json(&json!({
                "model": self.model,
                "prompt": "请简要描述这张截图中的内容，用户正在做什么工作？请用中文回答，限制在50字以内。",
                "images": [image_base64],
                "stream": false,
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(AppError::Analysis(format!(
                "Ollama API 错误: {}",
                response.status()
            )));
        }

        let result: serde_json::Value = response.json().await?;
        let insight = result["response"]
            .as_str()
            .unwrap_or("无法分析")
            .to_string();

        Ok(insight)
    }
}

#[async_trait]
impl Analyzer for LocalAnalyzer {
    async fn generate_report(
        &self,
        date: &str,
        stats: &DailyStats,
        activities: &[Activity],
        _screenshots_dir: &Path,
    ) -> Result<String> {
        log::info!("生成本地日报（尝试调用 Ollama）");

        // 首先生成固定的统计部分
        let mut report = format!("# 📈 工作日报 - {date}\n\n");

        // 固定模板部分：数据统计
        report.push_str("## 📊 今日概览\n\n");
        report.push_str(&format!(
            "- **总工作时长**: {}\n",
            format_duration(stats.total_duration)
        ));
        report.push_str(&format!("- **截图数量**: {} 张\n", stats.screenshot_count));
        report.push_str(&format!("- **使用应用**: {} 个\n\n", stats.app_usage.len()));

        // 时间分配
        report.push_str("## ⏱️ 时间分配\n\n");
        for cat in &stats.category_usage {
            let percentage = if stats.total_duration > 0 {
                (cat.duration as f64 / stats.total_duration as f64 * 100.0) as i32
            } else {
                0
            };
            report.push_str(&format!(
                "- **{}**: {} ({}%)\n",
                crate::monitor::get_category_name(&cat.category),
                format_duration(cat.duration),
                percentage
            ));
        }

        // 应用排行
        report.push_str("\n## 🖥️ 应用使用情况\n\n");
        for (i, app) in stats.app_usage.iter().take(5).enumerate() {
            report.push_str(&format!(
                "{}. **{}**: {}\n",
                i + 1,
                app.app_name,
                format_duration(app.duration)
            ));
        }

        // 网站访问
        if !stats.domain_usage.is_empty() {
            report.push_str("\n## 🌐 网站访问\n\n");
            for domain in stats.domain_usage.iter().take(5) {
                report.push_str(&format!(
                    "- **{}**: {}\n",
                    domain.domain,
                    format_duration(domain.duration)
                ));
            }
        }

        // 尝试调用 Ollama 生成 AI 扩展内容
        match self.generate_with_ollama(date, stats, activities).await {
            Ok(ai_content) => {
                log::info!("Ollama 生成成功");
                report.push('\n');
                report.push_str(&ai_content);
            }
            Err(e) => {
                log::warn!("Ollama 调用失败，使用备选内容: {e}");
                // 使用简单的备选内容
                report.push_str("\n## 🎯 今日工作内容\n\n");
                let apps_list = stats
                    .app_usage
                    .iter()
                    .take(3)
                    .map(|a| a.app_name.clone())
                    .collect::<Vec<_>>()
                    .join("、");
                report.push_str(&format!(
                    "今日主要使用 {apps_list} 等应用进行工作。持续努力中！\n"
                ));

                report.push_str("\n## 📊 专注度分析\n\n");
                report.push_str("今日工作整体表现不错，继续保持稳定的工作节奏。\n");

                report.push_str("\n## 💡 明日建议\n\n");
                report.push_str(
                    "建议定期休息，避免久坐。深度工作时可以关闭通讯软件通知，提高专注度。\n",
                );

                report.push_str("\n---\n*注：AI 分析暂不可用，使用基础模板生成。*");
            }
        }

        Ok(report)
    }
}
