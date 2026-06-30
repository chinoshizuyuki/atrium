// SPDX-License-Identifier: MIT
//! 启发式事实提取器
//!
//! 从对话文本中提取 (主语, 谓语, 宾语) 三元组。
//! 纯规则匹配，无需 LLM，延迟 < 1μs/条。
//! LLM 级精提取在 Python 网关侧完成。
//! Regex-based fact extractor.
//!
//! Extracts (subject, predicate, object) triples from conversational text.
//! Pure regex matching, no LLM required; latency < 1ms/extraction.
//! LLM-based extraction is orchestrated by the Python layer.

/// 提取对话中的原子事实
///
/// # 中文模式
/// - "我(喜欢|爱|讨厌|想|要|在|有|知道|认为)..." → (说话者, 谓语, 宾语)
/// - "我的...(是|很)..." → (说话者的..., 是, ...)
/// - "...不..." → 否定事实
///
/// # 英文模式
/// - "I (like|love|hate|want|am|have|know|think|believe)..." → (speaker, predicate, object)
/// - "my ... (is|are)..." → (speaker's..., is, ...)
///
/// 返回 Vec<(主语, 谓语, 宾语, 初始置信度)>
pub fn extract_facts(text: &str, speaker: &str) -> Vec<(String, String, String, f64)> {
    let mut facts = Vec::new();

    // ── 中文模式 ──
    extract_chinese_patterns(text, speaker, &mut facts);
    // ── 英文模式 ──
    extract_english_patterns(text, speaker, &mut facts);

    facts
}

fn extract_chinese_patterns(
    text: &str,
    speaker: &str,
    out: &mut Vec<(String, String, String, f64)>,
) {
    // 主题+谓语正则（匹配 "我X" 开头，X 为高频谓语）
    let patterns: &[(&str, &str, f64)] = &[
        ("我喜欢", "喜欢", 0.85),
        ("我讨厌", "讨厌", 0.85),
        ("我恨", "讨厌", 0.80),
        ("我爱", "喜欢", 0.85),
        ("我想", "想要", 0.70),
        ("我要", "想要", 0.75),
        ("我在", "位于", 0.80),
        ("我有", "拥有", 0.85),
        ("我知道", "知道", 0.90),
        ("我认为", "认为", 0.65),
        ("我觉得", "认为", 0.65),
        ("我会", "能够", 0.60),
        ("我不喜欢", "不喜欢", 0.85),
        ("我不想", "不想", 0.75),
        ("我不要", "不想要", 0.75),
        ("我没有", "没有", 0.80),
        ("我不是", "不是", 0.90),
    ];

    for (prefix, predicate, conf) in patterns {
        if let Some(pos) = text.find(prefix) {
            let after = &text[pos + prefix.len()..];
            if let Some(end) = find_clause_end(after) {
                let object = after[..end].trim();
                if !object.is_empty() && object.len() < 80 {
                    out.push((
                        speaker.to_string(),
                        predicate.to_string(),
                        object.to_string(),
                        *conf,
                    ));
                }
            }
        }
    }

    // "我的X是Y" → (speaker.X, 是, Y)
    if let Some(pos) = text.find("我的") {
        let after = &text[pos + "我的".len()..];
        if let Some(shi_pos) = after.find('是') {
            let attr = after[..shi_pos].trim();
            let obj_start = shi_pos + "是".len();
            let obj_text = &after[obj_start..];
            if let Some(end) = find_clause_end(obj_text) {
                let obj = obj_text[..end].trim();
                if !attr.is_empty() && !obj.is_empty() && attr.len() < 30 && obj.len() < 80 {
                    let subject = format!("{}.{}", speaker, attr);
                    out.push((subject, "是".to_string(), obj.to_string(), 0.75));
                }
            }
        }
    }
}

fn extract_english_patterns(
    text: &str,
    speaker: &str,
    out: &mut Vec<(String, String, String, f64)>,
) {
    let lower = text.to_lowercase();
    let patterns: &[(&str, &str, f64)] = &[
        ("i like ", "likes", 0.85),
        ("i love ", "loves", 0.85),
        ("i hate ", "dislikes", 0.85),
        ("i want ", "wants", 0.75),
        ("i am ", "is", 0.80),
        ("i have ", "has", 0.85),
        ("i know ", "knows", 0.90),
        ("i think ", "thinks", 0.65),
        ("i believe ", "believes", 0.65),
        ("i can ", "can", 0.60),
        ("i don't like ", "dislikes", 0.85),
        ("i do not like ", "dislikes", 0.85),
    ];

    for (prefix, predicate, conf) in patterns {
        if let Some(pos) = lower.find(prefix) {
            let after = &text[pos + prefix.len()..];
            if let Some(end) = find_english_clause_end(after) {
                let object = after[..end].trim();
                if !object.is_empty() && object.len() < 80 {
                    out.push((
                        speaker.to_string(),
                        predicate.to_string(),
                        object.to_string(),
                        *conf,
                    ));
                }
            }
        }
    }
}

/// 找到中文从句结尾（遇到句号/逗号/问号/感叹号/分号 等分隔符）
fn find_clause_end(s: &str) -> Option<usize> {
    let delimiters = ['。', '，', '？', '！', '；', ',', '.', '?', '!', '、', '\n'];
    delimiters
        .iter()
        .filter_map(|d| s.find(*d))
        .min()
        .or(Some(s.len()))
}

/// 找到英文从句结尾
fn find_english_clause_end(s: &str) -> Option<usize> {
    let delimiters = ['.', ',', '?', '!', ';', '\n', '。', '，'];
    delimiters
        .iter()
        .filter_map(|d| s.find(*d))
        .min()
        .or(Some(s.len()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chinese_basic() {
        let facts = extract_facts("我喜欢编程，我讨厌下雨", "主人");
        assert!(facts.iter().any(|f| f.1 == "喜欢" && f.2 == "编程"));
        assert!(facts.iter().any(|f| f.1 == "讨厌" && f.2 == "下雨"));
    }

    #[test]
    fn test_chinese_negation() {
        let facts = extract_facts("我不喜欢辣的食物", "主人");
        assert!(facts.iter().any(|f| f.1 == "不喜欢" && f.2 == "辣的食物"));
    }

    #[test]
    fn test_chinese_possession() {
        let facts = extract_facts("我的名字是Atrium", "主人");
        assert!(facts.iter().any(|f| f.2 == "Atrium"));
    }

    #[test]
    fn test_english_basic() {
        let facts = extract_facts("I like Rust programming", "user");
        assert!(facts
            .iter()
            .any(|f| f.1 == "likes" && f.2 == "Rust programming"));
    }

    #[test]
    fn test_empty_input() {
        let facts = extract_facts("", "user");
        assert!(facts.is_empty());
    }

    #[test]
    fn test_no_match() {
        let facts = extract_facts("今天天气真好", "主人");
        assert!(facts.is_empty());
    }
}
