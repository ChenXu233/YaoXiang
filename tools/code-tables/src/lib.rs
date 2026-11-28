//! 错误码注册表解析 / 校验 / RFC-013 码表生成 —— 单一解析源（#326）
//!
//! 两个消费者：
//! - build.rs（build-dependencies 引本 lib）：每次 cargo build 执行校验，error 即拒绝编译
//! - `code-tables` bin：`--check` 干跑比对 RFC-013 码表区间、`--fix` 就地重写
//!
//! 解析形态：条目行 = trim 后以 `("E####"` / `("W####"` 开头的行（define_codes! 条目
//! 与 std/result.rs 的 RUNTIME_ERROR_CODES 元组同形态，一个提取器通吃）。

use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;

/// RFC-013 文档相对仓库根的路径
pub const RFC013_REL: &str = "docs/src/rfc/accepted/013-error-code-specification.md";
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

// ============================================================================
// opcode 权威表门禁（P10 S6.2 / F7+F8）
// ============================================================================
//
// 判据与 `src/middle/bytecode/opcode.rs` 的声明面同源：该文件既写
// `pub const NAME: u8 = 0xNN;` 常量，又写 `OPCODE_INFO` 权威表
// （`OpcodeInfo { code: NAME, name: "Display", size: N }`）。本门禁把两份事实逐项
// 对账，任一不成立即 `Err`（build.rs 直接 panic 拒绝编译）：
// - **解析面非空**：两个提取器都要有产出——判据不成立时不得降级成「通过」；
// - **码值唯一**：两个常量同码 ⇒ 解码臂必有一支是死的、反查必有一支走错；
// - **每个常量有表项**：漏登记的 opcode 查不到转储名/长度（`BytecodeInstr::size` 会 panic）；
// - **表项指向已声明常量**：常量名写错 ⇒ 表项成死数据；
// - **表项名非空且唯一**：转储名是产物里唯一的人读标识，重复即无法区分；
// - **表项按 code 严格升序**：与常量声明同序，便于人工对账；乱序多为复制粘贴错。

/// opcode 词表与权威表所在文件（相对仓库根）
pub const OPCODE_REL: &str = "src/middle/bytecode/opcode.rs";

/// `pub const NAME: u8 = 0xNN;` 形式的一条 opcode 声明
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpcodeConst {
    pub name: String,
    pub code: u8,
    /// 声明所在行（1 起，供诊断定位）
    pub line: usize,
}

/// `OpcodeInfo { code: NAME, name: "Display", size: N }` 形式的一条权威表项
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpcodeTableRow {
    /// `code:` 后的常量名（与 [`OpcodeConst::name`] 对齐）
    pub name: String,
    /// `name:` 的转储名（人读标识）
    pub display: String,
    /// `size:` 的固定字节数（含 opcode 字节）
    pub size: usize,
    /// 表项所在行（1 起，供诊断定位）
    pub line: usize,
}

/// 解析 `pub const NAME: u8 = 0xNN;` 声明面（逐行，行注释后的内容忽略）
pub fn parse_opcode_consts(src: &str) -> Vec<OpcodeConst> {
    let mut out = Vec::new();
    for (idx, raw) in src.lines().enumerate() {
        let code_part = raw.split("//").next().unwrap_or("");
        let line = code_part.trim();
        let Some(rest) = line.strip_prefix("pub const ") else {
            continue;
        };
        let Some((name, tail)) = rest.split_once(':') else {
            continue;
        };
        let Some(tail) = tail.trim_start().strip_prefix("u8") else {
            continue;
        };
        let Some(tail) = tail.trim_start().strip_prefix('=') else {
            continue;
        };
        let Some(hex) = tail.trim().strip_suffix(';') else {
            continue;
        };
        let Some(hex) = hex.trim().strip_prefix("0x") else {
            continue;
        };
        let Ok(code) = u8::from_str_radix(hex.trim(), 16) else {
            continue;
        };
        out.push(OpcodeConst {
            name: name.trim().to_string(),
            code,
            line: idx + 1,
        });
    }
    out
}

/// 解析 `OpcodeInfo { code: NAME, name: "Display", size: N }` 表项面。
///
/// 容忍 rustfmt 的换行：先找 `OpcodeInfo`，要求其后（跳过空白）紧跟 `{`，
/// 再用花括号配对取出条目体；`pub struct OpcodeInfo {` 的定义体以字段文档开头，
/// 因而不满足 `code:` 起始的判据而被跳过。
pub fn parse_opcode_table(src: &str) -> Vec<OpcodeTableRow> {
    let mut out = Vec::new();
    let mut search_from = 0usize;
    while let Some(rel) = src[search_from..].find("OpcodeInfo") {
        let token_start = search_from + rel;
        search_from = token_start + "OpcodeInfo".len();
        let after = &src[search_from..];
        let trimmed = after.trim_start();
        if !trimmed.starts_with('{') {
            continue;
        }
        let body_start = search_from + (after.len() - trimmed.len()) + 1;
        let Some(body) = take_balanced_body(src, body_start) else {
            continue;
        };
        if !body.trim_start().starts_with("code:") {
            continue;
        }
        let mut name = None;
        let mut display = None;
        let mut size = None;
        for part in body.split(',') {
            let Some((key, value)) = part.split_once(':') else {
                continue;
            };
            let value = value.trim();
            match key.trim() {
                "code" => name = Some(value.to_string()),
                "name" => display = unquote(value),
                "size" => size = value.parse::<usize>().ok(),
                _ => {}
            }
        }
        let (Some(name), Some(display), Some(size)) = (name, display, size) else {
            continue;
        };
        out.push(OpcodeTableRow {
            name,
            display,
            size,
            line: src[..token_start].matches('\n').count() + 1,
        });
    }
    out
}

