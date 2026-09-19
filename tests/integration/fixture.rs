//! 集成测试共享夹具辅助（T4 入口语义适配）
//!
//! # 背景
//!
//! T4 定下 Script 模式（无 `yaoxiang.toml`）的入口语义：**顶层语句就是程序主体**，
//! `main` 不再被隐式调用——想跑就写 `main()`。
//!
//! 集成测试里大量夹具形如：
//!
//! ```ignore
//! run_ok("main = () => { x = 42; print(x) }");
//! ```
//!
//! 按新语义这类夹具会**空跑**（编译通过、`main` 不执行、断言无意义）。
//! 本模块提供 [`with_main_invoked`]，在夹具定义了 `main` 但未调用时自动补上
//! `main()`，让既有夹具继续真正执行。夹具测的是语言特性，不是入口语义，
//! 故这层适配不会掩盖被测行为。

/// 若源码定义了顶层 `main` 而没调用它，追加一行 `main()`。
///
/// 判定为纯文本（不重新解析），避免把夹具自身的语法错误误判成「无 main」。
/// 未定义 `main` 的源码原样返回——那类夹具（如顶层语句形态）本来就是程序主体。
///
/// 三类不追加：
/// - **`main` 是值绑定**（如 `main: Int = 5`）：那是 E3021 的夹具，加调用会
///   掩盖被测诊断。只在无注解或注解为 Fn 时追加。
/// - **空块 `main = () => {}`**：parser 把它解析为**空 Dict 字面量**（`{}` 二义，
///   与语法规范 §2.9 冲突，见 **#359**），不是函数体——追加 `main()`
///   会变成“调用一个 Dict”，报 E6006。
/// - **项目模式（有 yaoxiang.toml）**：那种文件是 Bin 角色，`main` 本就是
///   隐式入口，且顶层可执行语句被拒——追加反而报 E3005。
///   用 [`with_main_invoked_in`] 传目录让本函数自行判断；
///   无法判断目录时（如内存源码）调用方应确保无 manifest。
pub fn with_main_invoked(source: &str) -> String {
    with_main_invoked_opts(source, false)
}

/// 同 [`with_main_invoked`]，但能感知项目目录：
/// 目录下存在 `yaoxiang.toml` 时不追加（Bin 角色，`main` 是隐式入口）。
pub fn with_main_invoked_in(
    dir: &std::path::Path,
    source: &str,
) -> String {
    let has_manifest = dir.join("yaoxiang.toml").exists();
    with_main_invoked_opts(source, has_manifest)
}

fn with_main_invoked_opts(
    source: &str,
    has_manifest: bool,
) -> String {
    if has_manifest {
        return source.to_string();
    }
    let mut main_is_callable = false;
    let mut calls_main = false;
    for line in source.lines() {
        let t = line.trim();
        if t == "main()" {
            calls_main = true;
            continue;
        }
        // 无注解：`main = ...`
        if let Some(rhs) = t.strip_prefix("main =") {
            let rhs = rhs.trim_start();
            // 排除空块（Dict 字面量）与显式非函数值；`() =>` 是显式 lambda
            let callable =
                (rhs.starts_with('{') && !rhs.starts_with("{}") && !rhs.starts_with("{ }"))
                    || rhs.starts_with("() =>")
                    || rhs.starts_with('(');
            main_is_callable |= callable;
        }
        // Fn 注解：`main: () -> T = ...`
        if let Some(rest) = t.strip_prefix("main:") {
            if rest.contains("->") {
                main_is_callable = true;
            }
        }
    }
    if main_is_callable && !calls_main {
        format!("{source}\nmain()\n")
    } else {
        source.to_string()
    }
}
