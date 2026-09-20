//! 标准库 API 文档门禁
//!
//! `docs/src/reference/stdlib/*.md` 是**生成 + 手写**混合文档：
//!
//! - 生成区（`<!-- stdlib:KEY start/end -->` 标记之间）：函数一览表与签名块，
//!   由 `StdModule::exports()` 派生。签名逐字节来自 `NativeExport::signature`，
//!   因此不可能与实现漂移。
//! - 手写区（标记之外）：模块概述、借用/移动语义、错误模型、已知坑与示例。
//!
//! 本文件提供两道门禁：
//!
//! 1. **漂移门禁**——生成区必须与当前 `exports()` 逐字节一致（`test_stdlib_docs_match_generation`）。
//!    治愈：`cargo run --example gen-stdlib-docs`（与 `gen-std-interfaces`、
//!    `tools/code-tables --fix` 同构）。
//! 2. **示例可运行门禁**——文档中每个 ```yaoxiang 代码块必须真的能跑
//!    （`test_stdlib_docs_examples_run`）。手写叙述可以措辞不准，但示例不能假；
//!    这一道是手写部分唯一的客观防线。
//!
//! 另有**覆盖面门禁**（`test_stdlib_docs_covers_interface_modules`）：文档化的
//! 模块集必须覆盖 LSP 接口视图的全部模块，防止两处清单各自漂移。

use std::path::{Path, PathBuf};

use crate::std::gen_docs::{check_stdlib_docs, modules_for_docs, STDLIB_DOCS_REL};
use crate::std::gen_interfaces::generate_all_interfaces;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn docs_dir() -> PathBuf {
    repo_root().join(STDLIB_DOCS_REL)
}

/// 门禁一：生成区与 `StdModule::exports()` 一致
#[test]
fn test_stdlib_docs_match_generation() {
    let errors = check_stdlib_docs(&repo_root());
    assert!(
        errors.is_empty(),
        "标准库文档生成区与 exports() 漂移；治愈：cargo run --example gen-stdlib-docs\n{}",
        errors.join("\n")
    );
}

/// 本仓库**应当**有文档页的模块名（不含 `.md`）。
///
/// 两部分：native `StdModule`（生成器产出）+ 纯 yx 模块（手写页，RFC-036 §4）。
/// 两个门禁共用同一份清单——否则 yx 模块的页会从其中一个门禁的视野里消失。
fn documented_module_names() -> Vec<String> {
    let yx_backed: Vec<String> = crate::std::yx_sources::STD_YX_FILES
        .iter()
        .filter_map(|(file, _)| {
            file.strip_prefix("std/")
                .and_then(|f| f.strip_suffix(".yx"))
                .map(|n| format!("{}.md", n.replace('/', "-")))
        })
        .collect();
    modules_for_docs()
        .iter()
        .map(|m| format!("{}.md", m.module_path().trim_start_matches("std.")))
        .chain(yx_backed)
        .collect()
}

/// 门禁一（反向）：文档目录不得比生成器多出模块页
///
/// 生成器删掉某模块后，其 md 会变成无人维护的孤儿——同
/// `test_committed_interface_dir_has_no_orphan_files` 的反向检查。
#[test]
fn test_stdlib_docs_has_no_orphan_module_pages() {
    // 纯 yx 模块（RFC-036 §4，`src/std/yx_sources.rs` 的 STD_YX_FILES）不是 native
    // `StdModule`，不进 `modules_for_docs()`，但它们**确实**有文档页。
    // 这些页是手写的模块说明（不再由生成器产出），故显式列为合法页。
    let valid: Vec<String> = documented_module_names();

    let entries = std::fs::read_dir(docs_dir())
        .unwrap_or_else(|e| panic!("读取 {} 失败: {e}", docs_dir().display()));

    for entry in entries {
        let name = entry
            .unwrap_or_else(|e| panic!("读取目录条目失败: {e}"))
            .file_name()
            .to_string_lossy()
            .to_string();
        if name == "index.md" || !name.ends_with(".md") {
            continue;
        }
        assert!(
            valid.contains(&name),
            "{name} 是孤儿模块页（生成器已不再产出）；删除该文件，或把它加进 \
             gen_docs::modules_for_docs()"
        );
    }
}

/// 门禁三：文档化的模块集覆盖接口视图的全部模块
#[test]
fn test_stdlib_docs_covers_interface_modules() {
    let documented: Vec<String> = modules_for_docs()
        .iter()
        .map(|m| m.module_path().trim_start_matches("std.").to_string())
        .collect();

    for (name, _) in generate_all_interfaces() {
        assert!(
            documented.contains(&name),
            "接口视图有模块 `{name}`，但 gen_docs::modules_for_docs() 未覆盖它——\
             两处模块清单已漂移"
        );
    }
}

