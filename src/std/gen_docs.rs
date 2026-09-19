//! 标准库 API 文档生成器（混合模式）
//!
//! `docs/src/reference/stdlib/*.md` 采用**生成 + 手写**混合结构：
//!
//! - **生成区**（标记区间内）：函数一览表、每函数的签名块——由
//!   `StdModule::exports()` 派生，签名逐字节来自 `NativeExport::signature`，
//!   不可能写错。漂移即门禁红，治愈走 `cargo run --example gen-stdlib-docs`
//! - **手写区**（标记区间外）：模块概述、借用/移动语义、错误模型、已知坑，
//!   以及可运行示例——这部分需要人的判断，不生成
//!
//! 与 `gen_interfaces`（RFC-037 `.yx` 接口视图）、`tools/code-tables`
//! （RFC-013 码表区间）同构：单一源 → 派生文件 → 漂移门禁 → 治愈工具。
//!
//! 标记形态：
//!
//! ```text
//! <!-- stdlib:table:string start -->
//! | 函数 | 签名 |
//! | ---- | ---- |
//! | `split` | `(s: &String, sep: &String) -> List(String)` |
//! <!-- stdlib:table:string end -->
//!
//! <!-- stdlib:sig:string.split start -->
//! ```yaoxiang
//! split: (s: &String, sep: &String) -> List(String)
//! ```
//! <!-- stdlib:sig:string.split end -->
//! ```

use std::path::Path;

use crate::std::StdModule;

/// 文档目录（相对仓库根）
pub const STDLIB_DOCS_REL: &str = "docs/src/reference/stdlib";

/// 文档化范围：全部面向用户的 std 模块。
///
/// 比 `gen_interfaces::generate_all_interfaces()` 多出 `result` / `range` /
/// `assert` / `weak`——那四个模块不出现在 LSP 接口视图里，但属于标准库 API
/// 面，需要文档。`test_stdlib_docs_covers_interface_modules` 保证本列表是接口
/// 视图模块集的超集，防止两处清单漂移。
pub fn modules_for_docs() -> Vec<Box<dyn StdModule>> {
    vec![
        Box::new(crate::std::convert::ConvertModule),
        Box::new(crate::std::dict::DictModule),
        Box::new(crate::std::io::IoModule),
        // D5 硬切换：std.list 已由纯 yx 实现接管（src/std/list.yx），
        // 不再有 native `ListModule`；其接口面由 .yx 源码自身提供。
        Box::new(crate::std::math::MathModule),
        Box::new(crate::std::string::StringModule),
        Box::new(crate::std::time::TimeModule),
        Box::new(crate::std::result::ResultModule),
        Box::new(crate::std::range::RangeModule),
        Box::new(crate::std::assert::AssertModule),
        #[cfg(not(target_arch = "wasm32"))]
        Box::new(crate::std::net::NetModule),
        #[cfg(not(target_arch = "wasm32"))]
        Box::new(crate::std::concurrent::ConcurrentModule),
        #[cfg(not(target_arch = "wasm32"))]
        Box::new(crate::std::os::OsModule),
        #[cfg(not(target_arch = "wasm32"))]
        Box::new(crate::std::weak::WeakModule),
    ]
}

/// 模块短名（`std.string` → `string`），同时是文档文件名
fn module_key(module: &dyn StdModule) -> String {
    module
        .module_path()
        .strip_prefix("std.")
        .unwrap_or(module.module_path())
        .to_string()
}

fn start_marker(key: &str) -> String {
    format!("<!-- stdlib:{key} start -->")
}

fn end_marker(key: &str) -> String {
    format!("<!-- stdlib:{key} end -->")
}

/// 渲染「函数一览」表：两列，全部由 `exports()` 派生，无手写内容
fn render_table(module: &dyn StdModule) -> String {
    let mut out = String::from("| 函数 | 签名 |\n| ---- | ---- |\n");
    for export in module.exports() {
        out.push_str(&format!("| `{}` | `{}` |\n", export.name, export.signature));
    }
    out.trim_end().to_string()
}

/// 渲染单函数签名块（含围栏）。常量（签名不以 `(` 开头）同样渲染为
/// `NAME: Type`，与 `gen_interfaces` 的判定一致。
fn render_signature(export: &crate::std::NativeExport) -> String {
    format!("```yaoxiang\n{}: {}\n```", export.name, export.signature)
}

/// 从模块页的 frontmatter 读取 `description`（手写叙述，但只读来渲染索引）
fn module_description(
    root: &Path,
    name: &str,
) -> String {
    let path = root.join(STDLIB_DOCS_REL).join(format!("{name}.md"));
    let Ok(doc) = std::fs::read_to_string(&path) else {
        return String::new();
    };
    // frontmatter 形如：---\ntitle: '...'\ndescription: '...'\n---
    let head = doc.split("---").nth(1).unwrap_or("");
    for line in head.lines() {
        if let Some(rest) = line.strip_prefix("description:") {
            return rest
                .trim()
                .trim_matches(|c| c == '\'' || c == '"')
                .to_string();
        }
    }
    String::new()
}

