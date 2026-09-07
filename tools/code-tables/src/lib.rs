//! 错误码注册表解析 / 校验 / RFC-013 码表生成 —— 单一解析源（#326）
//!
//! 两个消费者：
//! - build.rs（build-dependencies 引本 lib）：每次 cargo build 执行校验，error 即拒绝编译
//! - `code-tables` bin：`--check` 干跑比对 RFC-013 码表区间、`--fix` 就地重写
//!
//! 解析形态：条目行 = trim 后以 `("E####"` / `("W####"` 开头的行（define_codes! 条目
//! 与 std/result.rs 的 RUNTIME_ERROR_CODES 元组同形态，一个提取器通吃）。

use serde_json::Value;
use std::path::Path;

/// RFC-013 文档相对仓库根的路径
pub const RFC013_REL: &str = "docs/src/design/rfc/accepted/013-error-code-specification.md";
/// 人工翻译源（build.rs 门槛强制）；其余语言由 i18n bot 异步补齐
pub const HARD_LOCALES: &[&str] = &["zh"];
/// bot 目标语言（缺失降为 warning）
pub const BOT_LOCALES: &[&str] = &["en", "ja", "ru", "zh-classical", "zh-x-miao"];

/// RFC-013 码表段位：（标记键，千位前缀）。标题行在区间外手写，区间只含表格。
pub const RFC_SEGMENTS: &[(&str, &str)] = &[
    ("E0xxx", "E0"),
    ("E1xxx", "E1"),
    ("E2xxx", "E2"),
    ("E3xxx", "E3"),
    ("E4xxx", "E4"),
    ("E5xxx", "E5"),
    ("E6xxx", "E6"),
    ("E7xxx", "E7"),
    ("E8xxx", "E8"),
    ("W1xxx", "W1"),
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryEntry {
    pub code: String,
    /// 来源文件名（e1xxx.rs / result.rs），用于段位校验与错误定位
    pub source: String,
}

#[derive(Debug, Default)]
pub struct Report {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl Report {
    fn error(
        &mut self,
        msg: impl Into<String>,
    ) {
        self.errors.push(msg.into());
    }
    fn warn(
        &mut self,
        msg: impl Into<String>,
    ) {
        self.warnings.push(msg.into());
    }
    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }
}

/// 从条目行提取码：trim 后 `("E####"` / `("W####"` 开头
pub fn extract_code_from_entry_line(line: &str) -> Option<String> {
    let rest = line.trim_start().strip_prefix("(\"")?;
    let end = rest.find('"')?;
    let code = &rest[..end];
    if code.len() == 5
        && (code.starts_with('E') || code.starts_with('W'))
        && code[1..].bytes().all(|b| b.is_ascii_digit())
    {
        Some(code.to_string())
    } else {
        None
    }
}

/// 扫描注册源（codes/*.rs 的宏条目 + std/result.rs 的元组），返回全部注册码
pub fn parse_registry(root: &Path) -> Result<Vec<RegistryEntry>, String> {
    let mut entries = Vec::new();

    let codes_dir = root.join("src/util/diagnostic/codes");
    let mut files: Vec<_> = std::fs::read_dir(&codes_dir)
        .map_err(|e| format!("读取 {} 失败: {}", codes_dir.display(), e))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension().is_some_and(|x| x == "rs")
                && p.file_name().and_then(|n| n.to_str()).is_some_and(|n| {
                    (n.starts_with('e') || n.starts_with('w')) && n.ends_with("xxx.rs")
                })
        })
        .collect();
    files.sort();

    for path in files {
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let text = std::fs::read_to_string(&path)
            .map_err(|e| format!("读取 {} 失败: {}", path.display(), e))?;
        for (i, line) in text.lines().enumerate() {
            if let Some(code) = extract_code_from_entry_line(line) {
                entries.push(RegistryEntry {
                    code,
                    source: format!("{name}:{}", i + 1),
                });
            }
        }
    }

    let result_rs = root.join("src/std/result.rs");
    if result_rs.exists() {
        let text = std::fs::read_to_string(&result_rs)
            .map_err(|e| format!("读取 {} 失败: {}", result_rs.display(), e))?;
        for (i, line) in text.lines().enumerate() {
            if let Some(code) = extract_code_from_entry_line(line) {
                entries.push(RegistryEntry {
                    code,
                    source: format!("result.rs:{}", i + 1),
                });
            }
        }
    }

    Ok(entries)
}