/// 从 `body_start`（`{` 之后）取花括号配对的条目体；未闭合返回 `None`
fn take_balanced_body(
    src: &str,
    body_start: usize,
) -> Option<&str> {
    let mut depth = 1usize;
    for (off, ch) in src[body_start..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&src[body_start..body_start + off]);
                }
            }
            _ => {}
        }
    }
    None
}

/// 去双引号；非字符串字面量返回 `None`（该表项随后被「缺表项」判据抓到）
fn unquote(value: &str) -> Option<String> {
    let inner = value.strip_prefix('"')?.strip_suffix('"')?;
    Some(inner.to_string())
}

/// 表 ↔ 声明面的一致性判据；返回空 = 通过
pub fn check_opcode_table(
    consts: &[OpcodeConst],
    rows: &[OpcodeTableRow],
) -> Vec<String> {
    let mut errors = Vec::new();
    if consts.is_empty() {
        errors.push(
            "解析面为空：opcode.rs 里没有解析到任何 `pub const NAME: u8 = 0xNN;` 声明".to_string(),
        );
    }
    if rows.is_empty() {
        errors.push(
            "解析面为空：opcode.rs 里没有解析到任何 `OpcodeInfo { code: … }` 表项".to_string(),
        );
    }

    // 码值唯一：同码意味着解码臂必有一支是死的、反查必有一支走错
    for i in 0..consts.len() {
        for j in (i + 1)..consts.len() {
            if consts[i].code == consts[j].code {
                errors.push(format!(
                    "码值重复：{}（第 {} 行）与 {}（第 {} 行）都是 0x{:02X}——一个字节只能有一个解码语义",
                    consts[i].name, consts[i].line, consts[j].name, consts[j].line, consts[i].code
                ));
            }
        }
    }

    let by_name: HashMap<&str, &OpcodeConst> =
        consts.iter().map(|c| (c.name.as_str(), c)).collect();
    let mut display_lines: HashMap<&str, usize> = HashMap::new();
    let mut referenced: HashMap<&str, usize> = HashMap::new();
    for row in rows {
        if row.display.trim().is_empty() {
            errors.push(format!(
                "表项名称为空：{}（第 {} 行）的 `name` 必须非空——转储与诊断靠它识别",
                row.name, row.line
            ));
        }
        if let Some(prev) = display_lines.insert(row.display.as_str(), row.line) {
            errors.push(format!(
                "表项名称重复：\"{}\" 同时出现在第 {} 与 {} 行",
                row.display, prev, row.line
            ));
        }
        match by_name.get(row.name.as_str()) {
            None => errors.push(format!(
                "表项引用了未声明的常量：{}（第 {} 行）",
                row.name, row.line
            )),
            Some(_) => {
                if let Some(prev) = referenced.insert(row.name.as_str(), row.line) {
                    errors.push(format!(
                        "常量 {} 被登记了两次（第 {} 与 {} 行）",
                        row.name, prev, row.line
                    ));
                }
            }
        }
    }

    // 每个常量有表项：漏登记 = 转储名/长度查不到
    for c in consts {
        if !referenced.contains_key(c.name.as_str()) {
            errors.push(format!(
                "常量 {}（0x{:02X}，第 {} 行）没有权威表项：新增 opcode 必须同提交在 OPCODE_INFO 登记转储名与固定长度",
                c.name, c.code, c.line
            ));
        }
    }

    // 表项按 code 严格升序
    let mut prev: Option<(u8, &str, usize)> = None;
    for row in rows {
        let Some(c) = by_name.get(row.name.as_str()) else {
            continue;
        };
        if let Some((prev_code, prev_name, _)) = prev {
            if c.code <= prev_code {
                errors.push(format!(
                    "表项未按 code 升序：{}（0x{:02X}，第 {} 行）排在 {}（0x{:02X}）之后",
                    row.name, c.code, row.line, prev_name, prev_code
                ));
            }
        }
        prev = Some((c.code, row.name.as_str(), row.line));
    }

    errors
}

