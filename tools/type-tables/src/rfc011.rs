//! T3：`from_builtin_name` 的规范名 vs RFC-011 的类型表。
//!
//! **文档侧口径（必须先读）**：RFC-011 里没有一张以「内置类型名」为列的表格（03 §5.7 假定它对标
//! RFC-013 的码表，但 RFC-013 是码表规范，RFC-011 是泛型系统设计）。因此本检查的文档侧口径是：
//! **RFC-011 全部 markdown 表格中的类型名 token** = 表格行里 inline code 包裹、且整体是
//! 「首字母大写 + 字母数字」的标识符（如 `Type1` 行的 `Int` / `String`）。
//! 另附「正文是否提及」作为信息列（word 级匹配），用于说明差异性质。

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::dispositions;
use crate::tables::BuiltinArm;
use crate::RFC011_REL;

/// markdown inline code 的定界符（U+0060）
const CODE_DELIM: char = '\u{0060}';

/// 一条差异
#[derive(Debug)]
pub struct Difference {
    pub check: &'static str,
    pub name: String,
    pub detail: String,
    pub disposition: Option<&'static dispositions::Disposition>,
}

/// T3 报告
#[derive(Debug)]
pub struct Rfc011Report {
    /// 文档类型表里的类型名 token → 行号
    pub doc_table_names: BTreeMap<String, Vec<usize>>,
    /// `from_builtin_name` 的规范名
    pub canonical: Vec<String>,
    /// 规范名 → 全文是否提及
    pub mentioned: BTreeMap<String, bool>,
    pub differences: Vec<Difference>,
}

impl Rfc011Report {
    pub fn unlisted(&self) -> usize {
        self.differences
            .iter()
            .filter(|d| d.disposition.is_none())
            .count()
    }

    pub fn render(&self) -> String {
        let doc_set: BTreeSet<&String> = self.doc_table_names.keys().collect();

        let mut out = String::new();
        out.push_str("── T3 与 RFC-011 一致 ─────────────────────────────────────────────────\n");
        out.push_str(&format!("T3.1 文档侧口径：文档 = {RFC011_REL}\n"));
        out.push_str("  文档侧 token = RFC-011 全部 markdown 表格行里、inline code 包裹且整体为\n");
        out.push_str("  「首字母大写 + 字母数字」的标识符（该文档没有以内置类型名为列的表格）。\n");
        out.push_str(&format!(
            "  实测：{} 个 token —— {}\n\n",
            self.doc_table_names.len(),
            self.doc_table_names
                .iter()
                .map(|(name, lines)| format!(
                    "{}(:{})",
                    name,
                    lines
                        .iter()
                        .map(|l| l.to_string())
                        .collect::<Vec<_>>()
                        .join(",")
                ))
                .collect::<Vec<_>>()
                .join(", ")
        ));

        out.push_str("T3.2 逐名比对（规范名 → 在类型表 / 全文提及）\n");
        out.push_str(&format!(
            "  {:<10} {:>10} {:>10}   判定\n",
            "名字", "在类型表", "全文提及"
        ));
        for name in &self.canonical {
            let in_doc = doc_set.contains(name);
            let mentioned = self.mentioned.get(name).copied().unwrap_or(false);
            let verdict = if in_doc {
                "ok"
            } else if mentioned {
                "差异（正文有、类型表无）"
            } else {
                "差异（文档未提）"
            };
            out.push_str(&format!(
                "  {:<10} {:>10} {:>10}   {}\n",
                name,
                if in_doc { "✓" } else { "-" },
                if mentioned { "✓" } else { "-" },
                verdict
            ));
        }
        out.push('\n');

        out.push_str("T3.3 差异清单（逐条处置）\n");
        out.push_str(&format!(
            "  差异 {} 条（文档独有 {} + 代码独有 {}）；已登记 {}；未登记 {}\n",
            self.differences.len(),
            self.differences
                .iter()
                .filter(|d| d.check == "T3.D2C")
                .count(),
            self.differences
                .iter()
                .filter(|d| d.check == "T3.C2D")
                .count(),
            self.differences.len() - self.unlisted(),
            self.unlisted()
        ));
        for d in &self.differences {
            out.push_str(&format!("  [{}] {}：{}\n", d.check, d.name, d.detail));
            match d.disposition {
                Some(disp) => {
                    out.push_str(&format!("        事实：{}\n", disp.reason));
                    out.push_str(&format!("        处置：{}\n", disp.action));
                }
                None => out.push_str(
                    "        处置：**未登记漂移**——必须登记事实与处置（dispositions.rs）\n",
                ),
            }
        }
        out.push('\n');
        out.push_str("T3.4 结论：");
        if self.differences.is_empty() {
            out.push_str("全绿（规范名与 RFC-011 类型表逐名一致）\n\n");
        } else if self.unlisted() == 0 {
            out.push_str(&format!(
                "非全绿：{} 条差异，全部已登记；T3 的「文档侧」当前不承载内置类型名清单（见 T3.1）\n\n",
                self.differences.len()
            ));
        } else {
            out.push_str(&format!(
                "非全绿：{} 条差异，其中 {} 条未登记（需补 dispositions.rs）\n\n",
                self.differences.len(),
                self.unlisted()
            ));
        }
        out
    }
}