/// 加载 locales/{lang}.json（build.rs 与 bin 共用）
pub fn load_locale(
    root: &Path,
    lang: &str,
) -> Result<Value, String> {
    load_json(&root.join("locales").join(format!("{lang}.json")))
}

fn load_json(path: &Path) -> Result<Value, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("读取 {} 失败: {}", path.display(), e))?;
    serde_json::from_str(&text).map_err(|e| format!("解析 {} 失败: {}", path.display(), e))
}

/// 全量校验：唯一性、段位合法性、locales 完备性（zh 硬 / bot 语言软）与孤儿 key
pub fn validate(
    root: &Path,
    entries: &[RegistryEntry],
) -> Result<Report, String> {
    let mut report = Report::default();

    // 唯一性
    for i in 0..entries.len() {
        for j in (i + 1)..entries.len() {
            if entries[i].code == entries[j].code {
                report.error(format!(
                    "重复定义: {} 出现于 {} 与 {}",
                    entries[i].code, entries[i].source, entries[j].source
                ));
            }
        }
    }

    // 段位合法性：codes 文件名段位与码千位一致；result.rs 仅收 E6xxx（运行时值码）
    for e in entries {
        if e.source.starts_with("result.rs") {
            if !e.code.starts_with("E6") {
                report.error(format!(
                    "段位错误: {} 在 result.rs（运行时值码仅收 E6xxx）",
                    e.code
                ));
            }
            continue;
        }
        let file = e.source.split(':').next().unwrap_or_default();
        let seg = match file.chars().nth(1) {
            Some(s) => s,
            None => {
                report.error(format!("无法解析来源文件段位: {}", e.source));
                continue;
            }
        };
        // 只比数字千位（文件名小写 e/w 与码大写 E/W 天然不同）
        if e.code.as_bytes().get(1) != Some(&(seg as u8)) {
            report.error(format!(
                "段位错误: {} 定义在 {}（千位段应为 {}）",
                e.code, file, seg
            ));
        }
    }

    // locales 完备性
    let locales_dir = root.join("locales");
    let zh: Value = load_json(&locales_dir.join("zh.json"))?;

    for e in entries {
        match zh.get(e.code.as_str()) {
            None => report.error(format!("zh.json 缺条目: {}（{}）", e.code, e.source)),
            Some(v) => {
                let title_ok = v
                    .get("title")
                    .and_then(|t| t.as_str())
                    .is_some_and(|s| !s.is_empty());
                if !title_ok {
                    report.error(format!("zh.json 条目 {}: title 缺失或为空", e.code));
                }
                if v.get("template").is_none() {
                    report.error(format!("zh.json 条目 {}: 缺 template 字段", e.code));
                }
            }
        }
    }

    // 孤儿 key（实现无定义）：码已删但翻译残留——bot 不会清理，任何语言都是 error
    for lang in std::iter::once("zh").chain(BOT_LOCALES.iter().copied()) {
        let v: Value = load_json(&locales_dir.join(format!("{lang}.json")))?;
        if let Some(map) = v.as_object() {
            for key in map.keys() {
                if is_code_key(key) && !entries.iter().any(|e| e.code == *key) {
                    report.error(format!(
                        "locales 孤立: {key} 在 {lang}.json 有模板但实现未定义"
                    ));
                }
            }
        }
    }

    // bot 语言缺失：降为 warning（推送 → bot 落地的窗口期不阻塞）
    for lang in BOT_LOCALES {
        let v: Value = load_json(&locales_dir.join(format!("{lang}.json")))?;
        for e in entries {
            if v.get(e.code.as_str()).is_none() {
                report.warn(format!(
                    "locales 缺失: {} 未在 {lang}.json（bot 将异步补齐）",
                    e.code
                ));
            }
        }
    }

    Ok(report)
}