/// `build.rs` 入口：读 `OPCODE_REL` 并跑判据。
///
/// - `Ok(摘要)`：通过——`build.rs` 打印为一行 `cargo:warning` 信息；
/// - `Err(明细)`：违规——`build.rs` 必须 `panic!` 使构建失败，明细逐条含数量与行号。
///
/// 读取失败也算 `Err`：解析面不完整时判定不成立，不得降级成「通过」（防静默降级）。
pub fn opcode_gate(root: &Path) -> Result<String, String> {
    let path = root.join(OPCODE_REL);
    let src = std::fs::read_to_string(&path)
        .map_err(|e| format!("读取 {} 失败: {}", path.display(), e))?;
    let consts = parse_opcode_consts(&src);
    let rows = parse_opcode_table(&src);
    let errors = check_opcode_table(&consts, &rows);
    if errors.is_empty() {
        Ok(format!(
            "opcode 表门禁 F8 通过：{} 个常量 / {} 个表项（码值唯一、每常量有表项、表项名非空且唯一、按 code 升序）",
            consts.len(),
            rows.len()
        ))
    } else {
        Err(errors.join("\n  "))
    }
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
    #[test]
    fn test_parse_opcode_consts_reads_u8_hex_only() {
        let src = "\
pub const NOP: u8 = 0x00;
pub const LABEL: u8 = 0x0B;
// pub const RECYCLED: u8 = 0x06;
pub const NOT_OPCODE: u32 = 0x10;
pub const NO_INIT: u8;
";
        let consts = parse_opcode_consts(src);
        assert_eq!(
            consts.len(),
            2,
            "只收 `pub const NAME: u8 = 0xNN;` 形态（注释/非 u8/无初值都不算），实际: {consts:?}"
        );
        assert_eq!(consts[0].name, "NOP", "声明名解析错");
        assert_eq!(consts[0].code, 0x00, "码值解析错");
        assert_eq!(consts[0].line, 1, "行号应为 1（1 起）");
        assert_eq!(consts[1].name, "LABEL", "第二项声明名解析错");
    }

    #[test]
    fn test_parse_opcode_table_skips_struct_definition() {
        let src = "\
pub struct OpcodeInfo {
    /// 码值
    pub code: u8,
    pub name: &'static str,
}

pub static OPCODE_INFO: &[OpcodeInfo] = &[
    OpcodeInfo {
        code: NOP,
        name: \"Nop\",
        size: 1,
    },
];
";
        let rows = parse_opcode_table(src);
        assert_eq!(
            rows.len(),
            1,
            "结构体定义与类型引用都不得被当成表项，实际: {rows:?}"
        );
        assert_eq!(rows[0].name, "NOP", "表项常量名解析错");
        assert_eq!(rows[0].display, "Nop", "转储名解析错");
        assert_eq!(rows[0].size, 1, "固定长度解析错");
    }

    #[test]
    fn test_check_opcode_table_accepts_consistent_pair() {
        let consts =
            parse_opcode_consts("pub const NOP: u8 = 0x00;\npub const RETURN: u8 = 0x01;\n");
        let rows = parse_opcode_table(
            "pub static OPCODE_INFO: &[OpcodeInfo] = &[\n    OpcodeInfo { code: NOP, name: \"Nop\", size: 1 },\n    OpcodeInfo { code: RETURN, name: \"Return\", size: 1 },\n];\n",
        );
        let errors = check_opcode_table(&consts, &rows);
        assert!(errors.is_empty(), "一致的表不应报错，实际: {errors:?}");
    }

    #[test]
    fn test_check_opcode_table_detects_duplicate_code() {
        let consts = parse_opcode_consts("pub const NOP: u8 = 0x00;\npub const DUP: u8 = 0x00;\n");
        let rows = parse_opcode_table(
            "pub static OPCODE_INFO: &[OpcodeInfo] = &[\n    OpcodeInfo { code: NOP, name: \"Nop\", size: 1 },\n    OpcodeInfo { code: DUP, name: \"Dup\", size: 1 },\n];\n",
        );
        let errors = check_opcode_table(&consts, &rows);
        assert!(
            errors.iter().any(|e| e.contains("码值重复")),
            "两个常量同码必须报「码值重复」，实际: {errors:?}"
        );
        assert!(
            errors
                .iter()
                .any(|e| e.contains("NOP") && e.contains("DUP")),
            "重复诊断必须同时指名两个常量，实际: {errors:?}"
        );
    }

    #[test]
    fn test_check_opcode_table_detects_missing_entry() {
        let consts =
            parse_opcode_consts("pub const NOP: u8 = 0x00;\npub const RETURN: u8 = 0x01;\n");
        let rows = parse_opcode_table(
            "pub static OPCODE_INFO: &[OpcodeInfo] = &[\n    OpcodeInfo { code: NOP, name: \"Nop\", size: 1 },\n];\n",
        );
        let errors = check_opcode_table(&consts, &rows);
        assert!(
            errors
                .iter()
                .any(|e| e.contains("没有权威表项") && e.contains("RETURN")),
            "漏登记的常量必须被点名（含诊断定位），实际: {errors:?}"
        );
    }

    #[test]
    fn test_check_opcode_table_rejects_empty_parse_surface() {
        let errors = check_opcode_table(&[], &[]);
        assert!(
            errors.iter().filter(|e| e.contains("解析面为空")).count() == 2,
            "两侧解析面为空都必须报错（不得降级成通过），实际: {errors:?}"
        );
    }
}
