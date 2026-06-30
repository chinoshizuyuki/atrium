// SPDX-License-Identifier: MIT
//! Embedding 降级策略
//! Embedding fallback strategy module.

/// 降级等级
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FallbackLevel {
    Full,        // 向量 + 关键词混合
    KeywordOnly, // 仅关键词
    ExactMatch,  // 仅精确匹配
}

/// 搜索结果
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub text: String,
    pub score: f64,
}

/// 嵌入降级器
pub struct EmbeddingFallback {
    level: FallbackLevel,
    keyword_index: std::collections::HashMap<String, Vec<String>>,
}

impl Default for EmbeddingFallback {
    fn default() -> Self {
        Self::new()
    }
}

impl EmbeddingFallback {
    pub fn new() -> Self {
        Self {
            level: FallbackLevel::KeywordOnly,
            keyword_index: std::collections::HashMap::new(),
        }
    }

    pub fn set_level(&mut self, level: FallbackLevel) {
        self.level = level;
    }
    pub fn level(&self) -> FallbackLevel {
        self.level
    }

    /// 构建关键词倒排索引
    pub fn index_texts(&mut self, texts: &[String]) {
        self.keyword_index.clear();
        for text in texts {
            for word in tokenize(text) {
                self.keyword_index
                    .entry(word)
                    .or_default()
                    .push(text.clone());
            }
        }
    }

    /// 根据当前降级等级搜索
    pub fn search(&self, query: &str) -> Vec<SearchResult> {
        match self.level {
            FallbackLevel::Full => self.full_search(query),
            FallbackLevel::KeywordOnly => self.keyword_search(query),
            FallbackLevel::ExactMatch => self.exact_search(query),
        }
    }

    /// Full 模式：关键词 + TF 加权混合搜索，模拟向量召回的广度与精度
    fn full_search(&self, query: &str) -> Vec<SearchResult> {
        let mut kw_results = self.keyword_search(query);
        // TF 加权增强：命中关键词越多的文档得分越高
        let query_tokens = tokenize(query);
        if query_tokens.len() > 1 {
            for r in &mut kw_results {
                let hit_count = query_tokens
                    .iter()
                    .filter(|t| tokenize(&r.text).iter().any(|dt| dt == *t))
                    .count();
                let tf_boost = hit_count as f64 / query_tokens.len() as f64;
                // 混合：原关键词得分 60% + TF 匹配度 40%
                r.score = r.score * 0.6 + tf_boost * 0.4;
            }
            kw_results.sort_by(|a, b| {
                b.score
                    .partial_cmp(&a.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        kw_results
    }

    fn keyword_search(&self, query: &str) -> Vec<SearchResult> {
        let tokens = tokenize(query);
        let mut scores: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
        for token in &tokens {
            if let Some(matches) = self.keyword_index.get(token) {
                for m in matches {
                    *scores.entry(m.clone()).or_insert(0.0) += 1.0;
                }
            }
        }
        let mut results: Vec<SearchResult> = scores
            .into_iter()
            .map(|(text, score)| SearchResult {
                text,
                score: score / tokens.len() as f64,
            })
            .collect();
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).expect("embedding score cmp"));
        results
    }

    fn exact_search(&self, query: &str) -> Vec<SearchResult> {
        self.keyword_index
            .values()
            .flatten()
            .filter(|t| t == &query)
            .map(|t| SearchResult {
                text: t.clone(),
                score: 1.0,
            })
            .collect()
    }
}

/// 简易分词（英文空格 + 中文单字）
fn tokenize(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    for word in text.split_whitespace() {
        if word
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c.is_ascii_punctuation())
        {
            // 纯英文/数字：整体作为词
            tokens.push(word.to_lowercase());
        } else {
            // 含中文：2-gram 分词
            let chars: Vec<char> = word.chars().collect();
            if chars.len() == 1 {
                tokens.push(chars[0].to_string());
            } else {
                for w in chars.windows(2) {
                    tokens.push(w.iter().collect::<String>());
                }
            }
        }
    }
    tokens
}

/// 测试用例
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keyword_search() {
        let mut fb = EmbeddingFallback::new();
        fb.index_texts(&["主人喜欢编程".to_string(), "主人喜欢Rust".to_string()]);
        println!("index: {:?}", fb.keyword_index);
        let r = fb.search("喜欢");
        println!("results: {:?}", r);
        assert_eq!(r.len(), 2);
    }

    #[test]
    fn test_exact_search() {
        let mut fb = EmbeddingFallback::new();
        fb.set_level(FallbackLevel::ExactMatch);
        fb.index_texts(&["hello".to_string()]);
        assert_eq!(fb.search("hello").len(), 1);
        assert_eq!(fb.search("world").len(), 0);
    }
}
