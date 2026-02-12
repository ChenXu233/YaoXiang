//! 智能建议引擎
//!
//! 提供变量拼写建议、类型推断建议等智能辅助功能

use std::collections::{HashMap, HashSet};
use std::cmp::Ordering;

/// 相似度阈值
const SIMILARITY_THRESHOLD: f64 = 0.5;

/// 建议类型
#[derive(Debug, Clone)]
pub enum Suggestion {
    /// 变量名拼写建议
    Variable { typo: String, suggestions: Vec<String> },
    /// 类型建议
    Type { expected: String, found: String, suggestion: String },
    /// 函数调用建议
    Function { name: String, suggestions: Vec<(String, usize)> },
    /// 模块导入建议
    Module { name: String, suggestions: Vec<String> },
    /// 通用帮助
    Generic { message: String },
}

/// 智能建议引擎
pub struct SuggestionEngine {
    /// 已定义的名字（变量、函数等）- 存储Owned字符串
    defined_names: HashSet<String>,
    /// 相似度缓存
    similarity_cache: HashMap<String, Vec<(String, f64)>>,
    /// 名称到类型的映射（用于更精确的建议）
    name_to_types: HashMap<String, String>,
}

impl SuggestionEngine {
    /// 创建新的建议引擎
    pub fn new() -> Self {
        Self {
            defined_names: HashSet::new(),
            similarity_cache: HashMap::new(),
            name_to_types: HashMap::new(),
        }
    }

    /// 从作用域创建建议引擎
    pub fn from_scope(scope: &[&str]) -> Self {
        let mut engine = Self::new();
        for &name in scope {
            engine.defined_names.insert(name.to_string());
        }
        engine
    }

    /// 添加定义的名字
    pub fn add_defined_name(&mut self, name: &str) {
        self.defined_names.insert(name.to_string());
    }

    /// 添加名字到类型的映射
    pub fn add_name_type(&mut self, name: &str, ty: &str) {
        self.name_to_types.insert(name.to_string(), ty.to_string());
    }

    /// 查找相似的名字
    pub fn find_similar(&self, name: &str) -> Vec<(String, f64)> {
        // 检查缓存
        if let Some(cached) = self.similarity_cache.get(name) {
            return cached.clone();
        }

        let mut suggestions: Vec<(String, f64)> = self.defined_names
            .iter()
            .map(|def_name| (def_name.clone(), self.similarity(name, def_name)))
            .filter(|(_, score)| *score >= SIMILARITY_THRESHOLD)
            .collect();

        // 按相似度降序排序
        suggestions.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
        suggestions.truncate(5); // 最多返回5个建议

        suggestions
    }

    /// 计算两个字符串的相似度
    fn similarity(&self, s1: &str, s2: &str) -> f64 {
        if s1 == s2 {
            return 1.0;
        }

        let dist = self.levenshtein_distance(s1, s2);
        let max_len = s1.len().max(s2.len());

        if max_len == 0 {
            return 1.0;
        }

        // 使用改进的相似度公式
        let base_similarity = 1.0 - (dist as f64 / max_len as f64);

        // 如果有共同前缀，给予额外奖励
        let prefix_bonus = if !s1.is_empty() && !s2.is_empty()
            && (s1.starts_with(&s2[..1]) || s2.starts_with(&s1[..1])) {
            0.1
        } else {
            0.0
        };

        (base_similarity + prefix_bonus).min(1.0)
    }

    /// Levenshtein 编辑距离
    fn levenshtein_distance(&self, a: &str, b: &str) -> usize {
        if a.is_empty() {
            return b.len();
        }
        if b.is_empty() {
            return a.len();
        }

        let a_chars: Vec<char> = a.chars().collect();
        let b_chars: Vec<char> = b.chars().collect();

        let a_len = a_chars.len();
        let b_len = b_chars.len();

        // 使用行优化
        let mut prev_row: Vec<usize> = (0..=b_len).collect();
        let mut curr_row: Vec<usize> = Vec::with_capacity(b_len + 1);

        for i in 1..=a_len {
            curr_row.clear();
            curr_row.push(i);

            for j in 1..=b_len {
                let cost = if a_chars[i - 1] == b_chars[j - 1] { 0 } else { 1 };
                let value = prev_row[j]
                    .min(curr_row[j - 1] + 1)
                    .min(prev_row[j - 1] + cost);
                curr_row.push(value);
            }

            std::mem::swap(&mut prev_row, &mut curr_row);
        }

        prev_row[b_len]
    }

    /// 为未知变量生成建议
    pub fn suggest_for_unknown_variable(&self, name: &str) -> Option<Suggestion> {
        let similar = self.find_similar(name);

        if similar.is_empty() {
            return None;
        }

        let suggestions: Vec<String> = similar
            .into_iter()
            .map(|(name, _)| name)
            .collect();

        Some(Suggestion::Variable {
            typo: name.to_string(),
            suggestions,
        })
    }

    /// 清空缓存
    pub fn clear_cache(&mut self) {
        self.similarity_cache.clear();
    }

    /// 获取定义的名字数量
    pub fn len(&self) -> usize {
        self.defined_names.len()
    }

    /// 检查是否为空
    pub fn is_empty(&self) -> bool {
        self.defined_names.is_empty()
    }
}

impl Default for SuggestionEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// 为 Diagnostic 提供建议方法
impl crate::util::diagnostic::Diagnostic {
    /// 获取针对此错误的建议
    pub fn get_suggestions(&self) -> Option<Vec<Suggestion>> {
        match self {
            // 这里可以添加更多特定错误的建议
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_similarity_same() {
        let engine = SuggestionEngine::new();
        assert_eq!(engine.similarity("hello", "hello"), 1.0);
    }

    #[test]
    fn test_similarity_different() {
        let engine = SuggestionEngine::new();
        let sim = engine.similarity("hello", "hallo");
        assert!(sim > 0.7, "Expected similarity > 0.7, got {}", sim);
    }

    #[test]
    fn test_similarity_completely_different() {
        let engine = SuggestionEngine::new();
        let sim = engine.similarity("abc", "xyz");
        assert!(sim < 0.5, "Expected low similarity, got {}", sim);
    }

    #[test]
    fn test_find_similar() {
        let mut engine = SuggestionEngine::new();
        engine.add_defined_name("variable");
        engine.add_defined_name("variant");
        engine.add_defined_name("value");
        engine.add_defined_name("valley");

        let similar = engine.find_similar("varible");
        assert!(!similar.is_empty(), "Should find similar names");

        // 应该有 "variable" 或 "variant"
        let names: Vec<String> = similar.iter().map(|(n, _)| n.clone()).collect();
        assert!(names.contains(&"variable".to_string()) || names.contains(&"variant".to_string()));
    }

    #[test]
    fn test_levenshtein() {
        let engine = SuggestionEngine::new();

        assert_eq!(engine.levenshtein_distance("kitten", "sitting"), 3);
        assert_eq!(engine.levenshtein_distance("", ""), 0);
        assert_eq!(engine.levenshtein_distance("abc", ""), 3);
        assert_eq!(engine.levenshtein_distance("", "abc"), 3);
        assert_eq!(engine.levenshtein_distance("abc", "abc"), 0);
    }

    #[test]
    fn test_from_scope() {
        let engine = SuggestionEngine::from_scope(&["foo", "bar", "baz"]);
        assert_eq!(engine.len(), 3);
        assert!(!engine.is_empty());
    }
}
