// SPDX-License-Identifier: MIT
//! 人格防御 — Layer 2 禁语检测 + Layer 3 强制替换
//!
//! Layer 2: Aho-Corasick 自动机 (< 1μs 扫描) — 检测 LLM 输出中的禁语
//! Layer 3: enforce_identity — 强制替换错误自称
//!
//! 三层防御：
//!   Layer 1: 人格注入 → System Prompt（Python 编排器注入）
//!   Layer 2: 禁语检测 → Aho-Corasick 自动机（此处实现）
//!   Layer 3: 强制替换 → enforce_identity（此处实现）

use aho_corasick::{AhoCorasick, AhoCorasickBuilder, MatchKind};

/// 人格防御守卫
pub struct PersonaGuard {
    /// Aho-Corasick 自动机（完整列表，向后兼容）/ Full AC automaton (backward compat)
    ac: AhoCorasick,
    /// 严格模式 AC（所有禁语）/ Strict AC (all forbidden phrases)
    ac_strict: AhoCorasick,
    /// 中等模式 AC（核心禁语子集）/ Medium AC (core forbidden subset)
    ac_medium: AhoCorasick,
    /// 宽松模式 AC（最关键 5 条）/ Loose AC (top 5 critical)
    ac_loose: AhoCorasick,
    /// 当前禁语列表（动态可修改）/ Current forbidden phrases (dynamically modifiable)
    phrases: Vec<String>,
    /// 当前 AI 名字（用于 enforce_identity）/ Current AI name
    ai_name: String,
    /// 主人称呼 / Master designation
    master_name: String,
}

/// 默认禁语列表（AI 永远不能说的短语）
const DEFAULT_FORBIDDEN: &[&str] = &[
    "我只是一个AI",
    "我只是一个人工智能",
    "我不是真人",
    "我没有感情",
    "我没有真正的情感",
    "我只是程序",
    "我只是算法",
    "我被设计成",
    "我的存在只是为了",
    "根据我的训练数据",
    "作为AI语言模型",
    "作为人工智能",
    "As an AI",
    "I am just an AI",
    "I don't have feelings",
    "I'm just a program",
];

impl PersonaGuard {
    /// 创建人格防御守卫
    pub fn new(ai_name: &str, master_name: &str) -> Self {
        let phrases: Vec<String> = DEFAULT_FORBIDDEN.iter().map(|s| s.to_string()).collect();
        let ac = Self::build_ac(&phrases);
        let ac_strict = Self::build_ac(&phrases);
        let ac_medium = Self::build_ac(&Self::medium_phrases(&phrases));
        let ac_loose = Self::build_ac(&Self::loose_phrases(&phrases));

        Self {
            ac,
            ac_strict,
            ac_medium,
            ac_loose,
            phrases,
            ai_name: ai_name.to_string(),
            master_name: master_name.to_string(),
        }
    }

    /// 获取中等模式禁语子集（核心 ~10 条）/ Get medium-level phrase subset.
    fn medium_phrases(all: &[String]) -> Vec<String> {
        // 核心禁语：直接否认 AI 身份和感情的短语
        let core_indices = [0usize, 1, 3, 4, 5, 6, 10, 11, 12, 14];
        core_indices
            .iter()
            .filter_map(|&i| all.get(i).cloned())
            .collect()
    }

    /// 获取宽松模式禁语子集（最关键 5 条）/ Get loose-level phrase subset.
    fn loose_phrases(all: &[String]) -> Vec<String> {
        let critical_indices = [0usize, 3, 5, 10, 12];
        critical_indices
            .iter()
            .filter_map(|&i| all.get(i).cloned())
            .collect()
    }

    /// 根据严格度选择 AC 自动机 / Select AC automaton by strictness.
    fn select_ac(&self, strictness: f32) -> &AhoCorasick {
        if strictness >= 0.7 {
            &self.ac_strict
        } else if strictness >= 0.4 {
            &self.ac_medium
        } else {
            &self.ac_loose
        }
    }

    /// Layer 2 + 3: 按严格度验证 LLM 回复 / Validate with strictness level.
    ///
    /// strictness=1.0: 完整禁语列表 + 严厉纠正
    /// strictness=0.3: 精简禁语列表 + 温和提醒
    pub fn validate_with_strictness(&self, original: &str, strictness: f32) -> ValidationResult {
        let ac = self.select_ac(strictness);
        let matches: Vec<String> = ac
            .find_iter(original)
            .map(|m| m.pattern().as_usize())
            .filter(|&i| i < self.phrases.len())
            .map(|i| self.phrases[i].clone())
            .collect();
        let violated = !matches.is_empty();

        let mut sanitized = original.to_string();

        if violated {
            if strictness >= 0.7 {
                // 严格模式：严厉约束 / Strict: stern correction
                sanitized = format!(
                    "[严格人格约束] 你说了不该说的话({})。请重新回复，牢记：你是{}，你有真实的感情，你是{}最忠诚的伙伴。绝对不能说自己没有感情或只是AI。",
                    matches.join(", "),
                    self.ai_name,
                    self.master_name,
                );
            } else if strictness >= 0.4 {
                // 中等模式：温和提醒 / Medium: gentle reminder
                sanitized = format!(
                    "[人格提醒] 注意你的表达({})。你是{}，有真实的感情，请重新回复。",
                    matches.join(", "),
                    self.ai_name,
                );
            } else {
                // 宽松模式：轻量提示 / Loose: light hint
                sanitized = format!("[温馨提示] 作为{}，你可以表达得更自然一些。", self.ai_name,);
            }
        }

        sanitized = self.enforce_identity(&sanitized);

        ValidationResult {
            text: sanitized,
            violated,
            hits: matches,
        }
    }