/// T3 入口
pub fn check(
    root: &Path,
    builtin: &[BuiltinArm],
) -> Result<Rfc011Report, String> {
    let path = root.join(RFC011_REL);
    let text =
        std::fs::read_to_string(&path).map_err(|e| format!("读取 {} 失败: {e}", path.display()))?;

    let mut doc_table_names: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (idx, line) in text.lines().enumerate() {
        if !line.trim_start().starts_with('|') {
            continue;
        }
        for token in inline_code_identifiers(line) {
            doc_table_names.entry(token).or_default().push(idx + 1);
        }
    }
    if doc_table_names.is_empty() {
        return Err(format!(
            "{RFC011_REL} 的表格里没有解析到任何类型名 token（口径失效，需检查文档结构）"
        ));
    }

    let canonical: Vec<String> = builtin.iter().map(|a| a.canonical().to_string()).collect();
    let accepted: BTreeSet<String> = builtin
        .iter()
        .flat_map(|a| a.names.iter().cloned())
        .collect();

    let mut mentioned: BTreeMap<String, bool> = BTreeMap::new();
    for name in &canonical {
        mentioned.insert(name.clone(), mentions_word(&text, name));
    }

    let mut differences = Vec::new();
    // 文档 → 代码
    for (name, lines) in &doc_table_names {
        if accepted.contains(name) {
            continue;
        }
        differences.push(Difference {
            check: "T3.D2C",
            name: name.clone(),
            detail: format!(
                "RFC-011 类型表出现该名（行 {}），但 from_builtin_name 不解析",
                lines
                    .iter()
                    .map(|l| l.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            disposition: dispositions::lookup("T3.D2C", name),
        });
    }
    // 代码 → 文档
    for name in &canonical {
        if doc_table_names.contains_key(name) {
            continue;
        }
        let how = if mentioned.get(name).copied().unwrap_or(false) {
            "正文提及，但不在类型表"
        } else {
            "全文未提及"
        };
        differences.push(Difference {
            check: "T3.C2D",
            name: name.clone(),
            detail: format!("from_builtin_name 的规范名不在 RFC-011 类型表（{how}）"),
            disposition: dispositions::lookup("T3.C2D", name),
        });
    }

    Ok(Rfc011Report {
        doc_table_names,
        canonical,
        mentioned,
        differences,
    })
}

/// 抽出一行里的「inline code 包裹 + 整体是首字母大写标识符」的 token
fn inline_code_identifiers(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = line;
    while let Some(start) = rest.find(CODE_DELIM) {
        let after = &rest[start + CODE_DELIM.len_utf8()..];
        let Some(end) = after.find(CODE_DELIM) else {
            break;
        };
        let token = &after[..end];
        if is_type_name_token(token) {
            out.push(token.to_string());
        }
        rest = &after[end + CODE_DELIM.len_utf8()..];
    }
    out
}

fn is_type_name_token(token: &str) -> bool {
    let mut chars = token.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_uppercase() {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric())
}

/// word 级提及（前后字符都不是字母/数字/下划线）
fn mentions_word(
    text: &str,
    name: &str,
) -> bool {
    let bytes = text.as_bytes();
    let mut from = 0usize;
    while let Some(pos) = text[from..].find(name) {
        let begin = from + pos;
        let end = begin + name.len();
        let before_ok = begin == 0 || !is_word_byte(bytes[begin - 1]);
        let after_ok = end >= bytes.len() || !is_word_byte(bytes[end]);
        if before_ok && after_ok {
            return true;
        }
        from = end;
    }
    false
}

fn is_word_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

#[cfg(test)]
mod tests {
    use super::*;

    // 文档侧 token 抽取是 T3 的口径核心：只认「inline code + 首字母大写标识符」。
    #[test]
    fn test_inline_code_identifiers_extracts_only_capitalized_identifiers() {
        // Arrange：一行里混有表格分隔、普通类型名、带参构造与说明文字
        let line = "| `Type1` | 具体类型 | `Int`, `String`, `Array(Int, 3)` |";

        // Act
        let tokens = inline_code_identifiers(line);

        // Assert：带参构造（Array(...)）不算 token，其余按出现顺序保留
        assert_eq!(
            tokens,
            vec!["Type1", "Int", "String"],
            "只应抽出「整体是首字母大写标识符」的 inline code token"
        );
    }

    #[test]
    fn test_is_type_name_token_rejects_lowercase_and_operators() {
        assert!(is_type_name_token("Int"), "规范类型名应被接受");
        assert!(!is_type_name_token("int"), "小写别名不是类型表 token");
        assert!(
            !is_type_name_token("Array(Int, 3)"),
            "带参构造不是单个标识符"
        );
        assert!(!is_type_name_token("&T"), "记号/运算符不是标识符");
        assert!(!is_type_name_token(""), "空 token 不是类型名");
    }

    #[test]
    fn test_mentions_word_matches_whole_word_only() {
        let text = "Int32 与 int 都不是独立的 Int 提及；Int 才是。";
        assert!(mentions_word(text, "Int"), "完整单词 Int 应命中");
        assert!(
            !mentions_word("只写 Int32", "Int"),
            "Int32 不应算作 Int 的提及"
        );
        assert!(!mentions_word("只写 Int", "Int32"), "更长的名字不应命中");
    }
}