fn is_code_key(s: &str) -> bool {
    s.len() == 5
        && (s.starts_with('E') || s.starts_with('W'))
        && s[1..].bytes().all(|b| b.is_ascii_digit())
}

/// 生成指定段位的码表 markdown（表头 + 按 code 升序的行，两列：代码 / 说明（zh title））
pub fn render_segment(
    segment_key: &str,
    entries: &[RegistryEntry],
    zh: &Value,
) -> Option<String> {
    let prefix = RFC_SEGMENTS
        .iter()
        .find(|(k, _)| *k == segment_key)
        .map(|(_, p)| p.to_string())?;

    let mut codes: Vec<&RegistryEntry> = entries
        .iter()
        .filter(|e| e.code.starts_with(&prefix))
        .collect();
    codes.sort_by(|a, b| a.code.cmp(&b.code));
    if codes.is_empty() {
        return None;
    }

    let title = |code: &str| -> String {
        zh.get(code)
            .and_then(|v| v.get("title"))
            .and_then(|t| t.as_str())
            .unwrap_or("?")
            .to_string()
    };

    let rows: Vec<(String, String)> = codes
        .iter()
        .map(|e| (e.code.clone(), title(&e.code)))
        .collect();
    let w0 = rows.iter().map(|r| r.0.chars().count()).max().unwrap_or(4);
    let w1 = rows
        .iter()
        .map(|r| r.1.chars().count())
        .max()
        .unwrap_or(4)
        .max(4);

    let mut out = String::new();
    out.push_str(&format!("| 代码  | 说明{} |\n", " ".repeat(w1 - 4)));
    out.push_str(&format!("| ----- | {} |\n", "-".repeat(w1)));
    for (code, desc) in &rows {
        out.push_str(&format!(
            "| {code}{} | {desc}{} |\n",
            " ".repeat(w0 - code.chars().count()),
            " ".repeat(w1 - desc.chars().count())
        ));
    }
    // 行尾空格清理（markdownlint）
    let out = out
        .lines()
        .map(|l| l.trim_end().to_string())
        .collect::<Vec<_>>()
        .join("\n");
    Some(out)
}

/// 比对 RFC-013 各标记区间与生成内容；返回 (段, 原因) 错误清单
pub fn check_rfc_tables(
    doc: &str,
    entries: &[RegistryEntry],
    zh: &Value,
) -> Vec<String> {
    let mut errors = Vec::new();
    for (seg, _) in RFC_SEGMENTS {
        let start = format!("<!-- code-table:{seg} start -->");
        let end = format!("<!-- code-table:{seg} end -->");
        let Some(s) = doc.find(&start) else {
            errors.push(format!("{seg}: 缺少区间起始标记 {start}"));
            continue;
        };
        let Some(e) = doc[s..].find(&end) else {
            errors.push(format!("{seg}: 缺少区间结束标记 {end}"));
            continue;
        };
        let actual = doc[s + start.len()..s + e].trim().to_string();
        match render_segment(seg, entries, zh) {
            Some(expected) => {
                if actual != expected {
                    errors.push(format!(
                        "{seg}: 码表区间与注册表不一致（运行 code-tables --fix 治愈）"
                    ));
                }
            }
            None => {
                if !actual.is_empty() {
                    errors.push(format!("{seg}: 该段无注册码但区间非空（--fix 清空）"));
                }
            }
        }
    }
    errors
}