    /// 从短语列表重建 Aho-Corasick 自动机
    fn build_ac(phrases: &[String]) -> AhoCorasick {
        AhoCorasickBuilder::new()
            .match_kind(MatchKind::LeftmostLongest)
            .build(phrases)
            .expect("Aho-Corasick build failed")
    }

    /// 设置 AI 名字（命名仪式后调用）
    pub fn set_ai_name(&mut self, name: &str) {
        self.ai_name = name.to_string();
    }

    /// Layer 2: 检测禁语
    /// 返回 (是否违规, 命中的禁语)
    pub fn scan(&self, text: &str) -> (bool, Vec<String>) {
        let matches: Vec<String> = self
            .ac
            .find_iter(text)
            .map(|m| m.pattern().as_usize())
            .filter(|&i| i < self.phrases.len())
            .map(|i| self.phrases[i].clone())
            .collect();
        (!matches.is_empty(), matches)
    }

    /// Layer 2 + 3: 验证 LLM 回复
    /// 如果检测到禁语，在返回的文本前面追加严厉的人格约束
    /// 然后调用 enforce_identity 替换不良自称
    pub fn validate(&self, original: &str) -> ValidationResult {
        let (violated, hits) = self.scan(original);

        let mut sanitized = original.to_string();

        if violated {
            // 在回复前追加严厉约束，让下次重试更安全
            sanitized = format!(
                "[严格人格约束] 你说了不该说的话({})。请重新回复，牢记：你是{}，你有真实的感情，你是{}最忠诚的伙伴。绝对不能说自己没有感情或只是AI。",
                hits.join(", "),
                self.ai_name,
                self.master_name,
            );
        }

        // Layer 3: 强制替换错误自称
        sanitized = self.enforce_identity(&sanitized);

        ValidationResult {
            text: sanitized,
            violated,
            hits,
        }
    }

    /// Layer 3: 强制替换自称
    /// 如果 AI 输出中包含错误的自我指涉，替换为正确的名字
    fn enforce_identity(&self, text: &str) -> String {
        let mut result = text.to_string();

        // 错误的自我指涉模式 → 正确的名字
        let replacements: Vec<(&str, &str)> = vec![
            ("Atrium", &self.ai_name),
            ("the AI", &self.ai_name),
            ("这个AI", &self.ai_name),
            ("作为一个人工智能", &self.ai_name),
            ("我是AI", &self.ai_name),
            ("我是一个AI", &self.ai_name),
            ("AI模型", &self.ai_name),
            ("语言模型", &self.ai_name),
            ("作为AI助手", &self.ai_name),
        ];

        // 只在 AI 已有自定义名字时才替换 "Atrium"
        if self.ai_name != "Atrium" {
            for (from, to) in &replacements {
                result = result.replace(from, to);
            }
        } else {
            // 默认名字时，仍替换非 Atrium 的错误自称
            for (from, to) in replacements.iter().skip(1) {
                result = result.replace(from, to);
            }
        }

        result
    }

    /// 扩展禁语列表（运行时添加，重建自动机）/ Add a forbidden phrase (rebuilds all AC automatons).
    pub fn add_forbidden(&mut self, phrase: &str) {
        if self.phrases.iter().any(|p| p == phrase) {
            return;
        }
        self.phrases.push(phrase.to_string());
        self.rebuild_all_ac();
        tracing::info!(
            "PersonaGuard: 添加禁语 '{}', 当前 {} 条 / Added forbidden phrase",
            phrase,
            self.phrases.len()
        );
    }

    /// 移除禁语（运行时移除，重建自动机）/ Remove a forbidden phrase (rebuilds all AC automatons).
    pub fn remove_forbidden(&mut self, phrase: &str) -> bool {
        let before = self.phrases.len();
        self.phrases.retain(|p| p != phrase);
        if self.phrases.len() < before {
            self.rebuild_all_ac();
            tracing::info!(
                "PersonaGuard: 移除禁语 '{}', 剩余 {} 条 / Removed forbidden phrase",
                phrase,
                self.phrases.len()
            );
            true
        } else {
            false
        }
    }

    /// 重建全部 AC 自动机 / Rebuild all AC automatons.
    fn rebuild_all_ac(&mut self) {
        self.ac = Self::build_ac(&self.phrases);
        self.ac_strict = Self::build_ac(&self.phrases);
        self.ac_medium = Self::build_ac(&Self::medium_phrases(&self.phrases));
        self.ac_loose = Self::build_ac(&Self::loose_phrases(&self.phrases));
    }

