use crate::analysis::{format_duration, Analyzer};
use crate::config::AiProvider;
use crate::database::{Activity, DailyStats};
use crate::error::{AppError, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;
use std::path::Path;
use std::time::Duration;

/// 摘要上传分析器
/// 只上传统计摘要，不上传原始截图
pub struct SummaryAnalyzer {
    provider: AiProvider,
    endpoint: String,
    model: String,
    api_key: Option<String>,
    client: Client,
}

impl SummaryAnalyzer {
    pub fn new(provider: AiProvider, endpoint: &str, model: &str, api_key: Option<&str>) -> Self {
        // 创建带超时配置的 HTTP 客户端
        let client = Client::builder()
            .timeout(Duration::from_secs(90)) // 综合超时时间
            .connect_timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            provider,
            endpoint: endpoint.to_string(),
            model: model.to_string(),
            api_key: api_key.map(|s| s.to_string()),
            client,
        }
    }

    /// 判断是否为 Ollama 服务
    fn is_ollama(&self) -> bool {
        matches!(self.provider, AiProvider::Ollama)
    }

    /// 使用 Ollama 生成报告
    async fn generate_with_ollama(&self, prompt: &str) -> Result<String> {
        log::info!("使用 Ollama 生成: {} / {}", self.endpoint, self.model);
        let response = self
            .client
            .post(format!("{}/api/generate", self.endpoint))
            .json(&json!({
                "model": self.model,
                "prompt": prompt,
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
        Ok(result["response"].as_str().unwrap_or("").to_string())
    }

    /// 使用 OpenAI 兼容格式生成报告（支持所有 OpenAI 兼容的 API）
    async fn generate_with_openai_compatible(&self, prompt: &str) -> Result<String> {
        log::info!(
            "使用 OpenAI 兼容 API 生成: {} / {}",
            self.endpoint,
            self.model
        );

        let mut request = self.client
            .post(format!("{}/chat/completions", self.endpoint))
            .json(&json!({
                "model": self.model,
                "messages": [
                    {
                        "role": "system",
                        "content": "你是一个专业的工作效率分析助手，帮助用户分析和总结每日工作。请用中文回答。"
                    },
                    {
                        "role": "user",
                        "content": prompt
                    }
                ],
                "max_tokens": 5000,
                "temperature": 0.2,
            }));

        // 如果有 API Key，添加 Authorization header
        if let Some(api_key) = &self.api_key {
            if !api_key.is_empty() {
                request = request.header("Authorization", format!("Bearer {api_key}"));
            }
        }

        let response = request.send().await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::Analysis(format!("API 错误: {error_text}")));
        }

        let result: serde_json::Value = response.json().await?;
        Ok(result["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string())
    }

    /// 提取活动关键词（从窗口标题和 OCR 文本）
    fn extract_keywords(&self, activities: &[Activity]) -> Vec<String> {
        let mut keywords = Vec::new();

        for activity in activities {
            // 从窗口标题提取关键词
            let title_words: Vec<&str> = activity
                .window_title
                .split(|c: char| !c.is_alphanumeric() && c != '-' && c != '_')
                .filter(|w| w.len() > 3)
                .take(3)
                .collect();

            for word in title_words {
                if !keywords.contains(&word.to_string()) {
                    keywords.push(word.to_string());
                }
            }

            // 从 OCR 文本提取关键词
            if let Some(ocr_text) = &activity.ocr_text {
                let ocr_words: Vec<&str> = ocr_text
                    .split(|c: char| !c.is_alphanumeric() && c != '-' && c != '_')
                    .filter(|w| {
                        w.len() > 3 && w.chars().all(|c| c.is_alphabetic() || c >= '\u{4e00}')
                    })
                    .take(5)
                    .collect();

                for word in ocr_words {
                    if !keywords.contains(&word.to_string()) && keywords.len() < 30 {
                        keywords.push(word.to_string());
                    }
                }
            }
        }

        keywords.truncate(30);
        keywords
    }

    /// 从小时摘要生成日报提示词（三层架构的第三层）
    /// 当前为预留功能
    #[allow(dead_code)]
    pub fn build_daily_prompt_from_hourly(
        &self,
        date: &str,
        stats: &DailyStats,
        hourly_summaries: &[crate::database::HourlySummary],
    ) -> String {
        use crate::analysis::format_duration;

        let mut prompt = format!(
            r#"以下是一位辛苦打工人 {} 的工作概况：

## 📊 整体统计
- 总工作时长: {}
- 截图数量: {}

## ⏰ 各时段工作摘要
"#,
            date,
            format_duration(stats.total_duration),
            stats.screenshot_count
        );

        // 添加小时摘要
        for summary in hourly_summaries {
            prompt.push_str(&format!(
                "\n### {:02}:00-{:02}:00（{}分钟）\n{}\n主要应用：{}\n",
                summary.hour,
                summary.hour + 1,
                summary.total_duration / 60,
                summary.summary,
                summary.main_apps
            ));
        }

        // 添加应用使用统计
        prompt.push_str("\n## 📱 应用使用情况\n");
        for app in &stats.app_usage {
            prompt.push_str(&format!(
                "- {}: {}\n",
                app.app_name,
                format_duration(app.duration)
            ));
        }

        prompt.push_str(
            r#"

请根据以上数据，用你自己的风格生成一份有价值的工作日报。

你可以自由发挥，不需要遵循固定模板。可以：
- 用轻松幽默的方式点评今天的工作
- 分析时间的使用情况和效率
- 发现一些有意思的工作模式
- 给出接地气的改进建议

用 Markdown 格式书写，让这份报告既有干货又有趣味。"#,
        );

        prompt
    }

    /// 当 AI 不可用时的备用内容
    fn generate_fallback_ai_content(&self, keywords: &[String], apps_list: &str) -> String {
        let mut content = String::new();

        content.push_str("## 🎯 今日工作内容\n\n");
        if keywords.is_empty() {
            content.push_str(&format!(
                "今日主要使用 {apps_list} 等应用进行工作。持续努力中！\n"
            ));
        } else {
            content.push_str(&format!(
                "今日工作涉及：{}。使用的主要应用包括 {}。\n",
                keywords
                    .iter()
                    .take(5)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("、"),
                apps_list
            ));
        }

        content.push_str("\n## 📊 专注度分析\n\n");
        content.push_str("今日工作整体表现不错，继续保持稳定的工作节奏。\n");

        content.push_str("\n## 💡 明日建议\n\n");
        content.push_str("建议定期休息，避免久坐。深度工作时可以关闭通讯软件通知，提高专注度。\n");

        content.push_str(
            "\n---\n*注：由基础模板生成。配置 AI 模型（OpenAI/Ollama）后可获得更深度的智能分析。*",
        );

        content
    }
}

#[async_trait]
impl Analyzer for SummaryAnalyzer {
    async fn generate_report(
        &self,
        date: &str,
        stats: &DailyStats,
        activities: &[Activity],
        _screenshots_dir: &Path,
    ) -> Result<String> {
        log::info!("生成混合模式日报：固定模板 + AI 扩展");

        let mut report = String::new();

        // ==================== 标题 ====================
        report.push_str(&format!("# 工作日报\n\n**日期：{date}**\n\n"));

        // ==================== 今日概览 ====================
        report.push_str("## 一、今日概览\n\n");
        report.push_str("| 指标 | 数值 |\n");
        report.push_str("|:--|--:|\n");
        report.push_str(&format!("| 总工作时长 | {} |\n", format_duration(stats.total_duration)));
        report.push_str(&format!("| 截图数量 | {} 张 |\n", stats.screenshot_count));
        report.push_str(&format!("| 使用应用数 | {} 个 |\n", stats.app_usage.len()));
        report.push_str(&format!("| 访问网站数 | {} 个 |\n", stats.domain_usage.len()));
        report.push('\n');

        // ==================== 时间分配 ====================
        if !stats.category_usage.is_empty() {
            report.push_str("## 二、时间分配\n\n");
            report.push_str("| 类别 | 时长 | 占比 |\n");
            report.push_str("|:--|--:|--:|\n");
            for cat in &stats.category_usage {
                let percentage = if stats.total_duration > 0 {
                    (cat.duration as f64 / stats.total_duration as f64 * 100.0) as i32
                } else {
                    0
                };
                report.push_str(&format!(
                    "| {} | {} | {}% |\n",
                    crate::monitor::get_category_name(&cat.category),
                    format_duration(cat.duration),
                    percentage
                ));
            }
            report.push('\n');
        }

        // ==================== 应用使用明细 ====================
        if !stats.app_usage.is_empty() {
            report.push_str("## 三、应用使用明细\n\n");
            report.push_str("| 序号 | 应用名称 | 使用时长 |\n");
            report.push_str("|--:|:--|--:|\n");
            for (i, app) in stats.app_usage.iter().enumerate() {
                report.push_str(&format!(
                    "| {} | {} | {} |\n",
                    i + 1,
                    app.app_name,
                    format_duration(app.duration)
                ));
            }
            report.push('\n');
        }

        // ==================== 网站访问明细 ====================
        if !stats.domain_usage.is_empty() {
            report.push_str("## 四、网站访问明细\n\n");
            report.push_str("| 序号 | 网站域名 | 访问时长 |\n");
            report.push_str("|--:|:--|--:|\n");
            for (i, domain) in stats.domain_usage.iter().enumerate() {
                report.push_str(&format!(
                    "| {} | {} | {} |\n",
                    i + 1,
                    domain.domain,
                    format_duration(domain.duration)
                ));
            }
            report.push('\n');
        }

        // ==================== AI 分析 ====================
        report.push_str("## 五、AI 分析\n\n");

        // 准备 AI 输入
        let apps_list = stats
            .app_usage
            .iter()
            .take(8)
            .map(|a| format!("{} ({})", a.app_name, format_duration(a.duration)))
            .collect::<Vec<_>>()
            .join(", ");

        let urls_list = stats
            .domain_usage
            .iter()
            .take(5)
            .map(|d| d.domain.clone())
            .collect::<Vec<_>>()
            .join(", ");

        let keywords = self.extract_keywords(activities);
        let top_keywords = keywords
            .into_iter()
            .take(8)
            .collect::<Vec<_>>()
            .join(", ");

        // 规整的 AI 提示词（站在共同度过工作一天的角度，深入分析数据）
        let prompt = format!(
            r#"你是用户今天的工作伙伴，陪伴用户度过了这一天。请仔细分析以下数据，提炼出有价值的洞察，生成一份温暖亲切的工作回顾。

【今日原始数据】
工作时长：{}
主要应用：{}
访问网站：{}
屏幕内容关键词：{}

【核心要求】
你的任务是分析和总结，而不是复述数据。请做到：
1. 从应用组合和时长分布中推断今天的工作类型和重心。
2. 从网站访问和关键词中理解用户在研究或处理什么内容。
3. 综合以上信息，给出有洞察力的分析，帮助用户真正了解自己这一天的工作状态。
4. 如果某项数据显示为"无"，请明确说明未获取到该信息，不要推测。

【格式规则】
1. 只使用中文文字和必要标点符号（句号，逗号），禁止使用emoji、项目符号、装饰符号或星号。
2. 每个部分不超过5句话，使用短句。
3. 标题格式必须完全一致，不得修改。

【输出格式】
请严格按照以下四个加粗标题输出：

**工作内容概述**

根据应用使用和关键词，分析用户今天主要在做什么类型的工作，解决什么问题或推进什么任务。

**效率评估**

根据时长分布和应用切换情况，分析今天的专注度和时间利用效率如何。

**改进建议**

基于以上分析，给出1到2条具体可行的改进建议。

**今日小结**

用1到2句话，像朋友一样总结今天，给予肯定和鼓励。"#,
            format_duration(stats.total_duration),
            if apps_list.is_empty() { "无".to_string() } else { apps_list.clone() },
            if urls_list.is_empty() { "无".to_string() } else { urls_list },
            if top_keywords.is_empty() { "无".to_string() } else { top_keywords }
        );

        // 调用 AI
        let ai_content = if self.is_ollama() {
            match self.generate_with_ollama(&prompt).await {
                Ok(content) => content.trim().to_string(),
                Err(e) => {
                    log::warn!("Ollama 生成失败: {e}");
                    self.generate_fallback_ai_content(&[], &apps_list)
                }
            }
        } else {
            match self.generate_with_openai_compatible(&prompt).await {
                Ok(content) => content.trim().to_string(),
                Err(e) => {
                    log::warn!("API 生成失败: {e}");
                    self.generate_fallback_ai_content(&[], &apps_list)
                }
            }
        };

        report.push_str(&ai_content);

        Ok(report)
    }
}