/// 就地重写 RFC-013 各标记区间；返回重写的段数
pub fn fix_rfc_tables(
    doc: &str,
    entries: &[RegistryEntry],
    zh: &Value,
) -> Result<(String, usize), String> {
    let mut out = doc.to_string();
    let mut rewritten = 0;
    for (seg, _) in RFC_SEGMENTS {
        let start = format!("<!-- code-table:{seg} start -->");
        let end = format!("<!-- code-table:{seg} end -->");
        let s = out
            .find(&start)
            .ok_or_else(|| format!("{seg}: 缺少区间起始标记"))?;
        let e_rel = out[s..]
            .find(&end)
            .ok_or_else(|| format!("{seg}: 缺少区间结束标记"))?;
        let body = match render_segment(seg, entries, zh) {
            Some(text) => text,
            None => String::new(),
        };
        let replacement = format!("{start}\n{body}\n{end}");
        if out[s..s + e_rel + end.len()] != replacement {
            out.replace_range(s..s + e_rel + end.len(), &replacement);
        }
        rewritten += 1;
    }
    Ok((out, rewritten))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_code_from_entry_line() {
        assert_eq!(
            extract_code_from_entry_line(
                "    (\"E1001\", TypeCheck, foo(x: &str) => .param(\"name\", x)),"
            ),
            Some("E1001".to_string())
        );
        assert_eq!(
            extract_code_from_entry_line("    (\"E6009\", \"invalid range step\"),"),
            Some("E6009".to_string())
        );
        // 非条目行
        assert_eq!(
            extract_code_from_entry_line("        .param(\"name\", x)"),
            None
        );
        assert_eq!(
            extract_code_from_entry_line("codes.extend_from_slice(e1xxx::E1XXX);"),
            None
        );
        assert_eq!(
            extract_code_from_entry_line("    (\"E10\", TypeCheck),"),
            None
        );
        assert_eq!(
            extract_code_from_entry_line("    (\"EBAD1\", TypeCheck),"),
            None
        );
    }

    #[test]
    fn test_render_segment_two_columns_sorted() {
        let zh: Value = serde_json::from_str(
            r#"{"E6003": {"title": "数组索引越界"}, "E6001": {"title": "除零错误"}}"#,
        )
        .unwrap();
        let entries = vec![
            RegistryEntry {
                code: "E6003".into(),
                source: "e6xxx.rs:1".into(),
            },
            RegistryEntry {
                code: "E6001".into(),
                source: "e6xxx.rs:2".into(),
            },
        ];
        let out = render_segment("E6xxx", &entries, &zh).unwrap();
        let lines: Vec<&str> = out.lines().collect();
        // 列宽按最长 title（数组索引越界 = 6 字符）pad，行按 code 升序
        assert_eq!(lines[2], "| E6001 | 除零错误   |");
        assert_eq!(lines[3], "| E6003 | 数组索引越界 |");
    }

    #[test]
    fn test_check_and_fix_roundtrip() {
        let doc: String = RFC_SEGMENTS
            .iter()
            .map(|(seg, _)| {
                format!("<!-- code-table:{seg} start -->\n\n<!-- code-table:{seg} end -->")
            })
            .collect::<Vec<_>>()
            .join("\n");
        let entries: Vec<RegistryEntry> = vec![];
        assert!(check_rfc_tables(&doc, &entries, &serde_json::Value::Null).is_empty());
        let (fixed, n) = fix_rfc_tables(&doc, &entries, &serde_json::Value::Null).unwrap();
        assert_eq!(n, 10);
        assert!(check_rfc_tables(&fixed, &entries, &serde_json::Value::Null).is_empty());
    }

    #[test]
    fn test_check_detects_drift() {
        let drifted = "<!-- code-table:E1xxx start -->\n| 代码  | 说明 |\n| ----- | ---- |\n| E1001 | 被手改的行 |\n<!-- code-table:E1xxx end -->";
        let doc: String = RFC_SEGMENTS
            .iter()
            .map(|(seg, _)| {
                if *seg == "E1xxx" {
                    drifted.to_string()
                } else {
                    format!("<!-- code-table:{seg} start -->\n\n<!-- code-table:{seg} end -->")
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        let zh: Value = serde_json::from_str(r#"{"E1001": {"title": "未知变量"}}"#).unwrap();
        let entries = vec![RegistryEntry {
            code: "E1001".into(),
            source: "e1xxx.rs:1".into(),
        }];
        let errs = check_rfc_tables(&doc, &entries, &zh);
        assert!(errs.iter().any(|e| e.contains("E1xxx")));
        // --fix 治愈
        let (fixed, _) = fix_rfc_tables(&doc, &entries, &zh).unwrap();
        assert!(check_rfc_tables(&fixed, &entries, &zh).is_empty());
    }
}