/// 门禁二：文档中的每个可运行示例都必须真的能跑
///
/// 判定「可运行」的形式是块内出现 `main: () -> Void = {`（无注解块是值、非函数，
/// 与本门禁无关）。检查前会补一行 `main()`——块里只定义不调用，`run` 不会自动调它，
/// 断言即使全错也退出码 0（这曾让本门禁空转，2026-09 修）。
/// 诊断走 stderr；退出码非 0 或 stderr 非空都算失败。
#[test]
fn test_stdlib_docs_examples_run() {
    let mut checked = 0usize;
    let mut failures: Vec<String> = Vec::new();

    // 遍历**所有**有文档页的模块，含纯 yx 模块（`list` 等）——它们不进
    // `modules_for_docs()`，但页里同样有可运行示例，漏掉等于无人守。
    for page in documented_module_names() {
        let name = page.trim_end_matches(".md");
        let path = docs_dir().join(&page);
        let Ok(doc) = std::fs::read_to_string(&path) else {
            continue;
        };

        for (i, code) in yaoxiang_examples(&doc).into_iter().enumerate() {
            // 可运行示例 = 带 Fn 注解的入口（B 方案：无注解块是值，不是函数）
            if !code.contains("main: () -> Void = {") {
                continue;
            }
            // 生成区里的签名块不是可运行程序
            if !code.contains('{') || code.trim().ends_with("= {") {
                continue;
            }
            checked += 1;

            // 块里只定义 `main`，没有调用——`run` 不会自动调它，于是断言
            // 即使全错也退出码 0。必须补一行 `main()` 才真的执行。
            let mut src = code.trim_end().to_string();
            if !src.ends_with("main()") {
                src.push_str(
                    "
main()
",
                );
            }

            let file = std::env::temp_dir().join(format!("yx_doc_{name}_{i}.yx"));
            if let Err(e) = std::fs::write(&file, &src) {
                failures.push(format!("{name}.md#{i}: 写临时文件失败: {e}"));
                continue;
            }

            let out = std::process::Command::new(yaoxiang_binary())
                .arg("run")
                .arg(&file)
                // 隔离 cwd：示例里的相对路径读写全部落在临时目录；
                // 进程级 cwd 被其他测试污染（如残留进已删除的 TempDir）也免疫。
                .current_dir(std::env::temp_dir())
                .output();

            match out {
                Ok(o) => {
                    let stderr = String::from_utf8_lossy(&o.stderr);
                    if !o.status.success() || !stderr.trim().is_empty() {
                        failures.push(format!(
                            "{name}.md#{i}: 示例执行失败\n--- 源码 ---\n{code}\n--- stderr ---\n{stderr}"
                        ));
                    }
                }
                Err(e) => failures.push(format!("{name}.md#{i}: 无法启动解释器: {e}")),
            }
        }
    }

    assert!(checked > 0, "未找到任何可运行示例，语料选择器可能失效");
    assert!(
        failures.is_empty(),
        "{} 个文档示例无法运行（共检查 {checked} 个）：\n{}",
        failures.len(),
        failures.join("\n\n")
    );
}

/// 提取 markdown 中全部 ```yaoxiang 围栏代码块
fn yaoxiang_examples(doc: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = doc;
    while let Some(i) = rest.find("```yaoxiang\n") {
        let after = &rest[i + "```yaoxiang\n".len()..];
        let Some(end) = after.find("```") else { break };
        out.push(after[..end].to_string());
        rest = &after[end + 3..];
    }
    out
}

/// 被测解释器路径
///
/// 必须用 `CARGO_BIN_EXE_*`：引擎二进制从 `yaoxiang` 改名为 `yaoxiang-rs` 后，
/// 按名字猜路径会拿到 target/ 里残留的旧构建产物（旧 exe 永不重建），
/// 于是本门禁一直在拿几个月前的编译器验收文档示例。
///
/// lib 单测里 cargo 不提供 `env!` 形式的该变量，只能运行时取；
/// 取不到（如直接跑 lib test 目标）则回退到 target/<profile>/ 下的产物路径。
fn yaoxiang_binary() -> PathBuf {
    if let Ok(path) = std::env::var("CARGO_BIN_EXE_yaoxiang-rs") {
        return PathBuf::from(path);
    }
    // 回退：测试可执行文件位于 target/<profile>/deps/，主二进制在上一层
    let mut exe = std::env::current_exe().expect("无法定位当前测试可执行文件");
    exe.pop();
    if exe.ends_with("deps") {
        exe.pop();
    }
    exe.join(if cfg!(windows) {
        "yaoxiang-rs.exe"
    } else {
        "yaoxiang-rs"
    })
}

/// 临时文件清理的辅助断言：确认被测路径确实存在（提前暴露构建布局变化）
#[test]
fn test_yaoxiang_binary_is_discoverable() {
    let bin = yaoxiang_binary();
    assert!(
        bin.exists(),
        "未找到解释器 {}；文档示例门禁依赖 target/<profile>/yaoxiang-rs",
        bin.display()
    );
    // 防回归：引擎改名前旧名产物会永远留在 target/ 里不重建。
    // 路径必须指向当前 bin 名，否则门禁会拿陈旧编译器验收文档示例。
    let name = bin.file_name().unwrap_or_default().to_string_lossy();
    assert!(
        name.starts_with("yaoxiang-rs"),
        "被测二进制名为 {name}，疑似指向改名前的陈旧产物；"
    );
}

/// 门禁辅助：把 `Path` 形式的文档路径暴露给潜在的调试调用
#[allow(dead_code)]
fn docs_path_for(module: &str) -> PathBuf {
    Path::new(&docs_dir()).join(format!("{module}.md"))
}