/// 渲染「模块索引」表（总览页用）
///
/// 模块名、链接与导出数由 `modules_for_docs()` 派生；说明列取自各模块页的
/// frontmatter，保证索引与页面标题一致。
fn render_module_index(root: &Path) -> String {
    let mut out = String::from("| 模块 | 导出数 | 说明 |\n| ---- | ------ | ---- |\n");
    for module in modules_for_docs() {
        let name = module_key(module.as_ref());
        out.push_str(&format!(
            "| [`std.{name}`](./{name}) | {} | {} |\n",
            module.exports().len(),
            module_description(root, &name)
        ));
    }
    out.trim_end().to_string()
}

/// 总览页的生成区（与各模块页分开，因为它是全模块聚合）
fn index_regions(root: &Path) -> Vec<(String, String)> {
    vec![("index:modules".to_string(), render_module_index(root))]
}

/// 一个模块的**必需**生成区：函数一览表。表由 `exports()` 全量派生，
/// 因此"每个导出都在文档里出现过"由它保证。
pub fn required_regions(module: &dyn StdModule) -> Vec<(String, String)> {
    let name = module_key(module);
    vec![(format!("table:{name}"), render_table(module))]
}

/// 该模块全部合法签名区：签名块是**可选**的（函数可被合并成一节叙述，如
/// `convert` 的十个 `*.to_string` 别名），但一旦出现就必须与 `exports()` 逐字
/// 节一致。
pub fn signature_regions(module: &dyn StdModule) -> Vec<(String, String)> {
    let name = module_key(module);
    module
        .exports()
        .iter()
        .map(|e| (format!("sig:{name}.{}", e.name), render_signature(e)))
        .collect()
}

/// 提取文档中出现的全部 `stdlib:` 区间 key
fn marker_keys(doc: &str) -> Vec<String> {
    let mut keys = Vec::new();
    let mut rest = doc;
    while let Some(i) = rest.find("<!-- stdlib:") {
        let after = &rest[i + "<!-- stdlib:".len()..];
        match after.find(' ') {
            Some(sp) => {
                keys.push(after[..sp].to_string());
                rest = &after[sp..];
            }
            None => break,
        }
    }
    keys
}

/// 比对单个文档文件的全部生成区；返回错误清单（空 = 一致）
fn check_file(
    doc: &str,
    module: &dyn StdModule,
) -> Vec<String> {
    let mut regions = required_regions(module);

    // 可选的签名区：只校验文档中**实际存在**的那些
    let present: Vec<String> = marker_keys(doc);
    for (key, body) in signature_regions(module) {
        if present.contains(&key) {
            regions.push((key, body));
        }
    }

    check_marked_regions(doc, &regions, &present)
        .into_iter()
        .collect()
}