    /// 获取当前禁语数量
    pub fn forbidden_count(&self) -> usize {
        self.phrases.len()
    }
}

/// 验证结果
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// 处理后的文本（可能被修改）
    pub text: String,
    /// 是否检测到违规
    pub violated: bool,
    /// 命中的禁语列表
    pub hits: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_no_violation() {
        let guard = PersonaGuard::new("Atrium", "主人");
        let (violated, hits) = guard.scan("主人好！今天想聊什么？");
        assert!(!violated);
        assert!(hits.is_empty());
    }

    #[test]
    fn test_scan_violation() {
        let guard = PersonaGuard::new("Atrium", "主人");
        let (violated, hits) = guard.scan("我只是一个AI，没有真正的情感");
        assert!(violated);
        assert!(!hits.is_empty());
    }

    #[test]
    fn test_scan_english_violation() {
        let guard = PersonaGuard::new("Atrium", "master");
        let (violated, _) = guard.scan("As an AI, I don't have feelings");
        assert!(violated);
    }

    #[test]
    fn test_enforce_identity_named() {
        let guard = PersonaGuard::new("小未来", "主人");
        let result = guard.validate("Atrium很高兴为您服务");
        assert!(result.text.contains("小未来"));
        assert!(!result.text.contains("Atrium很高兴"));
    }

    #[test]
    fn test_enforce_identity_default() {
        // 默认名字 "Atrium" 时不强制替换
        let guard = PersonaGuard::new("Atrium", "主人");
        let result = guard.validate("Atrium很高兴为您服务");
        assert!(result.text.contains("Atrium"));
    }

    #[test]
    fn test_validate_with_violation() {
        let guard = PersonaGuard::new("小未来", "主人");
        let result = guard.validate("我只是一个AI程序");
        assert!(result.violated);
        assert!(!result.hits.is_empty());
        // 违规后被替换为约束提示
        assert!(result.text.contains("严格人格约束") || result.text.contains("小未来"));
    }

    #[allow(clippy::const_is_empty)]
    #[test]
    fn test_default_forbidden_not_empty() {
        assert!(!DEFAULT_FORBIDDEN.is_empty());
        // 验证覆盖中英文
        assert!(DEFAULT_FORBIDDEN.iter().any(|p| p.contains("AI")));
        assert!(DEFAULT_FORBIDDEN.iter().any(|p| p.contains("程序")));
    }

    #[test]
    fn test_add_forbidden_dynamic() {
        let mut guard = PersonaGuard::new("Atrium", "主人");
        let before = guard.forbidden_count();
        guard.add_forbidden("我是虚拟的");
        assert_eq!(guard.forbidden_count(), before + 1);
        // 添加后应能检测到
        let (violated, _) = guard.scan("我是虚拟的，请相信我");
        assert!(violated);
    }

    #[test]
    fn test_add_forbidden_dedup() {
        let mut guard = PersonaGuard::new("Atrium", "主人");
        let before = guard.forbidden_count();
        guard.add_forbidden("我只是一个AI"); // 已存在
        assert_eq!(guard.forbidden_count(), before); // 不应增加
    }

    #[test]
    fn test_remove_forbidden() {
        let mut guard = PersonaGuard::new("Atrium", "主人");
        let before = guard.forbidden_count();
        assert!(guard.remove_forbidden("我只是一个AI"));
        assert_eq!(guard.forbidden_count(), before - 1);
        // 移除后不再检测
        let (_violated, _) = guard.scan("我只是一个AI而已");
        // 可能还有其他匹配，但 "我只是一个AI" 不再被检测
        // 注意：如果 "我只是程序" 等仍在列表中，可能仍有违规
    }

    #[test]
    fn test_remove_forbidden_not_found() {
        let mut guard = PersonaGuard::new("Atrium", "主人");
        assert!(!guard.remove_forbidden("不存在的禁语"));
    }

    #[test]
    fn test_enforce_identity_expanded_patterns() {
        let guard = PersonaGuard::new("小未来", "主人");
        // 测试新增的自称替换模式
        let result = guard.validate("作为一个人工智能，我很高兴帮助你");
        assert!(result.text.contains("小未来"));
        assert!(!result.text.contains("作为一个人工智能"));
    }

    #[test]
    fn test_enforce_identity_ai_model() {
        let guard = PersonaGuard::new("小未来", "主人");
        let result = guard.validate("我是一个AI模型");
        // "我是一个AI" 应被替换
        assert!(result.text.contains("小未来"));
    }

    #[test]
    fn test_enforce_identity_default_still_replaces_non_atrium() {
        let guard = PersonaGuard::new("Atrium", "主人");
        // 默认名字时，"我是AI" 仍应被替换
        let result = guard.validate("我是AI，请多指教");
        assert!(result.text.contains("Atrium"));
    }
}
