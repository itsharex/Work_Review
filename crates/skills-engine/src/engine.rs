use crate::adaptive::AdaptiveEngine;
use crate::executor::{ExecutionContext, ExecutionResult, SkillExecutor};
use crate::model::{SignalSource, SkillPackage};
use crate::registry::{SkillRegistry, builtin_skills};
use crate::state::SkillState;
use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;

/// 技能引擎：顶层 API
pub struct SkillEngine {
    registry: SkillRegistry,
}

impl SkillEngine {
    /// 创建引擎并加载内置技能
    pub fn new() -> Self {
        let mut registry = SkillRegistry::new();
        for skill in builtin_skills() {
            registry.register(skill);
        }
        Self { registry }
    }

    /// 从文件加载技能注册表
    pub fn load_from_file(path: &Path) -> Result<Self> {
        let json = std::fs::read_to_string(path)?;
        let registry = SkillRegistry::load_from_json(&json)?;
        Ok(Self { registry })
    }

    /// 保存技能注册表到文件
    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        let json = self.registry.save_to_json()?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// 执行技能
    pub fn execute(&mut self, skill_id: &str, ctx: &ExecutionContext) -> ExecutionResult {
        SkillExecutor::execute(&mut self.registry, skill_id, ctx)
    }

    /// 注册新技能
    pub fn register_skill(&mut self, package: SkillPackage) {
        self.registry.register(package);
    }

    /// 注销技能
    pub fn unregister_skill(&mut self, id: &str) -> Option<SkillPackage> {
        self.registry.unregister(id)
    }

    /// 启用技能
    pub fn enable_skill(&mut self, id: &str) -> bool {
        self.registry.enable(id)
    }

    /// 禁用技能
    pub fn disable_skill(&mut self, id: &str) -> bool {
        self.registry.disable(id)
    }

    /// 列出所有技能
    pub fn list_skills(&self) -> Vec<&SkillPackage> {
        self.registry.list_all()
    }

    /// 列出已启用技能
    pub fn list_enabled_skills(&self) -> Vec<&SkillPackage> {
        self.registry.list_enabled()
    }

    /// 获取技能状态
    pub fn get_skill_state(&self, id: &str) -> Option<&SkillState> {
        self.registry.get_state(id)
    }

    /// 处理自适应信号
    pub fn process_adaptive_signal(
        &mut self,
        skill_id: &str,
        signal: SignalSource,
        context: HashMap<String, serde_json::Value>,
    ) -> crate::adaptive::AdaptiveResult {
        let package = match self.registry.get_package(skill_id) {
            Some(p) => p.clone(),
            None => return crate::adaptive::AdaptiveResult::Skipped,
        };
        let state = match self.registry.get_state_mut(skill_id) {
            Some(s) => s,
            None => return crate::adaptive::AdaptiveResult::Skipped,
        };
        AdaptiveEngine::process_signal(state, &package, signal, &context)
    }

    /// 一键回滚技能学习状态
    pub fn rollback_skill(&mut self, skill_id: &str, before_timestamp: u64) -> Option<usize> {
        let state = self.registry.get_state_mut(skill_id)?;
        Some(AdaptiveEngine::rollback(state, before_timestamp))
    }

    /// 暂停技能学习
    pub fn pause_learning(&mut self, skill_id: &str) -> bool {
        if let Some(state) = self.registry.get_state_mut(skill_id) {
            AdaptiveEngine::pause_learning(state);
            true
        } else {
            false
        }
    }

    /// 恢复技能学习
    pub fn resume_learning(&mut self, skill_id: &str) -> bool {
        if let Some(state) = self.registry.get_state_mut(skill_id) {
            AdaptiveEngine::resume_learning(state);
            true
        } else {
            false
        }
    }

    /// 获取所有技能的执行统计
    pub fn get_all_stats(&self) -> Vec<(&str, &crate::state::SkillStats)> {
        let mut result = Vec::new();
        for pkg in self.registry.list_all() {
            if let Some(stats) = self.registry.get_execution_stats(&pkg.id) {
                result.push((pkg.id.as_str(), stats));
            }
        }
        result
    }
}

impl Default for SkillEngine {
    fn default() -> Self {
        Self::new()
    }
}