/// 归一化正文以便比对：折叠空白 + 规范表格分隔行
///
/// 生成区的内容正确性是本门禁的职责；**排版不是**——`prettier` 在 CI 与
/// auto-translate 流程里会（a）按 CJK 显示宽度补齐表格列宽、（b）把分隔行
/// `| --- | --- |` 拉到与列宽一致。后者是**真实字符**而非空白，单靠折叠空白
/// 比不出来，因此这里把分隔行的短横统一为固定 token。
fn normalize(body: &str) -> String {
    body.lines()
        .map(|line| {
            let collapsed = line.split_whitespace().collect::<Vec<_>>().join(" ");
            // 表格分隔行：只由 |、-、:、空格组成且含 - → 统一成 | --- |
            if collapsed.contains('-')
                && collapsed
                    .chars()
                    .all(|c| matches!(c, '|' | '-' | ':' | ' '))
            {
                "| --- |".to_string()
            } else {
                collapsed
            }
        })
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// 通用的标记区比对：`regions` 为期望内容，`present` 用于孤儿检测
fn check_marked_regions(
    doc: &str,
    regions: &[(String, String)],
    present: &[String],
) -> Vec<String> {
    let mut errors = Vec::new();

    for (key, expected) in regions {
        let start = start_marker(key);
        let end = end_marker(key);
        let Some(s) = doc.find(&start) else {
            errors.push(format!("{key}: 缺少区间起始标记 {start}"));
            continue;
        };
        let Some(e_rel) = doc[s..].find(&end) else {
            errors.push(format!("{key}: 缺少区间结束标记 {end}"));
            continue;
        };
        let actual = &doc[s + start.len()..s + e_rel];
        if normalize(actual) != normalize(expected) {
            errors.push(format!(
                "{key}: 生成区与 exports() 漂移（运行 cargo run --example gen-stdlib-docs 治愈）"
            ));
        }
    }

    // 孤儿检测：文档里有签名区，但对应导出已不存在（函数被改名/删除）
    let valid: Vec<String> = regions.iter().map(|(k, _)| k.clone()).collect();
    for key in present {
        if key.starts_with("sig:") && !valid.contains(key) {
            errors.push(format!(
                "{key}: 孤儿签名区——该导出已不在 exports() 中（删除该节或修正函数名）"
            ));
        }
    }

    errors
}

/// 校验整个文档目录；返回错误清单
pub fn check_stdlib_docs(root: &Path) -> Vec<String> {
    let dir = root.join(STDLIB_DOCS_REL);
    let mut errors = Vec::new();

    for module in modules_for_docs() {
        let name = module_key(module.as_ref());
        let path = dir.join(format!("{name}.md"));
        let Ok(doc) = std::fs::read_to_string(&path) else {
            errors.push(format!("{name}.md: 文档缺失（路径 {}）", path.display()));
            continue;
        };
        for e in check_file(&doc, module.as_ref()) {
            errors.push(format!("{name}.md: {e}"));
        }
    }

    // 总览页的模块索引表
    let index_path = dir.join("index.md");
    match std::fs::read_to_string(&index_path) {
        Ok(doc) => {
            for e in check_marked_regions(&doc, &index_regions(root), &[]) {
                errors.push(format!("index.md: {e}"));
            }
        }
        Err(_) => errors.push(format!(
            "index.md: 总览页缺失（路径 {}）",
            index_path.display()
        )),
    }

    errors
}

/// 就地把各生成区重写为当前 `exports()` 的内容
///
/// 表区必需；签名区只重写文档中已存在的那些（缺标记不算错，因为节是可选
/// 的）。返回重写的文件数。
pub fn fix_stdlib_docs(root: &Path) -> Result<usize, String> {
    let dir = root.join(STDLIB_DOCS_REL);
    let mut rewritten = 0;

    for module in modules_for_docs() {
        let name = module_key(module.as_ref());
        let path = dir.join(format!("{name}.md"));
        let doc = std::fs::read_to_string(&path)
            .map_err(|e| format!("读取 {} 失败: {e}", path.display()))?;

        let mut out = doc.clone();
        let present: Vec<String> = marker_keys(&doc);
        let mut regions = required_regions(module.as_ref());
        for (key, body) in signature_regions(module.as_ref()) {
            if present.contains(&key) {
                regions.push((key, body));
            }
        }

        for (key, body) in regions {
            let start = start_marker(&key);
            let end = end_marker(&key);
            let s = out
                .find(&start)
                .ok_or_else(|| format!("{name}.md: {key}: 缺少区间起始标记"))?;
            let e_rel = out[s..]
                .find(&end)
                .ok_or_else(|| format!("{name}.md: {key}: 缺少区间结束标记"))?;
            // 只在“内容”真的不同时重写：排版（prettier 的 CJK 列宽补齐）应当
            // 保留，不能被本工具的紧凑表格覆盖，否则与 prettier 来回抖动。
            let actual = &out[s + start.len()..s + e_rel];
            if normalize(actual) != normalize(&body) {
                // 区间内首尾各留一个空行：markdownlint MD058 要求表格前后有空行。
                out.replace_range(
                    s..s + e_rel + end.len(),
                    &format!("{start}\n\n{body}\n\n{end}"),
                );
            }
        }

        if out != doc {
            std::fs::write(&path, &out)
                .map_err(|e| format!("写入 {} 失败: {e}", path.display()))?;
            rewritten += 1;
        }
    }

    // 总览页的模块索引表
    let index_path = dir.join("index.md");
    let doc = std::fs::read_to_string(&index_path)
        .map_err(|e| format!("读取 {} 失败: {e}", index_path.display()))?;
    let mut out = doc.clone();
    for (key, body) in index_regions(root) {
        let start = start_marker(&key);
        let end = end_marker(&key);
        let s = out
            .find(&start)
            .ok_or_else(|| format!("index.md: {key}: 缺少区间起始标记"))?;
        let e_rel = out[s..]
            .find(&end)
            .ok_or_else(|| format!("index.md: {key}: 缺少区间结束标记"))?;
        let actual = &out[s + start.len()..s + e_rel];
        if normalize(actual) != normalize(&body) {
            out.replace_range(
                s..s + e_rel + end.len(),
                &format!("{start}\n\n{body}\n\n{end}"),
            );
        }
    }
    if out != doc {
        std::fs::write(&index_path, &out)
            .map_err(|e| format!("写入 {} 失败: {e}", index_path.display()))?;
        rewritten += 1;
    }

    Ok(rewritten)
}
