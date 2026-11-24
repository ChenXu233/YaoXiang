//! CLI `yaoxiang check` 命令集成测试
//!
//! 测试 `check` 子命令对 .yx 文件的类型检查功能。
//! 覆盖正常文件、错误文件和无文件输入三种场景。
//!
//! 规范来源：
//! - RFC-029f: 编译目标角色语义（声明面入口存在性检查，#388 定案）
//! - RFC-029 / RFC-015: 模块发现与导入解析（E5001/E5003）
//! - RFC-014: 包管理系统（vendor 依赖布局）

#![cfg(feature = "cli")]

use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;
use yaoxiang::util::diagnostic::run_check_command_once;

/// 在临时目录中创建 .yx 源文件
fn create_yx_file(
    dir: &TempDir,
    name: &str,
    content: &str,
) -> PathBuf {
    let path = dir.path().join(name);
    std::fs::write(&path, content).expect("Failed to write test file");
    path
}

/// 创建一个临时目录
fn temp_dir() -> TempDir {
    TempDir::new().expect("Failed to create temp dir")
}

#[allow(clippy::ptr_arg)]
/// 辅助：对单个文件运行 check，返回 Ok(error_count) 或 Err
fn check_file(path: &PathBuf) -> Result<usize, anyhow::Error> {
    // #321 M2：run_check_command_once 返回 (error_count, warning_count)，
    // 本辅助仅关心错误数
    run_check_command_once(
        std::slice::from_ref(path),
        &[],
        &[],   // allow — issue #499（不带 --allow 的既有口径）
        false, // json
        false, // use_colors
        true,  // no_progress — 抑制进度输出
        false, // deny_shadowing
    )
    .map(|(errors, _warnings)| errors)
}

// 正常路径：无错误的 .yx 文件

#[test]
fn test_check_valid_file() {
    let dir = temp_dir();
    let file = create_yx_file(&dir, "valid.yx", r#"main = () => { x = 42; print(x) }"#);
    let result = check_file(&file);
    assert!(result.is_ok(), "Valid file should pass check");
    assert_eq!(result.unwrap(), 0, "Valid file should report 0 errors");
}

#[test]
fn test_check_module_import() {
    let dir = temp_dir();
    let file = create_yx_file(
        &dir,
        "import.yx",
        r#"use std.io; main = () => { io.println("ok") }"#,
    );
    let result = check_file(&file);
    assert!(result.is_ok(), "File with module import should pass check");
    assert_eq!(result.unwrap(), 0);
}

#[test]
fn test_check_complex_program() {
    let dir = temp_dir();
    let file = create_yx_file(
        &dir,
        "complex.yx",
        r#"
use std.list

main = () => {
    mut xs = [1, 2, 3]
    ys = list.map(xs, (x) => x * 2)
    print(ys)
}
"#,
    );
    let result = check_file(&file);
    assert!(result.is_ok(), "Complex program should pass check");
    assert_eq!(result.unwrap(), 0);
}

// 错误路径：有类型错误的 .yx 文件
#[test]
fn test_check_syntax_error() {
    // Arrange（4.10.1 + 4.2.5 显式翻转）：语法错误不再整体硬中止——rust
    // 式收集语义下 parse 诊断（E0012）计入错误数；旧断言「parse 阶段返回
    // Err」钉的是已修复的怪癖
    let dir = temp_dir();
    let file = create_yx_file(&dir, "syntax_error.yx", r#"main = () => { x =  }"#);

    // Act
    let result = check_result(&file);

    // Assert
    let codes = error_codes(&result);
    assert!(
        codes.contains(&"E0012".to_string()),
        "语法错误必须以 E0012 进收集通道: {codes:?}"
    );
    assert!(
        result.error_count >= 1,
        "语法错误计入错误数（非零退出的依据）: {}",
        result.error_count
    );
}

#[test]
fn test_check_type_mismatch() {
    let dir = temp_dir();
    let file = create_yx_file(
        &dir,
        "type_error.yx",
        r#"main = () => { x: Int = "hello" }"#,
    );
    let result = check_file(&file);
    assert!(result.is_ok(), "check should not panic on type errors");
    let error_count = result.unwrap();
    assert!(
        error_count > 0,
        "Type mismatch file should report errors > 0"
    );
}

#[test]
fn test_check_undeclared_variable() {
    let dir = temp_dir();
    let file = create_yx_file(
        &dir,
        "undeclared.yx",
        r#"main = () => { x = undefined_var }"#,
    );
    let result = check_file(&file);
    assert!(result.is_ok(), "check should not panic on undeclared var");
    let error_count = result.unwrap();
    assert!(
        error_count > 0,
        "Undeclared variable should report errors > 0"
    );
}

// 边界情况：空输入 / 路径不存在

#[test]
fn test_check_nonexistent_file() {
    let result = check_file(&PathBuf::from("nonexistent_file.yx"));
    assert!(result.is_err(), "Non-existent file should return error");
}

#[test]
fn test_check_file_no_yx_extension() {
    let dir = temp_dir();
    let file = create_yx_file(&dir, "hello.txt", "main = () => { print(42) }");
    let result = check_file(&file);
    // 非 .yx 文件路径会被 collect_yx_files_from_paths 过滤掉，返回 "No .yx files found"
    assert!(
        result.is_err(),
        "Non-yx file path should produce error (no .yx files found)"
    );
}

// issue #242：std 导出签名解析为真实 MonoType — 精确类型检查 std 调用

#[test]
fn test_check_std_call_wrong_arg_type_rejected() {
    // Arrange - write_file 签名为 (path: String, content: String) -> Bool
    let dir = temp_dir();
    let file = create_yx_file(
        &dir,
        "std_wrong_args.yx",
        "use std.io.{write_file}\nmain = () => {\n    write_file(123, 456)\n}\n",
    );

    // Act
    let result = check_file(&file);

    // Assert - Int 实参对 String 形参应报类型错误
    assert!(result.is_ok(), "check should not panic on type errors");
    assert!(
        result.unwrap() > 0,
        "write_file(123, 456) should report type errors > 0"
    );
}

#[test]
fn test_check_std_whole_module_qualified_call_rejected() {
    // Arrange - 整体导入后限定调用同样走精确签名
    let dir = temp_dir();
    let file = create_yx_file(
        &dir,
        "std_qualified_wrong.yx",
        "use std.io\nmain = () => {\n    io.write_file(123, 456)\n}\n",
    );

    // Act
    let result = check_file(&file);

    // Assert
    assert!(result.is_ok(), "check should not panic on type errors");
    assert!(
        result.unwrap() > 0,
        "io.write_file(123, 456) should report type errors > 0"
    );
}

#[test]
fn test_check_std_call_correct_args_accepted() {
    // Arrange - 参数类型正确的 write_file 调用（#104 后 write_file 在 std.fs）
    let dir = temp_dir();
    let file = create_yx_file(
        &dir,
        "std_correct.yx",
        "use std.fs.{write_file}\nmain = () => {\n    write_file(\"a.txt\", \"hello\")\n}\n",
    );

    // Act
    let result = check_file(&file);

    // Assert - 精确检查不应误报正确调用
    assert!(result.is_ok(), "Valid std call should pass check");
    assert_eq!(
        result.unwrap(),
        0,
        "write_file(String, String) should report 0 errors"
    );
}

#[test]
fn test_check_std_generic_higher_order_accepted() {
    // Arrange - [T] 泛型前缀签名（list.map）+ 闭包实参
    let dir = temp_dir();
    let file = create_yx_file(
        &dir,
        "std_map.yx",
        "use std.list\nmain = () => {\n    mut xs = [1, 2, 3]\n    ys = list.map(xs, (x) => x * 2)\n    print(ys)\n}\n",
    );

    // Act
    let result = check_file(&file);

    // Assert - 泛型绑定变量与闭包 Fn unify 不应误报
    assert!(result.is_ok(), "list.map with closure should pass check");
    assert_eq!(
        result.unwrap(),
        0,
        "list.map(xs, closure) should report 0 errors"
    );
}

#[test]
fn test_check_std_optional_param_both_arities_accepted() {
    // Arrange - (cond: Bool, ?msg: String) 可选参数的两种调用
    let dir = temp_dir();
    let file = create_yx_file(
        &dir,
        "std_assert.yx",
        "use std.assert.{assert}\nmain = () => {\n    assert(1 > 0)\n    assert(1 > 0, \"must hold\")\n}\n",
    );

    // Act
    let result = check_file(&file);

    // Assert - 1 参（宽容路径）与 2 参（精确路径）都应通过
    assert!(result.is_ok(), "assert with 1 or 2 args should pass check");
    assert_eq!(
        result.unwrap(),
        0,
        "assert optional param calls should report 0 errors"
    );
}

#[test]
fn test_check_std_result_return_accepted() {
    // Arrange - parse_int 返回 Result(Int, Error)
    let dir = temp_dir();
    let file = create_yx_file(
        &dir,
        "std_result.yx",
        "use std.string.{parse_int}\nuse std.io\nmain = () => {\n    r = parse_int(\"42\")\n    io.println(r)\n}\n",
    );

    // Act
    let result = check_file(&file);

    // Assert - 结构化 Result 返回类型的正确使用应通过
    assert!(result.is_ok(), "parse_int correct usage should pass check");
    assert_eq!(
        result.unwrap(),
        0,
        "parse_int(\"42\") should report 0 errors"
    );
}

// 多文件项目：check 与 run 必须走同一条编译路径（F1 回归守卫）

/// 在临时目录创建含 `yaoxiang.toml` 的项目，返回项目根目录。
fn create_project(files: &[(&str, &str)]) -> TempDir {
    let dir = TempDir::new().expect("Failed to create temp dir");
    std::fs::write(
        dir.path().join("yaoxiang.toml"),
        "[package]\nname = \"proj\"\nversion = \"0.1.0\"\n",
    )
    .expect("Failed to write manifest");
    for (name, content) in files {
        let path = dir.path().join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("Failed to create parent dirs");
        }
        std::fs::write(&path, content).expect("Failed to write source file");
    }
    dir
}

/// 检查单个文件并返回聚合结果（用于断言错误码）。
fn check_result(path: &PathBuf) -> yaoxiang::util::diagnostic::CheckResult {
    // 默认编译闸门：本文件的断言只看诊断码，不涉 [mono] 配置面
    yaoxiang::util::diagnostic::check_files_with_diagnostics(
        std::slice::from_ref(path),
        yaoxiang::frontend::config::CompileConfig::default(),
    )
    .expect("check should not fail at parse stage")
}

fn error_codes(result: &yaoxiang::util::diagnostic::CheckResult) -> Vec<String> {
    result
        .diagnostics
        .iter()
        .filter(|d| {
            matches!(
                d.diagnostic.severity,
                yaoxiang::util::diagnostic::Severity::Error
            )
        })
        .map(|d| d.diagnostic.code.clone())
        .collect()
}

#[test]
fn test_check_multifile_project_matches_run() {
    // Arrange - 跨文件导入的合法项目：check 与 run 结论必须一致
    let dir = create_project(&[
        ("lib.yx", "add_one: (x: Int) -> Int = (x) => x + 1\n"),
        (
            "main.yx",
            "use std.assert\nuse lib.{add_one}\n\nmain = () => {\n    assert.assert(add_one(41) == 42, \"ok\")\n}\n",
        ),
    ]);
    let main = dir.path().join("main.yx");

    // Act
    let checked = check_file(&main);

    // Assert - 此前 check 逐文件独立编译，报假错 E1001 unknown variable
    assert!(
        checked.is_ok(),
        "check on a valid multifile project should not fail: {checked:?}"
    );
    assert_eq!(
        checked.unwrap(),
        0,
        "check must agree with run: valid project has 0 errors"
    );
    yaoxiang::run_project(&main).expect("run should succeed on the same project");
}

/// 值绑定 main 是合法入口（#388 定案）：check 必须与 run 同判 0 错误。
///
/// 此前 check_project 不做入口检查，入口语义只活在 run 路径——check 对
/// 入口问题永远沉默，与「check 与 run 走同一条编译路径」的目标相悖。
#[test]
fn test_check_bin_value_main_agrees_with_run() {
    // Arrange - Bin 项目，入口为无注解块值 main（初始化期求值即执行）
    let dir = create_project(&[("main.yx", "main = { }\n")]);
    let main = dir.path().join("main.yx");

    // Act
    let checked = check_file(&main);

    // Assert - check 0 错误，run 同项目亦成功
    assert!(
        checked.is_ok(),
        "check on value-main project should not fail: {checked:?}"
    );
    assert_eq!(
        checked.unwrap(),
        0,
        "value main is a legal entry: check must report 0 errors"
    );
    yaoxiang::run_project(&main).expect("run should succeed on the same project");
}

/// Bin 项目缺 main：check 必须与 run 同报 E3020。
///
/// 此前 run 报 E3020 而 check 报 0 错——入口检查只活在 compile_project。
/// 判据是 manifest 声明面（`[[bin]].path` / `[run].main`，#388），故夹具
/// 显式声明 bin。
#[test]
fn test_check_missing_main_reports_e3020() {
    // Arrange - 声明面入口文件只有函数定义、无 main
    let dir = create_project(&[
        (
            "yaoxiang.toml",
            "[package]\nname = \"proj\"\nversion = \"0.1.0\"\n\n[[bin]]\nname = \"app\"\npath = \"main.yx\"\n",
        ),
        ("main.yx", "helper: () -> Void = { }\n"),
    ]);
    let main = dir.path().join("main.yx");

    // Act
    let result = check_result(&main);

    // Assert
    let codes = error_codes(&result);
    assert!(
        codes.contains(&"E3020".to_string()),
        "check must agree with run: missing main should report E3020, got {codes:?}"
    );
}

/// 项目内某文件语法错误：check 不再整体中止——带病文件的 parse 诊断与
/// 导入方的模块未找到一次性收齐（rust 式收集语义，WBS 4.10.1，
/// 2026-10-09 用户裁决），错误计数非零。
#[test]
fn test_check_multifile_parse_error_collects_all_errors() {
    // Arrange - lib.yx 语法错误（E0012 族 parse 失败），main 导入它
    let dir = create_project(&[
        ("lib.yx", "let = ;\n"),
        ("main.yx", "use lib.{x}\nmain: () -> Void = {}\n"),
    ]);
    let main = dir.path().join("main.yx");

    // Act
    let result = check_result(&main);

    // Assert - lib 的 parse 错误（E0012，词法/语法族 E0）与 main 的
    // E5001 同批出现
    let codes = error_codes(&result);
    assert!(
        codes.contains(&"E0012".to_string()),
        "带病文件的 parse 诊断必须进收集结果: {codes:?}"
    );
    assert!(
        codes.contains(&"E5001".to_string()),
        "导入带病模块应报模块未找到: {codes:?}"
    );
    assert!(
        result.error_count >= 2,
        "两类错误都计入错误数（非零退出的依据）: {}",
        result.error_count
    );
}

/// standalone（无 manifest）单文件的相对导入解析（WBS 4.2.5 裁决 A）：
/// 与 rustc 单文件 mod 解析对齐——邻旁文件沿导入者目录被发现，
/// 不再报 E5001。
#[test]
fn test_check_standalone_relative_use_resolves() {
    // Arrange - 无 manifest 的目录：a.yx 相对导入 b.yx
    let dir = temp_dir();
    create_yx_file(&dir, "b.yx", "f: (x: Int) -> Int = (x) => x + 1\n");
    let main = create_yx_file(&dir, "a.yx", "use b.{f}\nmain = () => {\n    y = f(1)\n}\n");

    // Act
    let result = check_file(&main);

    // Assert - 旧路径（check_single_file 单文件管线）报 E5001
    assert!(result.is_ok(), "standalone 相对导入应正常解析: {result:?}");
    assert_eq!(result.unwrap(), 0, "相对导入解析成功则零错误（裁决 A）");
}

/// standalone 多错误文件的收集语义（WBS 4.2.5 归一）：统一 Check 形态后
/// CollectAll 全报——旧 check_single_file 走 pipeline FailFast 只报首错。
#[test]
fn test_check_standalone_collects_all_errors() {
    // Arrange - 两个独立的类型错误（y 与 z）
    let dir = temp_dir();
    let file = create_yx_file(
        &dir,
        "multi_err.yx",
        "x: Int = 1\ny: Int = \"s\"\nz: Bool = 42\nmain: () -> Void = {}\n",
    );

    // Act
    let result = check_result(&file);

    // Assert - y 与 z 的 E1002 都在
    let codes = error_codes(&result);
    let e1002_count = codes.iter().filter(|c| *c == "E1002").count();
    assert!(
        e1002_count >= 2,
        "CollectAll 应收齐全部类型错误（旧行为只报首错）: {codes:?}"
    );
}

#[test]
fn test_check_missing_module_reports_e5001() {
    // Arrange - 导入不存在的模块
    let dir = create_project(&[("main.yx", "use nosuch.{thing}\n\nmain = () => {\n}\n")]);
    let main = dir.path().join("main.yx");

    // Act
    let result = check_result(&main);

    // Assert - 此前静默通过（E1001 都不一定报），现必须报 E5001
    let codes = error_codes(&result);
    assert!(
        codes.contains(&"E5001".to_string()),
        "missing module should report E5001, got: {codes:?}"
    );
}

#[test]
fn test_check_missing_export_reports_e5003() {
    // Arrange - 模块存在但导出项不存在
    let dir = create_project(&[("main.yx", "use std.math.{nosuch_fn}\n\nmain = () => {\n}\n")]);
    let main = dir.path().join("main.yx");

    // Act
    let result = check_result(&main);

    // Assert
    let codes = error_codes(&result);
    assert!(
        codes.contains(&"E5003".to_string()),
        "missing export should report E5003, got: {codes:?}"
    );
}

#[test]
fn test_check_vendor_dependency_importable() {
    // Arrange - 已安装的 vendor 依赖（RFC-014 布局：<pkg>-<ver>/src/<pkg>.yx）
    let dir = create_project(&[
        (
            ".yaoxiang/vendor/foo-0.1.0/src/foo.yx",
            "add_one: (x: Int) -> Int = (x) => x + 1\n",
        ),
        (
            "main.yx",
            "use std.assert\nuse foo.{add_one}\n\nmain = () => {\n    assert.assert(add_one(41) == 42, \"vendor\")\n}\n",
        ),
    ]);
    let main = dir.path().join("main.yx");

    // Act
    let checked = check_file(&main);

    // Assert - 此前 vendor 不在导入路径上，报 E1001 unknown variable
    assert_eq!(
        checked.expect("check should not fail"),
        0,
        "installed vendor dependency should be importable"
    );
    yaoxiang::run_project(&main).expect("run should succeed with vendor dependency");
}

#[test]
fn test_check_vendor_missing_dependency_reports_e5001() {
    // Arrange - 依赖未安装（vendor 目录为空）
    let dir = create_project(&[("main.yx", "use foo.{add_one}\n\nmain = () => {\n}\n")]);
    let main = dir.path().join("main.yx");

    // Act
    let result = check_result(&main);

    // Assert
    let codes = error_codes(&result);
    assert!(
        codes.contains(&"E5001".to_string()),
        "uninstalled dependency should report E5001, got: {codes:?}"
    );
}

#[test]
fn test_check_two_entries_sharing_module_reports_it_once() {
    // Arrange - 同项目两个互不可达入口共享带错误的 lib.yx（`check` 无参数时
    // 收集全项目文件，正是此多入口场景）
    let dir = create_project(&[
        ("lib.yx", "broken: Int = nosuch_value\n"),
        ("a.yx", "use lib\n\nmain = () => {\n}\n"),
        ("b.yx", "use lib\n\nmain = () => {\n}\n"),
    ]);
    let entries = vec![dir.path().join("a.yx"), dir.path().join("b.yx")];

    // Act
    let result = yaoxiang::util::diagnostic::check_files_with_diagnostics(
        &entries,
        yaoxiang::frontend::config::CompileConfig::default(),
    )
    .expect("check should not fail at parse stage");
    let lib_codes: Vec<String> = result
        .diagnostics
        .iter()
        .filter(|d| d.file.contains("lib.yx"))
        .map(|d| d.diagnostic.code.clone())
        .collect();

    // Assert - lib.yx 的 E1001 只上报一次，计数不随入口数翻倍；
    // RFC-029f 起 check 路径接入死代码警告——`broken` 是未引用的私有变量，
    // 合法产生 W1004，且去重语义对警告同样生效（每码恰好一次）
    let e1001_count = lib_codes.iter().filter(|c| *c == "E1001").count();
    let w1004_count = lib_codes.iter().filter(|c| *c == "W1004").count();
    assert_eq!(
        e1001_count, 1,
        "shared module errors must be reported exactly once, got: {lib_codes:?}"
    );
    assert_eq!(
        w1004_count, 1,
        "unused private var warning must also be deduplicated, got: {lib_codes:?}"
    );
    assert_eq!(
        result.error_count, 1,
        "error_count must not double-count the shared module, got {}",
        result.error_count
    );
}

// 约束方法派发的编译期诊断锚（#519 甲案续篇 / 2026-11-01 用户裁决 / WBS 3.20.7）

/// #519 翻面锚（2026-11-01 甲案续篇落地，先红后绿）：mono 特化改写未命中
/// 约束方法派发名时，check 编译期报 E1106。前史：2026-10-31 甲案删除调用点
/// 复检（RFC-011b 附录B 设计决策记录）后失败推迟到运行期 E6006，本锚当时
/// 断言 check 静默（`test_check_unsatisfied_constraint_generic_stays_silent`）；
/// 2026-11-01 用户裁决把未命中前移到编译期（WBS 3.20.7），本锚按原守卫注释
/// 的约定翻面。
///
/// 判据（RFC-011b 约束泛型运算符派发 + issue #519）：NoAdd 未实现 Add，
/// `sum(NoAdd, NoAdd)` 的 `a + b` 改写目标 `NoAdd.add` 在函数表中无任何
/// 同名条目（成因 A）——mono 不再把裸名放进 IR 等运行期 E6006，当场报
/// E1106 且 detail 指明「函数表中不存在任何实现」。
///
/// 聚合形态说明：check 的聚合模式是 CollectAll（RFC-039 02-stage-contract，
/// `driver/program.rs` CHECK_STAGES 注记），mono 失败走逐文件收集通道
/// （`driver/arms.rs` `monomorphization_multi_file` 的 Check 臂），故
/// check_file 的翻面形态是 error_count 0→1 而非 Err。
#[test]
fn test_check_unsatisfied_constraint_generic_reports_e1106() {
    // Arrange - 未声明 Add 的记录类型 NoAdd，sum 以 NoAdd 实例化（与
    // 裁决实证形态同构：约束形参 + 运算符 + 未实现接口）
    let dir = temp_dir();
    let file = create_yx_file(
        &dir,
        "unsatisfied_constraint.yx",
        r#"
NoAdd: Type = {
    x: Int,
}

sum: (T: Add) -> (a: T, b: T) -> T = a + b

main: () -> Void = {
    a = NoAdd(1)
    b = NoAdd(2)
    v = sum(a, b)
}

main()
"#,
    );

    // Act - check 命令行为面 + 诊断集合面各取一次
    let result = check_file(&file);
    let checked = check_result(&file);

    // Assert - 翻面：check 从静默 0 错改为报 1 错，诊断码 E1106、
    // 文本携带改写失败的派发目标名 `NoAdd.add`
    assert!(
        result.is_ok(),
        "check should surface the miss as an error count, not a pipeline failure: {result:?}"
    );
    assert_eq!(
        result.unwrap(),
        1,
        "mono dispatch rewrite miss must be reported by check (E1106), not deferred to runtime E6006 (issue #519)"
    );
    let e1106: Vec<_> = checked
        .diagnostics
        .iter()
        .filter(|d| d.diagnostic.code == "E1106")
        .map(|d| &d.diagnostic)
        .collect();
    assert!(
        !e1106.is_empty(),
        "unsatisfied constraint must report E1106, got: {}",
        checked
            .diagnostics
            .iter()
            .map(|d| d.diagnostic.code.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );
    assert!(
        e1106.iter().any(|d| d.message.contains("NoAdd.add")),
        "E1106 message must name the failed dispatch target `NoAdd.add`, got: {:?}",
        e1106.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
}

/// #519 成因 B 锚（WBS 3.20.7 / 2026-11-01 用户裁决甲案续篇）：跨模块类型
/// 身份缺口的编译期显性化。类型与泛型定义在不同单元时，mono 无法从类型
/// 实参还原方法的定义单元键（`Vec3.add` 在合并函数表登记为 `lib.Vec3.add`，
/// WBS 3.18），此前回退裸名把失败推迟到运行期 E6006 `Vec3.add not found`。
///
/// 判据：函数表中存在其他单元键下的同名方法条目时，E1106 的 detail 必须列出
/// 候选键（确定性：sort 后取前 3）——「类型定义模块身份在单态化期丢失」
/// 从静默数据缺陷变成可诊断的编译期错误。
#[test]
fn test_check_constraint_dispatch_cross_module_miss_reports_candidates() {
    // Arrange - lib.yx 定义 Vec3 与 Vec3.add（合并函数表键 lib.Vec3.add）；
    // main.yx 定义 sum 并以 Vec3 实例化——方法所属单元键与泛型所属单元键不同
    let dir = create_project(&[
        (
            "lib.yx",
            r#"
Vec3: Type = {
    x: Float,
    y: Float,
    z: Float,
    Add(Vec3, Vec3, Vec3),
}

Vec3.add: (self: &Vec3, other: &Vec3) -> Vec3 = (self, other) => {
    return Vec3(self.x + other.x, self.y + other.y, self.z + other.z)
}
"#,
        ),
        (
            "main.yx",
            r#"
use lib.{Vec3}

sum: (T: Add) -> (a: T, b: T) -> T = a + b

main = () => {
    a = Vec3(1.0, 2.0, 3.0)
    b = Vec3(4.0, 5.0, 6.0)
    v = sum(a, b)
}
"#,
        ),
    ]);
    let main = dir.path().join("main.yx");

    // Act
    let checked = check_result(&main);

    // Assert - check 报 E1106，detail 列出其他单元键下的候选 lib.Vec3.add
    let e1106: Vec<_> = checked
        .diagnostics
        .iter()
        .filter(|d| d.diagnostic.code == "E1106")
        .map(|d| &d.diagnostic)
        .collect();
    assert!(
        !e1106.is_empty(),
        "cross-module dispatch miss must report E1106, got: {}",
        checked
            .diagnostics
            .iter()
            .map(|d| d.diagnostic.code.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );
    assert!(
        e1106.iter().any(|d| d.message.contains("lib.Vec3.add")),
        "E1106 detail must list the candidate key lib.Vec3.add, got: {:?}",
        e1106.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
}

// ============ 3.20.8 / #511 t3 low：谓词排除判据不得回归窄化形态 ============
//
// 判据单源后（inference/statements.rs 的 const 用途精筛读 `predicate_names`，
// 由 checker.rs 注入点用 `is_predicate_application` 同源预计算——内置谓词 /
// `predicate_defs` / `env.vars` 三源），若该判据回归为 3.20.8 前的窄化形态
// （只认 `Terminates` 字面量 ∪ 证明函数基类型表判名），证明函数谓词
// `IsPositive` 应用 `IsPositive(b)` 的实参位 `b` 会被类型位置扫描
// （`ast::name_used_as_type` 的 `Generic` 臂）误计入 const 泛型用途——形参名被
// 当作 const 绑定并替换成底层具体类型，精化约束在到达消费端之前就丢失
// （E1092 类症状，见 `const_param::used_as_const_in_annotation` 的
// `is_predicate` 注入）。

/// 谓词应用实参位不得计入 const 泛型用途（判据窄化回归锚）。
#[test]
fn test_check_predicate_application_arg_not_collected_as_const() {
    // Arrange: 证明函数谓词 + 形参位精化——谓词应用的实参 `b` 是编译期
    // 表达式（值），不是类型引用；`f` 的签名不含 const 候选，check 应零诊断。
    let dir = temp_dir();
    let file = create_yx_file(
        &dir,
        "predicate_const_anchor.yx",
        "IsPositive: (x: Int) -> Type = { x > 0 }\n\
         f: (b: IsPositive(b)) -> Int = { 0 }\n\
         main = () => { print(f(5)) }",
    );

    // Act
    let result = check_file(&file);

    // Assert: 判据正确 ⇒ 谓词实参位被排除 ⇒ 无误收类诊断。
    // 判据回归窄化时 `b` 会进 const_binders 并被替换，此处将出现 E10xx。
    let errors = result.expect("well-formed predicate refinement should check cleanly");
    assert_eq!(
        errors, 0,
        "predicate application arg must not enter const binder names (3.20.8 single-source criterion); got {errors} error(s)"
    );
}

/// 内置谓词 `Terminates` 的测度位同样不得计入 const 泛型用途（内置源锚）。
#[test]
fn test_check_terminates_measure_arg_not_collected_as_const() {
    // Arrange: 内置谓词无源码声明（RFC-027 §6.9），判据必须经内置源命中；
    // 返回位 `Terminates(b)` 的 `b` 是测度表达式，`a`/`b` 均为运行时值参数。
    let dir = temp_dir();
    let file = create_yx_file(
        &dir,
        "terminates_const_anchor.yx",
        "gcd: (a: Int, b: Int) -> Terminates(b) = {\n\
         \x20 if b == 0 { return a }\n\
         \x20 return gcd(b, a % b)\n\
         }\n\
         main = () => { print(gcd(48, 18)) }",
    );

    // Act
    let result = check_file(&file);

    // Assert: 内置谓词源命中 ⇒ 测度位排除 ⇒ 零诊断（rfc027a T1 验收面）。
    let errors = result.expect("well-formed Terminates annotation should check cleanly");
    assert_eq!(
        errors, 0,
        "Terminates measure arg must not enter const binder names (3.20.8 single-source criterion); got {errors} error(s)"
    );
}

// ============ 3.20.9 / #522：跨单元谓词可见性（Registry 面）============
//
// 判据改为声明级 namespace（`probe::TypeLevelNamespace`，全部可见单元 scan
// 并集）后，lib.yx 声明谓词、main.yx 声明精化签名的多文件形态里，谓词应用
// 实参位 `N` 在全部诊断中必须保持**未替换**。修前 Registry 阶段判据三臂不
// 完整 ⇒ `N` 被误收 const 用途并替换成底层类型，诊断文本出现
// `IsPositive(int64)`（E1012 `found 'IsPositive(int64)'`）。
//
// 注：夹具经 check 仍会报既有的 `IsPositive(N)` 形态诊断（导入谓词的精化
// 正格化在导入面不可见，属另一缺陷面，非本锚判定对象）；本锚只钉
// 「误替换形态不得回归」。

/// #522 回归锚：多文件谓词应用实参位不得被替换成底层类型。
#[test]
fn test_check_multifile_predicate_arg_not_misreplaced() {
    // Arrange: lib.yx 声明谓词、main.yx `use lib.{IsPositive}` 后声明
    // const 形参签名——谓词应用实参位 `N` 是编译期表达式（值），不是类型引用
    let dir = temp_dir();
    create_yx_file(&dir, "lib.yx", "IsPositive: (x: Int) -> Type = { x > 0 }\n");
    let main_file = create_yx_file(
        &dir,
        "main.yx",
        "use lib.{IsPositive}\n\
         \n\
         sum: (N: Int) -> (x: IsPositive(N)) -> Int = (x) => x\n\
         \n\
         main: () -> Void = {\n\
         \x20   print(sum(42)(5))\n\
         }\n\
         \n\
         main()\n",
    );

    // Act: 子进程跑 CLI check（诊断文本是判据面，run_check_command_once 只回计数）
    let output = Command::new(env!("CARGO_BIN_EXE_yaoxiang-rs"))
        .arg("check")
        .arg(&main_file)
        .current_dir(&dir)
        .output()
        .expect("failed to run yaoxiang-rs check");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    // Assert: 谓词应用实参位 `N` 不得被替换——诊断中不得出现 `IsPositive(int64)`
    // （`N` 误收 const 用途并被替换成底层类型的 #522 形态）
    assert!(
        !text.contains("IsPositive(int64)"),
        "#522: cross-unit predicate application arg N must stay unreplaced in all diagnostics; got: {text}"
    );
}

// ============ 3.20.10 / #523：curry 内层参数位精化按 base 统一 ============
//
// RFC-027 §6：精化是编译期约束，对统一**透明**——消费端按证明函数基类型
// （`proof_fn_bases`）剥到 base 再统一。形参位（外层参数组）在注册期已剥
// （3.20.8 既有锚覆盖）；curry 内层参数组的精化标注此前原样进入调用点
// 统一，基类型实参被假拒绝——E1002 `Expected type 'IsPositive(N)', found
// type 'int64'`（N 保持未替换，判据面正确，是消费端统一缺口，#522 教训
// 的同轴：诊断不得出现替换后的谓词实参）。
//
// 修法：调用点逐参统一前与注册期共用同一剥除实现
// （`strip_refined_with_proof_fns`，判据单源）。

/// #523 主锚：curry 内层参数位的精化在完整应用时按 base 统一，不假拒绝。
#[test]
fn test_check_curry_inner_refined_param_unifies_with_base() {
    // Arrange: 本地谓词声明 + const 柯里签名，精化在内层参数组——
    // 部分应用 `sum(5)` 合法（约束不在此评估）；完整应用 `sum(5)(3)` 的
    // 第二个实参对内层 `IsPositive(N)` 统一，必须按基类型 Int(64) 接受。
    let dir = temp_dir();
    let file = create_yx_file(
        &dir,
        "curry_inner_refinement.yx",
        "IsPositive: (x: Int) -> Type = { x > 0 }\n\
         sum: (N: Int) -> (x: IsPositive(N)) -> Int = { 0 }\n\
         main = () => { print(sum(5)(3)) }",
    );

    // Act
    let result = check_file(&file);

    // Assert: 内层精化按 base 统一 ⇒ 零诊断（issue #523 验收面）。
    // 消费端缺口回归时此处报 E1002 `Expected type 'IsPositive(N)', found
    // type 'int64'`（假拒绝形态）。
    let errors = result.expect("curry call with inner refined param should check cleanly");
    assert_eq!(
        errors, 0,
        "#523: curry inner-layer refined param must unify by proof-fn base type (IsPositive(N) -> Int(64)), not be falsely rejected; got {errors} error(s)"
    );
}

/// #523 补锚：精化位在**后续** curry 组（第三层）同样按 base 统一。
#[test]
fn test_check_curry_refinement_in_later_group_unifies_with_base() {
    // Arrange: 三层柯里，精化只在第三组（中间组是普通值参数）——剥除
    // 必须逐层生效，不依赖「精化在第一个内层组」的位置假设。
    let dir = temp_dir();
    let file = create_yx_file(
        &dir,
        "curry_later_group_refinement.yx",
        "IsPositive: (x: Int) -> Type = { x > 0 }\n\
         f: (N: Int) -> (a: Int) -> (x: IsPositive(N)) -> Int = { 0 }\n\
         main = () => { print(f(5)(1)(3)) }",
    );

    // Act
    let result = check_file(&file);

    // Assert: 第三次应用的实参对 `IsPositive(N)` 统一按 base 放行。
    let errors = result.expect("curry call with refinement in a later group should check cleanly");
    assert_eq!(
        errors, 0,
        "#523: refined param in a later curry group must unify by base type; got {errors} error(s)"
    );
}

/// #523 补锚：**多个**内层组同时带精化，逐实参各自剥 base。
#[test]
fn test_check_curry_refined_params_in_two_groups_unify_with_base() {
    // Arrange: 两层内层组的参数位都带精化——剥除必须按实参逐位生效
    //（第二、第三次应用各自对精化形态统一），不能只剥第一处。
    let dir = temp_dir();
    let file = create_yx_file(
        &dir,
        "curry_two_refined_groups.yx",
        "IsPositive: (x: Int) -> Type = { x > 0 }\n\
         g: (N: Int) -> (x: IsPositive(N)) -> (y: IsPositive(N)) -> Int = { 0 }\n\
         main = () => { print(g(5)(3)(4)) }",
    );

    // Act
    let result = check_file(&file);

    // Assert: 两处内层精化都按 base 统一 ⇒ 零诊断。
    let errors = result.expect("curry call with refined params in two groups should check cleanly");
    assert_eq!(
        errors, 0,
        "#523: refined params in every curry group must unify by base type; got {errors} error(s)"
    );
}

/// #523 反向锚（t3 复审建议）：剥 base 只对统一**透明**，不遮蔽真不匹配——
/// 内层精化位的实参类型真错时必须仍报错。没有这条，「剥除退化成跳过统一」
/// 的遮蔽型回归会让上面三个零诊断锚全绿。
#[test]
fn test_check_curry_refined_param_genuine_mismatch_still_reports() {
    // Arrange: 与主锚同构，但第二实参是 String——对剥后基类型 Int(64)
    // 是真不匹配，必须恰好报一条错（E1002 Expected type 'int64'）。
    // 诊断文本不得含 IsPositive 字样（#522 红线，由既有
    // test_check_multifile_predicate_arg_not_misreplaced 的文本面锚钉住）。
    let dir = temp_dir();
    let file = create_yx_file(
        &dir,
        "curry_refined_genuine_mismatch.yx",
        "IsPositive: (x: Int) -> Type = { x > 0 }\n\
         sum: (N: Int) -> (x: IsPositive(N)) -> Int = { 0 }\n\
         main = () => { print(sum(5)(\"str\")) }",
    );

    // Act
    let result = check_file(&file);

    // Assert: 真不匹配仍被拒绝 ⇒ 恰一条诊断；零诊断 = 统一被遮蔽（回归）。
    let errors = result.expect("check should complete for genuinely mismatched refined param");
    assert_eq!(
        errors, 1,
        "#523: stripping refinement must stay transparent to unification — a genuinely mismatched argument (String vs Int(64)) must still be rejected with exactly one error; got {errors} error(s)"
    );
}

// ============ 3.20.11 / #524：导入谓词 def 跨单元传播（甲案）============
//
// 谓词 def（`PredicateDef { param_name, param_type, constraint }` 三元组）
// 随 Registry 导出面携带：Export 增设 `predicate_def` 载荷，定义方
// `extract_module_info` 用与第零遍同源的构建判据提取，导入方
// `register_use_export` 按 use 门控并入 `env.predicate_defs`，并同构登记
// proof-fn 形态变量（本地面的 `add_type_definition` RFC-027 Phase 2.5 分支
// 同款）——模板面与剥基类型面在导入方同权可用。
//
// 可见性语义（2026-11-03 用户裁决，甲案的一部分）：use 是唯一跨单元名引入
// 机制，**未导入的谓词解析失败是正确行为**——解析跟函数/类型走同一扇门。

/// #524 主锚：导入谓词的精化签名在完整应用时零诊断。
#[test]
fn test_check_imported_predicate_def_propagates_full_application() {
    // Arrange: lib.yx 声明谓词、main.yx `use lib.{IsPositive}` 后声明精化
    // 签名并完整应用——修前 env.predicate_defs 与 proof_fn_bases 双缺，
    // E1012 + E1002（CLI 复现：target/yx-scratch 同构夹具，2026-11-03）
    let dir = temp_dir();
    create_yx_file(&dir, "lib.yx", "IsPositive: (x: Int) -> Type = { x > 0 }\n");
    let main_file = create_yx_file(
        &dir,
        "main.yx",
        "use lib.{IsPositive}\n\
         \n\
         sum: (N: Int) -> (x: IsPositive(N)) -> Int = (x) => x\n\
         \n\
         main: () -> Void = {\n\
         \x20   print(sum(5)(3))\n\
         }\n",
    );

    // Act
    let result = check_file(&main_file);

    // Assert: def 随导出传播、按 use 门控并入 ⇒ 模板正格化与剥基类型在
    // 导入方都可用 ⇒ 零诊断（义务验证含在内）
    let errors = result.expect("imported predicate refinement should check cleanly");
    assert_eq!(
        errors, 0,
        "#524: imported predicate def must propagate via Registry export surface — full application sum(5)(3) must check with 0 diagnostics; got {errors} error(s)"
    );
}

/// #524 补锚：部分应用形态同样零诊断（#523 教训同轴——curry 内层精化位
/// 在「先部分应用、再完整应用」路径上同样要按 base 统一）。
#[test]
fn test_check_imported_predicate_def_propagates_partial_application() {
    // Arrange: 与主锚同构，部分应用先落一个绑定、再经绑定完整应用
    let dir = temp_dir();
    create_yx_file(&dir, "lib.yx", "IsPositive: (x: Int) -> Type = { x > 0 }\n");
    let main_file = create_yx_file(
        &dir,
        "main.yx",
        "use lib.{IsPositive}\n\
         \n\
         sum: (N: Int) -> (x: IsPositive(N)) -> Int = (x) => x\n\
         \n\
         main: () -> Void = {\n\
         \x20   partial = sum(5)\n\
         \x20   print(partial(3))\n\
         }\n",
    );

    // Act
    let result = check_file(&main_file);

    // Assert: 部分应用产出的函数值与后续完整应用都按剥后基类型统一 ⇒ 零诊断
    let errors = result.expect("imported predicate partial application should check cleanly");
    assert_eq!(
        errors, 0,
        "#524: imported predicate def must propagate — partial application sum(5) then full application must check with 0 diagnostics; got {errors} error(s)"
    );
}

/// #524 use 门控锚：**未导入**的谓词解析失败是正确行为（可见性语义，
/// 2026-11-03 用户裁决）——use 是唯一跨单元名引入机制，解析跟函数/类型
/// 走同一扇门。此锚常驻：若日后把全部 def 无视 use 门控整体并入，此处翻红。
#[test]
fn test_check_unimported_predicate_use_still_rejected() {
    // Arrange: lib 声明两个谓词，main 只导入其一，另一谓词用于精化签名
    let dir = temp_dir();
    create_yx_file(
        &dir,
        "lib.yx",
        "IsPositive: (x: Int) -> Type = { x > 0 }\n\
         NonNegative: (x: Int) -> Type = { x >= 0 }\n",
    );
    let main_file = create_yx_file(
        &dir,
        "main.yx",
        "use lib.{IsPositive}\n\
         \n\
         sum: (N: Int) -> (x: NonNegative(N)) -> Int = (x) => x\n\
         \n\
         main: () -> Void = {\n\
         \x20   print(sum(5)(3))\n\
         }\n",
    );

    // Act
    let result = check_file(&main_file);

    // Assert: 未导入谓词的使用必须被拒绝（不钉具体诊断码——那是解析失败
    // 形态的实现细节；钉住的是「必须失败」这一语义决策面）
    let errors = result.expect("check should complete for unimported predicate use");
    assert!(
        errors >= 1,
        "#524 visibility: an imported-gated predicate NOT brought in by use must fail resolution (use is the only cross-unit name introduction); got {errors} error(s)"
    );
}

// ============ 3.20.16 / #527：多参数谓词的导入面与本地面同构 ============
//
// 多参数带约束 TypeDefinition 形态谓词（`SumUpTo: (n: Int, s: Int) -> Type =
// { s == n * 2 }`）在本地面由 `add_type_definition` 的 RFC-027 Phase 2.5 分支
// 登记为 proof-fn 形态变量（形参类型 = 签名全部形参，经
// `lookup_proof_fn_base_type` 按全形参查元数可解析）；导入面此前因 def 构建
// 判据按单参数收窄（`PredicateDef` 是单参数三元组）拿不到 def，
// `register_use_export` Type 臂回退 Struct poly，与本地面不同构 ⇒ 使用被
// E1012 + E1002 拒绝——#524 甲案「导入谓词与本地面同权」在多参数形态的
// 边界偏差。
//
// 修法 = proof-fn 形参清单随导出携带（`Export.proof_fn_params`，定义方
// `env.vars` 已登记形态直读，判据单源仍是 `add_type_definition` 与
// `PredicateDef::proof_fn_type`），导入方经形态构造单源重建同构变量。
// 两条不变量由锚常驻钉住：
// - 模板面不扩权：多参数谓词**不并入** `env.predicate_defs`（本地面的
//   第零遍同样不登记）——元数检查只走证明函数面
//   （`lookup_proof_fn_base_type`），expected = 声明形参个数；若模板面被
//   扩权（def 并入后 `try_resolve` 阶段 1 接管），元数边界锚会因 E1093
//   消失而翻红。
// - use 门控不变：新载荷只随 use 选中的导出物化，未导入仍拒
//   （2026-11-03 裁决语义，#524 门控锚同轴）。

/// #527 本地面现状锚：多参数谓词在本文件的签名形参位使用——钉住现状
/// （修前修后同绿：本地面的登记形态与解析路径零变化）。
#[test]
fn test_check_local_multiparam_predicate_signature_use_clean() {
    // Arrange: 多参数带约束 TypeDefinition 谓词 + 签名形参位完整应用
    let dir = temp_dir();
    let file = create_yx_file(
        &dir,
        "local_multiparam_predicate.yx",
        "SumUpTo: (n: Int, s: Int) -> Type = { s == n * 2 }\n\
         g: (b: Int) -> (x: SumUpTo(b, 4)) -> Int = (x) => x\n\
         main: () -> Void = { print(g(3)(5)) }",
    );

    // Act
    let result = check_file(&file);

    // Assert: 本地面多参数谓词可解析（现状零诊断，导入面修后须与此同判）
    let errors = result.expect("local multiparam predicate should check cleanly");
    assert_eq!(
        errors, 0,
        "#527: local surface for a multi-param constrained predicate must stay clean (registration form and resolution path unchanged by the fix); got {errors} error(s)"
    );
}

/// #527 主锚：同一谓词 use 导入后，签名形参位使用与本地面同判（零诊断）。
/// 修前红：E1012 `expected 'int64', found 'SumUpTo(b, int64::4)'` +
/// E1002（导入面回退 Struct poly，正格化不发生）。
#[test]
fn test_check_imported_multiparam_predicate_matches_local() {
    // Arrange: lib.yx 声明多参数谓词、main.yx `use lib.{SumUpTo}` 后同形态使用
    let dir = temp_dir();
    create_yx_file(
        &dir,
        "lib.yx",
        "SumUpTo: (n: Int, s: Int) -> Type = { s == n * 2 }\n",
    );
    let main_file = create_yx_file(
        &dir,
        "main.yx",
        "use lib.{SumUpTo}\n\
         \n\
         g: (b: Int) -> (x: SumUpTo(b, 4)) -> Int = (x) => x\n\
         \n\
         main: () -> Void = {\n\
         \x20   print(g(3)(5))\n\
         }\n",
    );

    // Act
    let result = check_file(&main_file);

    // Assert: 导入面拿到与本地同构的 proof-fn 形态登记 ⇒ 与本地面同判零诊断
    //（#524 甲案同权语义在多参数形态成立）
    let errors = result.expect("imported multiparam predicate should check cleanly");
    assert_eq!(
        errors, 0,
        "#527: an imported multi-param predicate must resolve exactly like the local surface (proof-fn form registered via use-gated export payload); got {errors} error(s)"
    );
}

/// #527 use 门控锚（多参数形态）：未由 use 引入的多参数谓词仍解析失败。
/// 新载荷（`proof_fn_params`）只随 use 选中面物化——若导入面把多参数谓词
/// 无视 use 门整体并入，此处翻红（与 #524 门控锚同轴，#524 锚只覆盖单参数）。
#[test]
fn test_check_unimported_multiparam_predicate_still_rejected() {
    // Arrange: lib 声明多参数谓词，main 只导入另一谓词，多参数谓词用于精化签名
    let dir = temp_dir();
    create_yx_file(
        &dir,
        "lib.yx",
        "SumUpTo: (n: Int, s: Int) -> Type = { s == n * 2 }\n\
         NonNegative: (x: Int) -> Type = { x >= 0 }\n",
    );
    let main_file = create_yx_file(
        &dir,
        "main.yx",
        "use lib.{NonNegative}\n\
         \n\
         g: (b: Int) -> (x: SumUpTo(b, 4)) -> Int = (x) => x\n\
         \n\
         main: () -> Void = {\n\
         \x20   print(g(3)(5))\n\
         }\n",
    );

    // Act
    let result = check_file(&main_file);

    // Assert: 未导入的多参数谓词使用必须被拒绝（不钉具体诊断码——同 #524
    // 门控锚的口径：钉住「必须失败」这一语义决策面）
    let errors = result.expect("check should complete for unimported multiparam predicate use");
    assert!(
        errors >= 1,
        "#527 visibility: a multi-param predicate NOT brought in by use must fail resolution (the new proof_fn_params payload materializes only through use-selected exports); got {errors} error(s)"
    );
}

/// #527 反向锚：导入后**真不满足**的使用仍被拒绝——同构化不得遮蔽统一
/// 的真不匹配（与 #523 反向锚同轴：剥基类型对统一透明，不是跳过统一）。
#[test]
fn test_check_imported_multiparam_predicate_genuine_mismatch_still_reports() {
    // Arrange: 与主锚同构，但第二实参是 String——对剥后基类型 Int(64)
    // 是真不匹配
    let dir = temp_dir();
    create_yx_file(
        &dir,
        "lib.yx",
        "SumUpTo: (n: Int, s: Int) -> Type = { s == n * 2 }\n",
    );
    let main_file = create_yx_file(
        &dir,
        "main.yx",
        "use lib.{SumUpTo}\n\
         \n\
         g: (b: Int) -> (x: SumUpTo(b, 4)) -> Int = (x) => x\n\
         \n\
         main: () -> Void = {\n\
         \x20   print(g(3)(\"str\"))\n\
         }\n",
    );

    // Act
    let result = check_file(&main_file);

    // Assert: 真不匹配仍被拒绝 ⇒ 恰一条诊断（E1002 int64 vs string，与本地面
    // 同形态逐字同判）；零诊断 = 同构化吞掉了真不匹配（回归）。
    let errors =
        result.expect("check should complete for genuinely mismatched imported predicate use");
    assert_eq!(
        errors, 1,
        "#527: aligning the import surface with the local surface must stay transparent to unification — a genuinely mismatched argument (String vs Int(64) base) must still be rejected with exactly one error; got {errors} error(s)"
    );
}

/// #527 元数边界锚（模板面不扩权的实证）：导入多参数谓词在绑定注解位用错
/// 元数，E1093 必须发射且 expected = 声明形参个数（2）——判据走证明函数面
/// （`lookup_proof_fn_base_type` 按全形参查）。若多参数 def 被错误并入
/// `env.predicate_defs`（模板面扩权），`try_resolve` 阶段 1 的单参数模型会
/// 吞掉本次元数失配（expected==1==found），E1093 消失——本锚翻红。
/// 本地面同形态（修前实测）：E1002 + E1093 `'SumUpTo' expected 2 argument(s),
/// found 1`，导入面修后须同判。
#[test]
fn test_check_imported_multiparam_predicate_wrong_arity_reports_e1093_by_declaration() {
    // Arrange: 导入多参数谓词，绑定注解位只给 1 个实参（声明有 2 个）
    let dir = temp_dir();
    create_yx_file(
        &dir,
        "lib.yx",
        "SumUpTo: (n: Int, s: Int) -> Type = { s == n * 2 }\n",
    );
    let main_file = create_yx_file(
        &dir,
        "main.yx",
        "use lib.{SumUpTo}\n\
         \n\
         main = {\n\
         \x20   y: SumUpTo(1) = 5\n\
         \x20   print(y)\n\
         }\n",
    );

    // Act: 子进程跑 CLI check（E1093 的 expected 数是判据面，
    // run_check_command_once 只回计数）
    let output = Command::new(env!("CARGO_BIN_EXE_yaoxiang-rs"))
        .arg("check")
        .arg(&main_file)
        .current_dir(&dir)
        .output()
        .expect("failed to run yaoxiang-rs check");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    // Assert: E1093 发射且元数按声明的 2 个形参判（模板面未扩权的实证——
    // expected 数是「按声明形参查元数」与「阶段 1 单参数模型」的分界）
    assert!(
        text.contains("E1093"),
        "#527: arity mismatch against an imported multi-param predicate must be rejected via the proof-fn face (E1093); if the template face were expanded, stage-1 try_resolve would swallow this mismatch and E1093 would vanish; got:\n{text}"
    );
    assert!(
        text.contains("expected 2 argument(s)"),
        "#527: E1093 must count against the declaration's full parameter list (expected = 2), not the stage-1 single-param model; got:\n{text}"
    );
}

// ============ 3.20.35 / #441：调用形谓词体的精化入口 ============
//
// 谓词体是用户函数调用（`{ check_nn(x) }` 形）时，类型应用走精化判定
// （RFC-027 §3.1 应用即调用 + Phase 2.5 第 4 级）。修法 = 声明面判据单源
// `probe::is_constraint_body_item` 收宽（内核 ConstExpr 形态之外认**纯断言
// 体**里的 `Expr(Generic)` 调用项），模板面（`predicate_defs`）与 proof-fn
// 登记面（`add_type_definition` Phase 2.5 分支）同批登记。
//
// 本组锚（准入门：条款 = RFC-027 §3.1/§4.3 + 病史 = issue #441 修前假拒绝
// E1003+E1002 实测）分四面：①主锚红→绿（违反值 E4018，子进程 CLI 断码）；
// ②缺失被调的当前映射（E4018 proof_error 位——E4020 保持既有消费面，
// 「缺失 → E4020」是 #441 期望中未实现的部分，登记为后续工作项）；
// ③lambda 声明形同判；④守卫与导入面边界。运行时满足值面在
// execution.rs（run_ok 端到端）。锚不进语料差分基线面（tests/yaoxiang），
// 本批红线零基线再生。

/// #441 守卫锚：带字段记录体里的 `Assert` 项不进谓词登记——调用形收窄到
/// **纯断言体**。若守卫失效（谓词模板面劫持带字段记录），`Wrap(3)` 注解
/// 会被正格化为 `Refined` 而非记录构造面，此处从 0 诊断翻红。
#[test]
fn test_check_record_with_assert_item_keeps_record_face() {
    // Arrange: 单参数记录 + Assert 表达式项（字段与断言并存 = RFC-027 §14
    // 依赖记录形态，不是 `{ 断言 }` 谓词形态）
    let dir = temp_dir();
    let file = create_yx_file(
        &dir,
        "record_assert_item.yx",
        "Wrap: (N: Int) -> Type = { v: Int, Assert(N > 0) }\n\
         \n\
         main: () -> Int = {\n\
         \x20   mut z: Wrap(3) = Wrap(3)\n\
         \x20   return z.v\n\
         }\n",
    );

    // Act
    let result = check_file(&file);

    // Assert: 记录构造面照常工作（修前修后同为 0 诊断）
    let errors = result.expect("check should complete for record with Assert item");
    assert_eq!(
        errors, 0,
        "#441 guard: a record body with fields must keep its record-construction face — the call-form entry widens to assertion-only bodies (`{{ assertion }}`) and must not hijack records with Assert items; got {errors} error(s)"
    );
}

/// #441 导入面同权锚：调用形谓词随 `use` 导入后，签名形参位使用与本地面
/// 同判（0 诊断）。def（模板面）与 proof-fn 形态变量随导出 payload 同批
/// 携带——若导入面漏掉任一登记，注解实参位会落类型命名空间（E1003 族），
/// 此处翻红。
#[test]
fn test_check_imported_call_form_predicate_signature_use_matches_local() {
    // Arrange: lib 声明调用形谓词体；main 导入后在签名形参位使用
    //（符号实参、柯里形态——与 #527 同款，义务面不触发证明执行）
    let dir = temp_dir();
    create_yx_file(
        &dir,
        "lib.yx",
        "check_nn: (v: Int) -> Bool = { v >= 0 }\n\
         NonNegative: (x: Int) -> Type = { check_nn(x) }\n",
    );
    let main_file = create_yx_file(
        &dir,
        "main.yx",
        "use lib.{NonNegative}\n\
         \n\
         g: (b: Int) -> (x: NonNegative(b)) -> Int = (x) => x\n\
         \n\
         main: () -> Void = {\n\
         \x20   print(g(3)(5))\n\
         }\n",
    );

    // Act
    let result = check_file(&main_file);

    // Assert: 导入面拿到与本地同构的登记（def 进模板面 + proof-fn 形态
    // 变量重建），签名位使用零诊断
    let errors = result.expect("check should complete for imported call-form predicate use");
    assert_eq!(
        errors, 0,
        "#441: an imported call-form predicate must resolve its annotation exactly like the local surface (predicate_def and proof-fn form var travel with the use-gated export payload); got {errors} error(s)"
    );
}

/// #441 主锚（红→绿）：调用形谓词体 + 违反值（y=-5 → check_nn(-5)=false）
/// 必须报 E4018（第 4 级证明调用经 pipeline `execute_proof_calls` 编译期
/// 执行被调函数，返回 false）。修前同夹具假拒绝 E1003（实参 `y` 落类型
/// 命名空间）+ E1002——两条旧码必须**消失**（红→绿证据）。
/// 证明调用诊断在 check_module **之后**执行，`check_file` 的计数通道只给
/// 数量——码级断言走子进程 CLI。
#[test]
fn test_check_call_form_predicate_body_violating_value_reports_e4018() {
    // Arrange: issue #441 原夹具（违反值）
    let dir = temp_dir();
    let file = create_yx_file(
        &dir,
        "call_form_violates.yx",
        "check_nn: (v: Int) -> Bool = { v >= 0 }\n\
         NonNegative: (x: Int) -> Type = { check_nn(x) }\n\
         \n\
         main: () -> Int = {\n\
         \x20   mut y: Int = -5\n\
         \x20   mut z: NonNegative(y) = y\n\
         \x20   return z\n\
         }\n",
    );

    // Act: 子进程跑 CLI check（E4018 的码是判据面，计数通道只回数量）
    let output = Command::new(env!("CARGO_BIN_EXE_yaoxiang-rs"))
        .arg("check")
        .arg(&file)
        .current_dir(&dir)
        .output()
        .expect("failed to run yaoxiang-rs check");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    // Assert: 精化违反经证明调用执行被调 → E4018；修前假拒绝的两条旧码
    // 不得复活
    assert!(
        !output.status.success(),
        "#441: the violating value must be rejected at compile time; got success:\n{text}"
    );
    assert!(
        text.contains("E4018"),
        "#441: the violating value must reach the level-4 proof call (execute_proof_calls runs check_nn(-5) = false → E4018); got:\n{text}"
    );
    assert!(
        !text.contains("E1003"),
        "#441: the pre-fix false rejection (E1003: annotation argument resolved as a type name) must be gone; got:\n{text}"
    );
    assert!(
        !text.contains("E1002"),
        "#441: the pre-fix secondary false rejection (E1002: annotation stayed a generic application) must be gone; got:\n{text}"
    );
}

/// #441 缺失被调锚（当前映射）：谓词体调用的函数不在模块内时，精化入口
/// 照常正格化，证明调用进 pipeline 后被调找不到 → E4018（proof_error
/// 说明位「未在 AST 中找到」）。诊断码口径：E4020 保持既有消费面不变
/// （生产发射点 = 导入记录模型证明来源前置检查 + `UnprovenReason::
/// ProofFunctionRequired` 的 verdict 映射）；「缺失 → E4020」是 #441 期望
/// 中未实现的部分，登记为后续工作项——本锚钉住当前真实映射，后续轮若
/// 转 E4020 须随该轮裁决同步改锚。
#[test]
fn test_check_call_form_predicate_body_missing_callee_reports_e4018() {
    // Arrange: 调用形谓词体，但 check_nn 无定义
    let dir = temp_dir();
    let file = create_yx_file(
        &dir,
        "call_form_missing_callee.yx",
        "NonNegative: (x: Int) -> Type = { check_nn(x) }\n\
         \n\
         main: () -> Int = {\n\
         \x20   mut y: Int = 5\n\
         \x20   mut z: NonNegative(y) = y\n\
         \x20   return z\n\
         }\n",
    );

    // Act: 子进程跑 CLI check
    let output = Command::new(env!("CARGO_BIN_EXE_yaoxiang-rs"))
        .arg("check")
        .arg(&file)
        .current_dir(&dir)
        .output()
        .expect("failed to run yaoxiang-rs check");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    // Assert: 入口面不因被调缺失而拒绝声明（无 E1003 族），执行面缺被调
    // → E4018 proof_error 位
    assert!(
        !output.status.success(),
        "#441: a missing callee must fail the proof-call execution; got success:\n{text}"
    );
    assert!(
        text.contains("E4018"),
        "#441: missing callee must surface as E4018 (proof_error slot: callee not found in module AST); got:\n{text}"
    );
    assert!(
        !text.contains("E1003"),
        "#441: the annotation must still resolve (entry face is independent of callee existence); got:\n{text}"
    );
}

/// #441 lambda 声明形主锚：同一调用形谓词体的 Assign/lambda 形
/// （`= (x) => check_nn(x)`）与 TypeDefinition 形同判——违反值 E4018。
/// 修前该形态经证明函数面 IR 路径执行，链接不了用户被调
/// （E4018 执行失败位「Native function not found」）——同码不同因。
#[test]
fn test_check_call_form_lambda_body_violating_value_reports_e4018() {
    // Arrange: lambda 形调用体谓词 + 违反值
    let dir = temp_dir();
    let file = create_yx_file(
        &dir,
        "call_form_lambda_violates.yx",
        "check_nn: (v: Int) -> Bool = { v >= 0 }\n\
         NonNegativeA: (x: Int) -> Type = (x) => check_nn(x)\n\
         \n\
         main: () -> Int = {\n\
         \x20   mut y: Int = -5\n\
         \x20   mut z: NonNegativeA(y) = y\n\
         \x20   return z\n\
         }\n",
    );

    // Act: 子进程跑 CLI check
    let output = Command::new(env!("CARGO_BIN_EXE_yaoxiang-rs"))
        .arg("check")
        .arg(&file)
        .current_dir(&dir)
        .output()
        .expect("failed to run yaoxiang-rs check");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    // Assert: lambda 形走模板面正确执行被调体 → E4018（返回 false）
    assert!(
        !output.status.success(),
        "#441: the lambda-form violating value must be rejected at compile time; got success:\n{text}"
    );
    assert!(
        text.contains("E4018"),
        "#441: the lambda-form violating value must reach the level-4 proof call via the template face (check_nn(-5) = false → E4018); got:\n{text}"
    );
    assert!(
        !text.contains("E1003"),
        "#441: the lambda form must not fall back to generic-application rejection (E1003); got:\n{text}"
    );
}

// ============ 3.20.18 / #551：单参数无标注形参谓词的导入面与本地面同构 ============
//
// 单参数谓词且形参无类型标注（`P: (x) -> Type = { x > 0 }`，约束体可转
// ConstExpr ⇒ def 有登记）时，两臂构造的 proof-fn 形态变量分歧：
// - 本地面 `add_type_definition`（RFC-027 Phase 2.5 分支）按签名形参
//   `filter_map` 收集——无标注形参被丢弃 ⇒ vars 形参表为空 ⇒ 不进
//   `proof_fn_bases`（`params.first()` 取不到基类型）⇒ 注解不精化，
//   值绑定/返回位使用被拒（E1012 + E1002，实测 HEAD 夹具）。
// - 导入面 `register_use_export` 的 #524 单参数臂按 `def.param_type` 重建
//   ——`collect_predicate_defs` 对无标注形参默认 Int(64) ⇒ 形参表
//   [Int(64)] ⇒ 进 `proof_fn_bases` ⇒ 同形态使用被放行（0 诊断）。
// 修法（#527 同族统一化）：`extract_module_info` 的 `proof_fn_params` 载荷
// 取消「def 缺席」分流——全部 proof-fn 形态统一直读定义方 `env.vars` 的
// 已登记形态；导入臂据此重建（单参数与多参数同通道），def 仍进
// `env.predicate_defs`（模板面 #524 语义不变）。修后导入面与本地面逐字
// 同判；本地面与带标注形态零变化。
// 判据单源不变：vars 直读已是单源（`add_type_definition` +
// `PredicateDef::proof_fn_type`），导入面照抄，不开第二份。

/// #551 前提锚：无标注单参数谓词的**本地面**使用形态——钉住现状
/// （本地面零变化：修前修后同红，恰 2 条诊断）。若本地面形态翻绿或
/// 诊断数漂移，说明本地面被本修之外的改动触碰，先查那笔改动。
#[test]
fn test_check_local_unannotated_singleparam_predicate_use_rejected() {
    // Arrange: 无标注单参数谓词 + 精化签名形参位使用（值经 curry 返回）
    let dir = temp_dir();
    let file = create_yx_file(
        &dir,
        "local_unannotated_singleparam_predicate.yx",
        "P: (x) -> Type = { x > 0 }\n\
         g: (b: Int) -> (v: P(b)) -> Int = (v) => v\n\
         main: () -> Void = { print(g(3)(5)) }",
    );

    // Act
    let result = check_file(&file);

    // Assert: 现状恰 2 条（E1012 返回位 + E1002 绑定校验）——本地面形态
    // 修前修后同红（钉现状，不钉诊断码：码面是发射点位置的实现细节）
    let errors = result.expect("local unannotated predicate use should check completely");
    assert_eq!(
        errors, 2,
        "#551 premise: local surface for an un-annotated single-param predicate must keep its current verdict (exactly 2 diagnostics: return-position + binding unification) — the fix only aligns the import surface, the local surface must not move; got {errors} error(s)"
    );
}

/// #551 主锚：同一谓词 use 导入后，同形态使用与本地面**同判**（恰本地
/// 同款 2 条诊断）。修前红：导入面 0 诊断（def.param_type 的 Int(64)
/// 默认值造出本地面没有的形参表，假放行）；修后导入臂改从 vars 已登记
/// 形态重建 ⇒ 与本地面逐字同判。
#[test]
fn test_check_imported_unannotated_singleparam_predicate_matches_local() {
    // Arrange: 同一谓词与本地面同款使用，分别在单文件（本地面）与
    // use 导入（导入面）两个夹具中跑 check
    let local_dir = temp_dir();
    let local_file = create_yx_file(
        &local_dir,
        "local.yx",
        "P: (x) -> Type = { x > 0 }\n\
         g: (b: Int) -> (v: P(b)) -> Int = (v) => v\n\
         main: () -> Void = { print(g(3)(5)) }",
    );
    let import_dir = temp_dir();
    create_yx_file(&import_dir, "lib.yx", "P: (x) -> Type = { x > 0 }\n");
    let import_file = create_yx_file(
        &import_dir,
        "main.yx",
        "use lib.{P}\n\
         \n\
         g: (b: Int) -> (v: P(b)) -> Int = (v) => v\n\
         \n\
         main: () -> Void = { print(g(3)(5)) }",
    );

    // Act
    let local_errors =
        check_file(&local_file).expect("local surface fixture should check completely");
    let import_errors =
        check_file(&import_file).expect("import surface fixture should check completely");

    // Assert: 导入面与本地面同判，且确有拒绝（≥1 防 0==0 平凡通过）——
    // 修前导入面 0 诊断与本地面 2 诊断分歧，本锚翻红
    assert_eq!(
        import_errors, local_errors,
        "#551: an imported un-annotated single-param predicate must produce exactly the same verdict as the local surface (proof-fn form rebuilt from the defining unit's registered vars shape, not def.param_type's Int(64) default); got import {import_errors} vs local {local_errors} error(s)"
    );
    assert!(
        import_errors >= 1,
        "#551: the aligned import surface must still reject this use (same as the local premise anchor) — zero diagnostics would mean the fix swallowed the divergence instead of aligning it; got {import_errors} error(s)"
    );
}

/// #551 反向锚：导入无标注谓词在**元数不符**的使用上仍被模板面拒绝
/// （E1093，expected = 声明的单参数模型 1）——统一化只换 proof-fn 变量的
/// 构造源，def 并入 `env.predicate_defs` 的模板面（#524 甲案）不得受影响。
/// 此锚常驻：若 def 载荷在重构中被丢掉，此处翻红。
#[test]
fn test_check_imported_unannotated_singleparam_predicate_wrong_arity_reports_e1093() {
    // Arrange: 导入无标注单参数谓词，绑定注解位给 2 个实参（声明 1 个）
    let dir = temp_dir();
    create_yx_file(&dir, "lib.yx", "P: (x) -> Type = { x > 0 }\n");
    let main_file = create_yx_file(
        &dir,
        "main.yx",
        "use lib.{P}\n\
         \n\
         main = {\n\
         \x20   y: P(1, 2) = 5\n\
         \x20   print(y)\n\
         }",
    );

    // Act: 子进程跑 CLI check（E1093 的 expected 数是判据面，
    // run_check_command_once 只回计数）
    let output = Command::new(env!("CARGO_BIN_EXE_yaoxiang-rs"))
        .arg("check")
        .arg(&main_file)
        .current_dir(&dir)
        .output()
        .expect("failed to run yaoxiang-rs check");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    // Assert: E1093 发射且元数按声明的单参数模型判（expected = 1）——
    // 模板面（try_resolve 阶段 1）未受统一化影响的实证
    assert!(
        text.contains("E1093"),
        "#551: wrong-arity use of an imported un-annotated predicate must still be rejected by the template face (E1093) — def propagation into env.predicate_defs is #524 semantics and must survive the vars-direct-read unification; got:\n{text}"
    );
    assert!(
        text.contains("expected 1 argument(s)"),
        "#551: E1093 must keep the stage-1 single-param model (expected = 1) on the import surface, same as the local surface; got:\n{text}"
    );
}

// ============ 3.20.19 / #550：绑定注解位谓词精化的块局部绑定同权 ============
//
// 绑定注解位（`y: SumUpTo(1, 2) = 5`）的谓词精化，规范面由 RFC-027 §3.1
// 定义：「编译期谓词在绑定位置使用时（参数声明、变量声明、返回类型）」
// 且「同样适用于局部变量声明」——SPEC type-system §7.1 同口径（写
// `b: Positive(b)` 是与强度声明「同一个动作」，绑定注解位无排除）。
//
// 实现面此前的缺口轴：`StatementChecker::check_var_stmt_inner`（顶层与
// 带 Fn 注解函数体的绑定）已做 Refined 正格化，而 `ExpressionInferrer::
// infer_stmt` 的 Assign 臂（**块局部绑定**——无注解块值 / if 分支 /
// 循环体 / spawn 体经 `infer_block` 走查）直接用 raw `Generic` 形态统一，
// Refined 转换未发生 ⇒ 可满足值被 E1002 假拒绝。修法：正格化判据提取为
// 共享 `refine_binding_annotation`（判据单源，禁令三），两处消费面同判。
//
// 已知相邻缺口（不在本修范围，另案）：导入面**多参数**谓词的约束求值
// （proof_execution 在主模块 AST 中找证明函数）跨模块不可达，E4018
// 「未在 AST 中找到」——顶层同样发生，与绑定注解位无关。

/// #550 主锚（单参数）：块局部绑定注解位的可满足谓词应用零诊断。
/// 修前红：E1002 `Expected type 'IsPositive(int64::5)', found type 'int64'`
/// （raw Generic 统一，Refined 转换未发生）。
#[test]
fn test_check_binding_annotation_predicate_singleparam_block_local_accepted() {
    // Arrange: 单参数谓词 + 主块（无注解块值，走 infer_block）内的绑定注解
    let dir = temp_dir();
    let file = create_yx_file(
        &dir,
        "binding_annotation_singleparam.yx",
        "IsPositive: (x: Int) -> Type = { x > 0 }\n\
         main = {\n\
         \x20   y: IsPositive(5) = 3\n\
         \x20   print(y)\n\
         }",
    );

    // Act
    let result = check_file(&file);

    // Assert: RFC-027 §3.1 绑定注解位谓词——可满足值（3 > 0）零诊断；
    // E1002 = Refined 转换未发生（回归）
    let errors = result.expect("single-param block-local predicate binding should check");
    assert_eq!(
        errors, 0,
        "#550: a satisfiable predicate application on a block-local binding annotation must be refined (RFC-027 §3.1 declares variable declarations as binding positions) and accepted with 0 diagnostics; got {errors} error(s)"
    );
}

/// #550 主锚（多参数）：块局部绑定注解位的可满足多参谓词应用零诊断。
/// 修前红：E1002 `Expected type 'SumUpTo(int64::1, int64::2)', found type 'int64'`。
#[test]
fn test_check_binding_annotation_predicate_multiparam_block_local_accepted() {
    // Arrange: 多参数谓词 + 主块内绑定注解（2 == 1 * 2 可满足）
    let dir = temp_dir();
    let file = create_yx_file(
        &dir,
        "binding_annotation_multiparam.yx",
        "SumUpTo: (n: Int, s: Int) -> Type = { s == n * 2 }\n\
         main = {\n\
         \x20   y: SumUpTo(1, 2) = 5\n\
         \x20   print(y)\n\
         }",
    );

    // Act
    let result = check_file(&file);

    // Assert: 多参数与本地面同判（issue #550 四组合矩阵的本地面多参数臂）
    let errors = result.expect("multi-param block-local predicate binding should check");
    assert_eq!(
        errors, 0,
        "#550: a satisfiable multi-param predicate application on a block-local binding annotation must be accepted with 0 diagnostics, matching the top-level surface; got {errors} error(s)"
    );
}

/// #550 反向锚：块局部绑定注解位**真不满足**的谓词应用仍被拒——拒绝判据
/// 走证明函数面（E4018 约束求值），不是退回类型不匹配（E1002），更不是吞诊断。
/// E1002 缺席是「Refined 转换已发生、统一对剥基透明」的实证。
#[test]
fn test_check_binding_annotation_predicate_unsatisfiable_still_rejected() {
    // Arrange: 与主锚同构，但约束不可满足（3 == 1 * 2 为假）
    let dir = temp_dir();
    let main_file = create_yx_file(
        &dir,
        "binding_annotation_unsatisfiable.yx",
        "SumUpTo: (n: Int, s: Int) -> Type = { s == n * 2 }\n\
         main = {\n\
         \x20   y: SumUpTo(1, 3) = 5\n\
         \x20   print(y)\n\
         }",
    );

    // Act: 子进程跑 CLI check（E4018 的码面是判据面，run_check_command_once
    // 只回计数）
    let output = Command::new(env!("CARGO_BIN_EXE_yaoxiang-rs"))
        .arg("check")
        .arg(&main_file)
        .current_dir(&dir)
        .output()
        .expect("failed to run yaoxiang-rs check");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    // Assert: 拒绝来自证明函数面（E4018 约束不满足），而非类型不匹配（E1002）
    assert!(
        text.contains("E4018"),
        "#550: an unsatisfiable predicate application on a block-local binding annotation must still be rejected via the proof-function face (E4018 constraint evaluation), not swallowed; got:\n{text}"
    );
    assert!(
        !text.contains("E1002"),
        "#550: the rejection must come from the constraint face — E1002 here would mean the refinement conversion did not happen and unification ran against the raw Generic form (the pre-fix false rejection); got:\n{text}"
    );
}

// ============ 3.20.12 / #442：注解收泛型调用返回值的两侧表示统一 ============
//
// 泛型函数调用点（monomorphize）返回的代入形态是**应用**——
// `Generic{Box, [Int64]}`；同名注解（`Box(Int)`）经 try_instantiate 展开成
// **记录** `Struct{Box, ...}`（与构造器直造值侧同形）。两侧语义相同、表示
// 不同，绑定校验直接 unify 必假拒绝：E1002 `Expected type 'Box', found type
// 'Box(int64)'`（issue #442 复现）。
//
// 修法：绑定校验前把 init 的具体应用展开成记录形态再统一（与值侧构造共用
// `expand_ctor_applications` 唯一实现）。mono 链不受影响——调用点返回位
// 代入与 instantiation_requests 均未触碰。

/// #442 主锚：带泛型类型实参的注解收泛型函数调用结果，零诊断。
#[test]
fn test_check_generic_call_result_binds_generic_annotated_type() {
    // Arrange: 泛型类型 Box + 泛型构造函数 make_box（声明返回位 `Box(T)`）；
    // `b: Box(Int) = make_box(9)` 是 issue #442 的假拒绝形态——修前报
    // E1002 Expected type 'Box', found type 'Box(int64)'。
    let dir = temp_dir();
    let file = create_yx_file(
        &dir,
        "generic_call_annotated_result.yx",
        "Box: (T: Type) -> Type = { value: T, tag: Int }\n\
         make_box: (T: Type) -> (v: T) -> Box(T) = { return Box(v, 0) }\n\
         main: () -> Void = {\n\
         \x20   b: Box(Int) = make_box(9);\n\
         \x20   print(b.value);\n\
         }",
    );

    // Act
    let result = check_file(&file);

    // Assert: 返回类型在调用点已代入 T:=Int，注解侧展开形态与之一致 ⇒ 零诊断。
    let errors = result.expect("generic call result should bind to annotated generic type");
    assert_eq!(
        errors, 0,
        "#442: annotated binding `b: Box(Int) = make_box(9)` must be accepted — the call-site return type Box(T) is instantiated at the call, so annotation and init differ only in representation (Generic app vs expanded record), not in type; got {errors} error(s)"
    );
}

/// #442 回归锚：无注解收泛型调用结果（既有绿形态不得变）。
#[test]
fn test_check_generic_call_unannotated_binding_still_accepted() {
    // Arrange: 与主锚同族但无注解——推断面（不受表示统一影响的路径）。
    let dir = temp_dir();
    let file = create_yx_file(
        &dir,
        "generic_call_unannotated.yx",
        "Box: (T: Type) -> Type = { value: T, tag: Int }\n\
         make_box: (T: Type) -> (v: T) -> Box(T) = { return Box(v, 0) }\n\
         main: () -> Void = {\n\
         \x20   a = make_box(9);\n\
         \x20   print(a.value);\n\
         \x20   print(a.tag);\n\
         }",
    );

    // Act
    let result = check_file(&file);

    // Assert: 无注解绑定与字段访问维持既有行为 ⇒ 零诊断。
    let errors = result.expect("unannotated generic call binding should keep passing");
    assert_eq!(
        errors, 0,
        "#442 regression: unannotated binding `a = make_box(9)` with field access was green before the fix and must stay green; got {errors} error(s)"
    );
}

/// #442 回归锚：构造器直造收注解（既有绿形态不得变）。
#[test]
fn test_check_generic_ctor_direct_construction_still_accepted() {
    // Arrange: `c: Box(Int) = Box(9, 0)`——值侧构造与注解同形（都是展开
    // 记录），表示统一段不介入（init 不是应用形态）。
    let dir = temp_dir();
    let file = create_yx_file(
        &dir,
        "generic_ctor_direct.yx",
        "Box: (T: Type) -> Type = { value: T, tag: Int }\n\
         main: () -> Void = {\n\
         \x20   c: Box(Int) = Box(9, 0);\n\
         \x20   print(c.value);\n\
         \x20   print(c.tag);\n\
         }",
    );

    // Act
    let result = check_file(&file);

    // Assert: 构造器直造维持既有行为 ⇒ 零诊断。
    let errors = result.expect("direct constructor call should keep passing");
    assert_eq!(
        errors, 0,
        "#442 regression: direct construction `c: Box(Int) = Box(9, 0)` was green before the fix and must stay green; got {errors} error(s)"
    );
}

/// #442 反向锚：表示统一对真不匹配透明，不遮蔽——注解实参真错仍拒绝。
#[test]
fn test_check_generic_call_result_genuine_mismatch_still_reports() {
    // Arrange: 与主锚同构，但注解实参是 String——调用点代入的是 int64，
    // 展开逐字段统一必然失败，必须仍报错。没有这条，「展开退化成跳过统一」
    // 的遮蔽型回归会让主锚的零诊断失去含金量（#523 反向锚同构先例）。
    let dir = temp_dir();
    let file = create_yx_file(
        &dir,
        "generic_call_result_genuine_mismatch.yx",
        "Box: (T: Type) -> Type = { value: T, tag: Int }\n\
         make_box: (T: Type) -> (v: T) -> Box(T) = { return Box(v, 0) }\n\
         main: () -> Void = {\n\
         \x20   b: Box(String) = make_box(9);\n\
         \x20   print(b.value);\n\
         }",
    );

    // Act
    let result = check_file(&file);

    // Assert: 真不匹配仍被拒绝 ⇒ 恰一条诊断。
    let errors = result.expect("check should complete for genuinely mismatched annotation");
    assert_eq!(
        errors, 1,
        "#442: expanding the call-site application must stay transparent to real mismatches — `b: Box(String) = make_box(9)` (T:=int64) must still be rejected with exactly one error; got {errors} error(s)"
    );

    // Assert（文本面）：E1002 文本逐字钉住原始形态（expected 侧是记录名、
    // found 侧是应用形态）。单靠计数断言会被「跳过统一引发 E1001 级联恰好
    // 一条」的遮蔽形态巧合满足——文本面能区分。
    let output = Command::new(env!("CARGO_BIN_EXE_yaoxiang-rs"))
        .arg("check")
        .arg(&file)
        .current_dir(&dir)
        .output()
        .expect("failed to run yaoxiang-rs check");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        text.contains("error [E1002] Expected type 'Box', found type 'Box(int64)'"),
        "#442: genuine mismatch must report E1002 verbatim against the original forms (record name on the expected side, application form on the found side); got: {text}"
    );
}

/// #442 边界探针：跨单元（use 导入）注解收泛型调用结果。
#[test]
fn test_check_cross_unit_generic_call_result_binds_annotation() {
    // Arrange: lib.yx 定义 Box + make_box，main.yx use 后注解收返回值。
    // 跨单元与单文件走同一绑定校验段（声明名管道 #511 提供的只是名字来源，
    // 表示统一发生在消费端），修前同样假拒绝、修后同批转绿——钉住现状。
    let dir = temp_dir();
    create_yx_file(
        &dir,
        "lib.yx",
        "Box: (T: Type) -> Type = { value: T, tag: Int }\n\
         make_box: (T: Type) -> (v: T) -> Box(T) = { return Box(v, 0) }\n",
    );
    let main_file = create_yx_file(
        &dir,
        "main.yx",
        "use lib.{Box, make_box}\n\
         \n\
         main: () -> Void = {\n\
         \x20   b: Box(Int) = make_box(9);\n\
         \x20   print(b.value);\n\
         }\n",
    );

    // Act
    let result = check_file(&main_file);

    // Assert: 跨单元形态与单文件同判据同源 ⇒ 零诊断。
    let errors = result.expect("cross-unit generic call result should bind to annotation");
    assert_eq!(
        errors, 0,
        "#442: cross-unit `b: Box(Int) = make_box(9)` via use must be accepted — the binding-level representation unification is shared by both units; got {errors} error(s)"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// #500：比较兜底臂的编译期守卫——排序比较拒绝跨类型（运行期 E6007 是底线防御）
// ─────────────────────────────────────────────────────────────────────────────

/// #500 编译期守卫：跨类型**排序**比较（`1 < "x"`）在编译期被拒（E1002）。
///
/// 这守的是比较的层级防御关系：`infer_binary` 的比较臂（BinOp::Lt/Le/Gt/Ge）
/// 对两侧 unify 失败按纪律拒绝（B9 修复，不再吞错误），运行期 exec_compare 的
/// type mismatch 兜底臂只是底线防御——编译期已拦的形态不该再走到运行期。
///
/// 实测记录（2026-11-13 HEAD）：任务原文设想守卫「静态 Void vs Int 比较被编译
/// 期拒绝」——该前提**不成立**：跨类型**相等**比较（`1 == "a"`、void 值 == Int）
/// 经 `infer_equality` 对非记录类型吞掉 unify 结果而编译期放行、运行期落 E6007。
/// 该差异当时按任务边界只报告不顺路修，已由 issue #549 收口（2026-11-20 用户
/// 裁决甲案）：`infer_equality` 非记录形态走同一「两侧同型」纪律（见下方 #549
/// 锚组）；本守卫钉住排序比较这一侧的既有纪律。
#[test]
fn test_check_cross_type_ordered_comparison_rejected() {
    // Arrange
    let dir = temp_dir();
    let file = create_yx_file(&dir, "lt_mismatch.yx", r#"x = 1 < "x""#);

    // Act
    let result = check_file(&file);

    // Assert
    let error_count = result.expect("check should not panic on cross-type < comparison");
    assert!(
        error_count > 0,
        "跨类型排序比较（1 < \"x\"）必须编译期拒绝（infer_binary 比较臂纪律），实际 error_count = {error_count}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// #549：Eq/Neq 非记录形态的「两侧同型」编译期守卫（2026-11-20 用户裁决甲案）
// ─────────────────────────────────────────────────────────────────────────────

// 先红后绿证据（实测）：2026-11-21 修前 HEAD 二进制探针——`1 == "a"` /
// `1 != "a"` / `1 == 2.5` / `[1] == ["x"]` / `(1, 2) == (1, "x")` /
// `(1, 2) == [1, 2]` 六形态 `yaoxiang-rs check` 全部 exit 0（吞 unify 恒放行，
// 标量异型与静态 Void 运行期落 E6007、容器异型族运行成功打印 false）；
// 修后同六形态全部报 E1002、exit 1。本组锚按 RFC-039 D59 先红后绿纪律钉住
// 甲案落地面；诊断码与文面与排序比较臂同族同文（E1002，resolve 后两侧形态）。

/// #549 主锚：跨类型**相等**标量比较（`1 == "a"`）编译期拒绝（E1002）。
///
/// 修前该形态编译放行、运行期才落 E6007（#500 查证实测）；甲案后与排序
/// 比较臂同纪律，编译期报 E1002。
#[test]
fn test_check_cross_type_equality_scalar_rejected() {
    // Arrange
    let dir = temp_dir();
    let file = create_yx_file(&dir, "eq_scalar_mismatch.yx", r#"x = 1 == "a""#);

    // Act
    let result = check_result(&file);

    // Assert
    let codes = error_codes(&result);
    assert!(
        codes.iter().any(|c| c == "E1002"),
        "跨类型相等比较（1 == \"a\"）必须编译期报 E1002（infer_equality 两侧同型纪律，#549 甲案），实际 codes = {codes:?}"
    );
}

/// #549 同臂锚：`!=` 与 `==` 走同一判定（infer_equality 同时承载两者），
/// 跨类型**不等**标量比较同样编译期报 E1002。
#[test]
fn test_check_cross_type_inequality_scalar_rejected() {
    // Arrange
    let dir = temp_dir();
    let file = create_yx_file(&dir, "neq_scalar_mismatch.yx", r#"x = 1 != "a""#);

    // Act
    let result = check_result(&file);

    // Assert
    let codes = error_codes(&result);
    assert!(
        codes.iter().any(|c| c == "E1002"),
        "跨类型不等比较（1 != \"a\"）必须编译期报 E1002（与 Eq 同臂，#549 甲案），实际 codes = {codes:?}"
    );
}

/// #549 混合数值锚：`1 == 2.5`（Int vs Float）编译期拒绝。
///
/// RFC-011b 的 Int~Float 混合 widening 只承诺**算术**（Add/Sub/Mul/Div），
/// 比较无此承诺（infer_binary 比较臂注释同款判据）；unify 无 Int~Float 规则
/// （solver.rs 仅同宽度同型相等），故 Eq 侧混合数值按异型拒绝。
#[test]
fn test_check_cross_type_equality_int_float_rejected() {
    // Arrange
    let dir = temp_dir();
    let file = create_yx_file(&dir, "eq_int_float_mismatch.yx", r#"x = 1 == 2.5"#);

    // Act
    let result = check_result(&file);

    // Assert
    let codes = error_codes(&result);
    assert!(
        codes.iter().any(|c| c == "E1002"),
        "混合数值相等比较（1 == 2.5）必须编译期报 E1002（widening 仅算术承诺，比较两侧同型），实际 codes = {codes:?}"
    );
}

/// #549 容器元素锚：`Vec(Int) == Vec(String)` 形态（`[1] == ["x"]`）编译期拒绝。
///
/// 修前该族编译放行且**运行成功打印 false**（3.20.20 探针矩阵四形态族之一，
/// #304 运行期语义）——甲案裁决点本体。
#[test]
fn test_check_cross_type_equality_container_element_rejected() {
    // Arrange
    let dir = temp_dir();
    let file = create_yx_file(&dir, "eq_container_elem_mismatch.yx", r#"x = [1] == ["x"]"#);

    // Act
    let result = check_result(&file);

    // Assert
    let codes = error_codes(&result);
    assert!(
        codes.iter().any(|c| c == "E1002"),
        "容器元素异型相等（[1] == [\\\"x\\\"]）必须编译期报 E1002（#549 甲案裁决点），实际 codes = {codes:?}"
    );
}

/// #549 跨容器锚：`(1, 2) == [1, 2]`（Tuple vs List）编译期拒绝。
///
/// 该形态原是语料 `tests/yaoxiang/01-syntax/operators/range_equality.yx:31`
/// 的行为断言（「false 而非类型错误」）；甲案落地后按语料标准迁入
/// `tests/yaoxiang/06-compile-errors/types/cross_container_equality_err.yx`
/// （expect: compile-error E1002），本锚钉住同一契约的 CLI 判定面。
#[test]
fn test_check_cross_type_equality_cross_container_rejected() {
    // Arrange
    let dir = temp_dir();
    let file = create_yx_file(&dir, "eq_cross_container.yx", r#"x = (1, 2) == [1, 2]"#);

    // Act
    let result = check_result(&file);

    // Assert
    let codes = error_codes(&result);
    assert!(
        codes.iter().any(|c| c == "E1002"),
        "跨容器相等比较（(1, 2) == [1, 2]）必须编译期报 E1002（#549 甲案），实际 codes = {codes:?}"
    );
}

/// #549 对照锚：**同型**相等比较零变化——标量、容器、变量、`Any` 槽位四形态
/// 编译零错误。`Any == Any` 是 std/test.yx 的 `assert_eq: (a: Any, b: Any)`
/// 槽位（unify 顶部类型规则放行）；借用操作数按 RFC-009 读穿透剥层
/// （std/list.yx 的 `list[i] == item` 形态 (A, &A)），同一性质在此锚内
/// 由 `Any` 槽位形态共同守护。
#[test]
fn test_check_same_type_equality_stays_accepted() {
    // Arrange
    let dir = temp_dir();
    let file = create_yx_file(
        &dir,
        "eq_same_type_ok.yx",
        r#"
eq_int = 1 == 1
eq_str = "a" == "b"
eq_list = [1, 2] == [1, 3]
eq_any: (a: Any, b: Any) -> Bool = (a, b) => {
    return a == b
}
eq_via_any = eq_any(1, 2)
"#,
    );

    // Act
    let result = check_result(&file);

    // Assert
    let codes = error_codes(&result);
    assert!(
        codes.is_empty(),
        "同型相等比较（标量/容器/变量/Any 槽位）必须编译零错误（既有放行面零变化），实际 codes = {codes:?}"
    );
}

/// #549 边界登记锚：接口存在类型包装值（`Animal` 值 == `Animal` 值）走**记录
/// 臂**结构推导、与 #549 非记录臂无关——接口名经 type_defs 解析为 Struct，
/// `as_user_record` 命中后按「全字段可比」推导；接口体字段全是方法签名
/// （fn 不可比），故报 E1101 而非 E1002。同实现/跨实现形态同判（mono 不感知
/// 存在包装，RFC-011a §表示：typecheck 层用户可见类型仍是接口名）。此锚钉住
/// 边界：#549 只改非记录臂，记录形态（含接口名形态）既有判据零变化。
#[test]
fn test_check_interface_existential_equality_field_inference_rejected() {
    // Arrange：接口 + 两实现 + 存在容器元素相等（同实现/跨实现各一处）
    let dir = temp_dir();
    let file = create_yx_file(
        &dir,
        "iface_existential_eq.yx",
        r#"
Animal: (Self: Type) -> Type = {
    speak: (self: &Self) -> String,
}
Dog: Type = {
    name: String,
    Animal(Dog),
}
Dog.speak: (self: &Dog) -> String = { return "Woof" }
Cat: Type = {
    lives: Int,
    Animal(Cat),
}
Cat.speak: (self: &Cat) -> String = { return "Meow" }
main: () -> Void = {
    animals: Vec(Animal) = [Dog("Rex"), Cat(9)]
    cross = animals[0] == animals[1]
    same_elem = animals[0] == animals[0]
}
main()
"#,
    );

    // Act
    let result = check_result(&file);

    // Assert
    let codes = error_codes(&result);
    let e1101_count = codes.iter().filter(|c| *c == "E1101").count();
    assert!(
        e1101_count >= 2,
        "接口存在类型包装值相等（Animal 值 == Animal 值）走记录臂结构推导报 E1101（字段全为方法签名不可比；同/跨实现同判），实际 codes = {codes:?}"
    );
    assert!(
        !codes.iter().any(|c| c == "E1002"),
        "接口名形态不得落入 #549 非记录臂（记录判据零变化），实际 codes = {codes:?}"
    );
}

/// 规范来源：RFC-004 括号语义（返回位置的 `(T)` 是「完整类型（值）」，柯里链条
/// 到此终止）+ RFC-010a 规则①（return 值与值级返回位统一，E1012）。
/// 病史门：issue #546 —— `make_box: (T: Type) -> ((v: T) -> Box(T))` 的体是
/// 块体 `return (v) => Box(v, 0)`（作者写的 return 语句，载荷是 lambda）时，
/// 值级分解的 wrapped_closure 探测漏认 `StmtKind::Expr(Expr::Return(Some(Lambda)))`
/// 表示（control_flow.rs 的 return 语句与 pratt 箭头糖是两种并存表示），
/// 类型泛型注册分支剥到最内层，E1012 假拒绝（expected 'Box(t38)'）。修后
/// expected = 括号声明的那一层（内层 Fn 整体）。
#[test]
fn test_check_paren_return_generic_block_body_lambda_accepted() {
    // Arrange：issue #546 原文夹具——类型泛型 + 括号返回 + 块体 return lambda
    let dir = temp_dir();
    let file = create_yx_file(
        &dir,
        "issue546.yx",
        r#"
Box: (T: Type) -> Type = { value: T, tag: Int }
make_box: (T: Type) -> ((v: T) -> Box(T)) = { return (v) => Box(v, 0) }
"#,
    );

    // Act
    let result = check_file(&file);

    // Assert：括号声明的「返回内层函数值」形态必须零错误（E1012 假拒绝不得复发）
    let error_count = result.expect("check should not panic on issue #546 fixture");
    assert_eq!(
        error_count, 0,
        "嵌套括号声明（泛型 + 块体 return lambda）必须按 RFC-004 括号语义通过类型检查，实际 error_count = {error_count}"
    );
}

/// 规范来源：RFC-010a 规则①（return 值与值级返回位统一，E1012）。
/// 病史门：issue #546 修复的**拒绝面**——wrapped_closure 探测扩臂（认手写
/// return 的载荷 lambda）后，同形态的**真不匹配**不得被吞成接受：块体
/// return 的载荷 lambda 返回类型错误（应产 Box(T)，实产 Int）必须仍报
/// E1012，且 expected 侧是括号声明层（内层 Fn 整体 `fn(T) -> Box(T)`）。
/// 计数 + 文本逐字双断言（#442 批次工艺：计数面会被级联巧合满足，文本面
/// 才能钉住诊断形状）。
#[test]
fn test_check_paren_return_generic_block_body_genuine_mismatch_still_reports() {
    // Arrange：同主锚形态，但 return 载荷 lambda 返回 Int（真不匹配）
    let dir = temp_dir();
    let file = create_yx_file(
        &dir,
        "issue546_mismatch.yx",
        r#"
Box: (T: Type) -> Type = { value: T, tag: Int }
make_box: (T: Type) -> ((v: T) -> Box(T)) = { return (v) => 42 }
"#,
    );

    // Act
    let result = check_file(&file);

    // Assert：恰 1 条 E1012，expected 侧为括号声明层（内层 Fn 整体）
    let error_count = result.expect("check should not panic on issue #546 mismatch fixture");
    assert_eq!(
        error_count, 1,
        "块体 return 载荷真不匹配必须恰报 1 条错误（不吞成接受），实际 error_count = {error_count}"
    );
    let output = Command::new(env!("CARGO_BIN_EXE_yaoxiang-rs"))
        .arg("check")
        .arg(&file)
        .current_dir(&dir)
        .output()
        .expect("failed to run yaoxiang-rs check");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    // 文本面钉语义形状（类型变量号随分配序漂移，不逐字钉全串）：
    // expected 必须是括号声明层（内层 Fn 整体 `fn(tX) -> Box(tX)`——出现
    // `-> fn(t` 即剥到了整链或别的层），found 是 int64 载荷 lambda。
    assert!(
        text.contains("error [E1012] return type mismatch"),
        "#546: genuine mismatch must still report E1012 (not swallowed into acceptance); got: {text}"
    );
    assert!(
        text.contains("expected 'fn(t") && !text.contains("-> fn(t"),
        "#546: expected side must be the paren-declared layer (inner Fn whole, no second fn group); got: {text}"
    );
    assert!(
        text.contains("Box(t") && text.contains("-> int64'"),
        "#546: found side must be the Int-returning lambda against a Box expectation; got: {text}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 3.20.21 / #538：方法面的编译期值参数实参证据检查（E1113，SPEC 6.9.2-B）
// ─────────────────────────────────────────────────────────────────────────────
//
// 病史门：issue #538 —— 方法调用 `b.get(y)(y)` 的 q=0（编译期值）槽被运行期
// 值填充时零诊断（整段判定走 NotTarget：调用点 `callee_generic_name` 对值
// 接收者的 FieldAccess 返回 None），同形函数调用 `f(y)(y)` 报错。修前红态
// 实测（2026-11-13，issue 修订 `0504e42e` 之后的 HEAD）：`mut y` 变体 check
// 0 error（本锚先红后绿的红态取证）。
//
// 判据单源：方法面与函数面共用同一发射点（`collect_const_arg_instantiation_request`）
// 与同一槽位判据（`bind_args_to_params` + 调用点形参名表），方法注册补写
// q=0 事实（`CallSiteSlots::const_params`），不造第二份判定。
//
// **判据口径注记（义务一查证结论）**：issue 原文夹具的 `y: Int = 3` 不是
// 运行期值——6.8.5-2a 值环境把「非 mut + 字面量初始化」的绑定登记为编译期
// 已知（先例提交 2fa43b2a，早于 issue 复现 revision），两个面对它都不报
// E1113。函数面对同一夹具报的是 E1002（特化类型强制机制：`f(3)` 把返回位
// 定成 `int64::3` 后外层实参 `int64` 不匹配），该机制属特化面、不在本批
// 接线范围（函数面零变化红线）。故本批的主锚用 `mut y` 变体钉住「真运行期
// 值」的红转绿，issue 原文夹具单独作现状锚。

/// #538 主锚（先红后绿）：方法面的 q=0 槽被**真运行期值**（`mut y`，无编译期
/// 值）填充必须报 E1113，位置指向该实参——与函数面同判据同出口。
#[test]
fn test_check_method_call_runtime_value_in_const_slot_reports_e1113() {
    // Arrange - issue #538 夹具的 `mut y` 变体：y 是真运行期绑定（6.8.5-2a
    // 不登记 mut 绑定的编译期值），把它填进 q=0 槽必须被证据检查拒绝
    let dir = temp_dir();
    let file = create_yx_file(
        &dir,
        "method_const_slot_runtime_value.yx",
        r#"
Box: Type = { v: Int }
Box.get: (N: Int) -> (n: N) -> Int = (n) => n
main: () -> Void = {
    b: Box = Box(v = 1)
    mut y: Int = 3
    r: Int = b.get(y)(y)
}
"#,
    );

    // Act
    let result = check_file(&file);
    let checked = check_result(&file);
    let codes = error_codes(&checked);

    // Assert - 翻面：check 从 0 错（修前红态实测）改为报 E1113
    let errors = result.expect("check should surface the evidence miss as an error count");
    assert_eq!(
        errors, 1,
        "#538: filling the q=0 slot of a method call with a runtime value must be rejected (E1113), not silently accepted; got {errors} error(s)"
    );
    assert_eq!(
        codes.len(),
        1,
        "#538: exactly one diagnostic expected, got: {codes:?}"
    );
    assert_eq!(
        codes[0], "E1113",
        "#538: the rejection must be E1113 (compile-time evidence, same code as the function face), got: {codes:?}"
    );
    // F3（3.20.21 复审）：主锚钉住位置——诊断落在**内层调用实参 y** 那一行
    // （`r: Int = b.get(y)(y)` 行；列指向第一个实参），与函数面先例
    // （compile_time_evidence 的 test_const_arg_missing_evidence_is_reported
    // 断言 `span.start.line`）同一纪律。行号按探针实测写死：夹具首行为空行。
    let diag = &checked.diagnostics[0].diagnostic;
    assert_eq!(
        diag.span.map(|s| s.start.line),
        Some(7),
        "#538: the diagnostic must sit on the inner call's runtime argument `y` (the `r:` line), not the declaration or the outer call"
    );
}

/// #538 合法形态锚：字面量与 const 表达式填 q=0 槽 ⇒ 修前修后同绿。
///
/// `b.get(3)(3)`（字面量）与 `b.get(2+3)(5)`（纯运算 const 表达式）都是
/// 6.9.2-B 判据下的合法证据，方法面接线不得误伤（修前实测 0 错）。
#[test]
fn test_check_method_call_compile_time_value_args_stay_accepted() {
    // Arrange
    let dir = temp_dir();
    let file = create_yx_file(
        &dir,
        "method_const_slot_legal_forms.yx",
        r#"
Box: Type = { v: Int }
Box.get: (N: Int) -> (n: N) -> Int = (n) => n
main: () -> Void = {
    b: Box = Box(v = 1)
    r: Int = b.get(3)(3)
    s: Int = b.get(2+3)(5)
}
"#,
    );

    // Act
    let result = check_file(&file);

    // Assert
    let errors = result.expect("check should not panic on legal method-face const args");
    assert_eq!(
        errors, 0,
        "#538: legal compile-time value arguments (literal / const expression) must stay accepted on the method face; got {errors} error(s)"
    );
}

/// #538 现状锚：issue 原文夹具（`y: Int = 3`，非 mut 字面量绑定）修后保持绿。
///
/// 判据：6.8.5-2a 值环境——非 mut + 字面量初始化的绑定编译期已知，是合法
/// 证据；方法面与函数面对它**同判据**（都不报 E1113）。函数面对同一夹具
/// 报 E1002 来自特化类型强制（`f(3)` 把 `n: N` 定成 `int64::3`），属特化面
/// 机制，方法面本批不接（见本节头注记）——同绿即判据单源的现状。
#[test]
fn test_check_method_call_compile_time_known_binding_stays_accepted() {
    // Arrange - issue #538 原文夹具逐字（`y: Int = 3` 非 mut）
    let dir = temp_dir();
    let file = create_yx_file(
        &dir,
        "method_const_slot_issue_fixture.yx",
        r#"
Box: Type = { v: Int }
Box.get: (N: Int) -> (n: N) -> Int = (n) => n
main: () -> Void = {
    b: Box = Box(v = 1)
    y: Int = 3
    r: Int = b.get(y)(y)
}
"#,
    );

    // Act
    let result = check_file(&file);

    // Assert - `y` 按 6.8.5-2a 是编译期已知值（合法证据）⇒ 修前修后同绿
    let errors = result.expect("check should not panic on the issue #538 fixture");
    assert_eq!(
        errors, 0,
        "#538: the issue fixture's `y: Int = 3` is compile-time known per the value env (6.8.5-2a) — legal evidence on both faces; got {errors} error(s)"
    );
}

// ── #538 修复轮（WBS 3.20.21 复审 F1）：方法绑定键的「解析-消费」窗口 ──────
//
// 病史门：3.20.21 t3 复审（冻结 rev 8355a283）F1——`infer_call_expr` 的顺序
// 是 infer(func) → 推断全部实参 → 判定，而判定函数的 None 分支才
// `std::mem::take` 状态位；实参自身的 FieldAccess 臂会在消费前清空/覆盖键，
// 判定读到的不再是被调方写入的键。修法：调用边界快照（infer(func) 之后立即
// take，经参数消费），判定不再读状态位。两条锚对应两类可观测后果，修前红态
// 均在 HEAD 8355a283 探针实测（2026-11-14）：
// - 实参是**普通记录字段**（`b.v`）：FieldAccess 臂进体清空键 ⇒ 判定取到
//   None ⇒ 正确的 E1113 被**静默吞掉**（check 0 error）；
// - 实参是**method 引用**（`f.other` 命中 `Foo.other`）：其键覆盖被调方的键
//   ⇒ 判定用**错方法**的槽位表发射 E1113，诊断点名错方法的 binder（'M'）。
//
// 修复轮另查出：状态位**不跨语句存活**（每条语句表达式新建 ExpressionInferrer，
// 构造器置 None）且值位置 method 引用在合法程序里被类型面拒绝（E1002，
// `fn(N)` 不与 `fn(int64)` 统一）——「残留键跨语句误读」在合法程序中不可构造，
// 快照修法对它是结构性消除（契约恢复），不设可观测锚。

/// #538 修复轮 F1 锚①（先红后绿）：实参里的**普通字段访问**清空方法绑定键，
/// 不得让 q=0 槽的证据检查被静默吞掉——修后报恰 1 条 E1113 点名 `Box.get`
/// 自己的 binder N，位置落在内层调用实参 `b.v` 上。
#[test]
fn test_check_method_call_field_access_arg_in_const_slot_reports_e1113() {
    // Arrange - `b.v` 是合法实参形态（普通记录字段），但它的 FieldAccess 推断
    // 会清空方法绑定键；q=0 槽 N 被运行期值填充的证据检查必须照常发射
    let dir = temp_dir();
    let file = create_yx_file(
        &dir,
        "method_const_slot_field_access_arg.yx",
        r#"
Box: Type = { v: Int }
Box.get: (N: Int) -> (n: N) -> Int = (n) => n
main: () -> Void = {
    b: Box = Box(v = 1)
    mut y: Int = 3
    r: Int = b.get(b.v)(y)
}
"#,
    );

    // Act
    let checked = check_result(&file);
    let codes = error_codes(&checked);

    // Assert - 修前红态实测：check 0 error（正确的 E1113 被清空键吞掉）
    assert_eq!(
        codes.len(),
        1,
        "an argument field access must not suppress the q=0 slot evidence check of the enclosing method call; got {codes:?}"
    );
    assert_eq!(
        codes[0], "E1113",
        "the rejection must be E1113 (compile-time evidence, same code as the function face), got: {codes:?}"
    );
    let diag = &checked.diagnostics[0].diagnostic;
    assert!(
        diag.message.contains("'N'"),
        "the diagnostic must name Box.get's own const binder N, not another method's binder; got: {}",
        diag.message
    );
    assert_eq!(
        diag.span.map(|s| s.start.line),
        Some(7),
        "the diagnostic must sit on the inner call's argument `b.v` (the `r:` line), not the declaration or the outer call"
    );
}

/// #538 修复轮 F1 锚②（先红后绿）：判定对象必须是被调方**自己的键**——
/// 实参 method 引用写键覆盖后，判定不得借**别的方法**的槽位表与 q=0 名发射。
#[test]
fn test_check_method_call_evidence_names_callee_own_const_binder() {
    // Arrange - 实参 `f.other` 命中方法绑定 `Foo.other`（写键覆盖被调方的键），
    // 同时它对 `Box.get` 的 q=0 槽 N 确实缺编译期证据
    let dir = temp_dir();
    let file = create_yx_file(
        &dir,
        "method_const_slot_wrong_key_arg.yx",
        r#"
Box: Type = { v: Int }
Box.get: (N: Int) -> (n: N) -> Int = (n) => n
Foo: Type = { w: Int }
Foo.other: (M: Int) -> (m: M) -> Int = (m) => m
main: () -> Void = {
    b: Box = Box(v = 1)
    f: Foo = Foo(w = 2)
    mut y: Int = 3
    r: Int = b.get(f.other)(y)
}
"#,
    );

    // Act
    let checked = check_result(&file);
    let codes = error_codes(&checked);

    // Assert - 修前红态实测：E1113 点名 'M'（错方法 Foo.other 的 binder）——
    // 判定取到了实参覆盖写入的键；修后必须点名 'N'
    assert_eq!(
        codes.len(),
        1,
        "the evidence check must fire exactly once for the inner call's q=0 slot; got {codes:?}"
    );
    assert_eq!(
        codes[0], "E1113",
        "the rejection must be E1113, got: {codes:?}"
    );
    let diag = &checked.diagnostics[0].diagnostic;
    assert!(
        diag.message.contains("'N'"),
        "the diagnostic must name Box.get's own const binder N (the callee's key), not the arg's method binder M; got: {}",
        diag.message
    );
    assert!(
        !diag.message.contains("'M'"),
        "the diagnostic must not name the arg's method binder M — the judgment must use the callee's key; got: {}",
        diag.message
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 3.20.23 / #539：跨模块导入的 const 泛型调用不检查证据（E1113，SPEC 6.9.2-B）
// ─────────────────────────────────────────────────────────────────────────────
//
// 病史门：issue #539 —— 定义方的 q=0（编译期值）事实登记在签名收集期写入的
// `PolyType` 与槽位表里，两条都是 checker 本地状态；导出面只取 `body` 丢弃
// binder，导入面构造 callable（`import_binding`）也不经槽位表注入链 ⇒ 两个
// 事实在导入面双缺，判定整段 `NotTarget`：运行期值填 q=0 槽静默通过（对照：
// 同签名本地定义报 E1113）。修前红态实测（2026-11-14，HEAD `a5f315f9` 的
// CLI 探针）：`use lib.{pick}` + `mut y` 变体 check 0 error。
//
// 判据单源：修法 = `Export` 携带定义方**已登记形态**（`const_binders` 直读
// vars 的 PolyType、`call_site_params` 直读签名收集期槽位表，不从 AST 重推），
// 导入方 `import_binding` 按本地面 `check_fn_stmt` 的同款登记形态重建——
// 判定函数（`collect_const_arg_instantiation_request`）零改动，本地面零变化。
//
// **判据口径注记（义务一查证结论）**：SPEC type-system §7.1 / RFC-011 §4.1
// 明文「调用侧义务按声明强度检查……无证据时在调用点报告」，无跨模块豁免；
// 义务本身的强制（静默通过 → 编译错误）已由 RFC-039 D62 用户裁决（不可逆），
// 本批与 #538/#550/#551 同构——把已裁决义务接进尚未覆盖的面（D59 C 类）。
//
// **随本批对齐的既有差异（如实登记）**：特化强制（6.8.7 单值类型）与证据检查
// 共用同一段判定与同一组事实，导入面供给事实后两者**同时**生效——issue 原文
// 夹具（`y: Int = 3` 非 mut）修后与本地面同判：第二层调用报 E1002
// （`int64::3` 单值强制），修前导入面 0 error（静默放行无效程序）。
// 限定调用（`use lib` + `lib.pick(...)`）与方法面跨模块导入不经本批接线
// （scope 无限定名条目 / 方法槽位不随导出——#553 面），保持既有不判。

/// #539 主锚（先红后绿）：跨模块导入的 const 泛型函数，q=0 槽被**真运行期值**
/// （`mut y`，无编译期值）填充必须报 E1113——与本地面同码同位置同被调方 binder。
#[test]
fn test_check_imported_const_generic_call_runtime_value_reports_e1113() {
    // Arrange - issue #539 夹具的 `mut y` 变体：lib 定义 const 泛型函数，
    // main 导入后用真运行期绑定填 q=0 槽
    let dir = temp_dir();
    create_yx_file(
        &dir,
        "lib.yx",
        "pick: (N: Int) -> (n: N) -> Int = (n) => n\n",
    );
    let file = create_yx_file(
        &dir,
        "main.yx",
        "use lib.{pick}\n\
         \n\
         main: () -> Void = {\n\
         \x20   mut y: Int = 3\n\
         \x20   r: Int = pick(y)(y)\n\
         }\n",
    );

    // Act
    let checked = check_result(&file);
    let codes = error_codes(&checked);

    // Assert - 翻面：check 从 0 错（修前红态实测）改为报恰 1 条 E1113，
    // 点名被调方自己的 binder N、位置指向内层调用的运行期实参行
    assert_eq!(
        codes.len(),
        1,
        "#539: filling the q=0 slot of an imported const-generic call with a runtime value must be rejected (E1113), not silently accepted; got {codes:?}"
    );
    assert_eq!(
        codes[0], "E1113",
        "#539: the rejection must be E1113 (same code as the local face), got: {codes:?}"
    );
    let diag = &checked.diagnostics[0].diagnostic;
    assert!(
        diag.message.contains("'N'"),
        "#539: the diagnostic must name the callee's own const binder N (carried through the export payload), got: {}",
        diag.message
    );
    assert!(
        !diag.message.contains("'n'"),
        "#539: the diagnostic must not name the value param n — the judgment must use the callee's q=0 slot table; got: {}",
        diag.message
    );
    assert_eq!(
        diag.span.map(|s| s.start.line),
        Some(5),
        "#539: the diagnostic must sit on the inner call's runtime argument `y` (the `r:` line), not the declaration or the outer call"
    );
}

/// #539 合法形态锚：字面量与 const 表达式填导入函数的 q=0 槽 ⇒ 修前修后同绿。
///
/// `pick(3)(3)`（字面量）与 `pick(2+3)(5)`（纯运算 const 表达式）都是
/// 6.9.2-B 判据下的合法证据，导入面接线不得误伤（修前实测 0 错；
/// 运行期两次调用分别产出 3 与 5——特化请求的跨模块消费不改变运行值）。
#[test]
fn test_check_imported_const_generic_call_compile_time_args_stay_accepted() {
    // Arrange
    let dir = temp_dir();
    create_yx_file(
        &dir,
        "lib.yx",
        "pick: (N: Int) -> (n: N) -> Int = (n) => n\n",
    );
    let file = create_yx_file(
        &dir,
        "main.yx",
        "use lib.{pick}\n\
         \n\
         main: () -> Void = {\n\
         \x20   r: Int = pick(3)(3)\n\
         \x20   s: Int = pick(2+3)(5)\n\
         }\n",
    );

    // Act
    let result = check_file(&file);

    // Assert
    let errors = result.expect("check should not panic on legal imported const args");
    assert_eq!(
        errors, 0,
        "#539: legal compile-time value arguments (literal / const expression) must stay accepted on the imported face; got {errors} error(s)"
    );
}

/// #539 特化对齐锚（先红后绿）：导入面供给 q=0 事实后，6.8.7 单值类型强制
/// 与本地面**同判**——`pick(3)(4)` 的第二层实参对 `int64::3` 是真不匹配，
/// 必须报 E1002（修前导入面 0 error 静默放行；本地面同一夹具修前即报 E1002）。
#[test]
fn test_check_imported_const_generic_call_specialization_matches_local() {
    // Arrange - 与本地面 E1002 形态同构的导入夹具
    let dir = temp_dir();
    create_yx_file(
        &dir,
        "lib.yx",
        "pick: (N: Int) -> (n: N) -> Int = (n) => n\n",
    );
    let file = create_yx_file(
        &dir,
        "main.yx",
        "use lib.{pick}\n\
         \n\
         main: () -> Void = {\n\
         \x20   r: Int = pick(3)(4)\n\
         }\n",
    );

    // Act
    let checked = check_result(&file);
    let codes = error_codes(&checked);

    // Assert - 导入面与本地面同判（特化面机制随同一组事实对齐，不是新语义）
    assert_eq!(
        codes.len(),
        1,
        "#539: the imported face must enforce the single-value type exactly like the local face; got {codes:?}"
    );
    assert_eq!(
        codes[0], "E1002",
        "#539: the mismatched second-layer argument must be E1002 (specialization enforcement), got: {codes:?}"
    );
}

// ============ 3.20.22 / #537：证明函数形参精化的嵌套义务在调用点被静默丢弃 ============
//
// 证明函数（返回 `Type` 的声明）的形参带精化时，对该证明函数的每次编译期调用
// 都须先满足其形参精化（以调用实参代入）——规范面是 RFC-027 §3.1（「`b: Positive(b)`
// 和 `f(5)` 一样，就是函数调用」）与 §3.3（「函数前置条件：调用 `divide(a, b)` 时，
// `b` 必须满足 `Positive` 的证据要么来自当前假设，要么来自实参自身的精化类型标注」）
// 的组合，§12.2 另规定 Unproven 不得静默放行。
//
// 实现面缺口：证明执行器（frontend/proof_execution.rs）只求值被调函数**体**，
// 从不检查其形参精化 ⇒ 义务链在第一层嵌套处断裂（issue #537：`f(42, 7)` 违反
// `Eq(7, 0)` 却零诊断；普通函数对照面 g(7, 42) 正确报 E4018）。修法 = 在唯一
// 执行点展开被调方自身的形参精化（实参代入 → 嵌套证明调用入队 / 闭合 Bool 判定），
// 判据单源：被调签名取自 typecheck 的 `bindings`（checker 归一化的唯一真相源）。
//
// 隔离结论（探针矩阵，3.20.22 实录）：静默与 IR 生成器的谓词命名空间无关；
// 模块级常量变体同一根因（常量值可由模块 AST 字面折叠解析）。嵌套实参含负
// literal 的 E2031 是另一条独立边界（issue #556），3.20.24 已收编——折叠面
// 经共享 const 求值器接受一元取负字面量形态（见下方 #556 锚）。

/// #537 主锚：证明函数形参精化的嵌套义务违反必须报 E4018，且参数段是义务链
/// 终点 `Eq(7, 0)` 的实参 `[Int(7), Int(0)]`（不是外层调用的 `[Int(7), Int(42)]`）。
/// 修前红：零诊断（义务被静默丢弃）。
#[test]
fn test_check_proof_fn_param_refinement_nested_obligation_reported() {
    // Arrange: Check 的形参 p 带精化 Eq(n, 0)；调用点 f(42, 7) 使 Check(7, 42)
    // 的体为真，但 Check 自己的形参精化 Eq(7, 0) 为假
    let dir = temp_dir();
    let file = create_yx_file(
        &dir,
        "proof_fn_nested_obligation.yx",
        "Eq: (a: Int, b: Int) -> Type = { a == b }\n\
         Check: (n: Int, p: Eq(n, 0)) -> Type = { n > 0 }\n\
         f: (n: Int, p: Check(7, n)) -> Int = { n }\n\
         main: () -> Void = {\n\
         \x20   y: Int = f(42, 7)\n\
         }",
    );

    // Act: 子进程跑 CLI check（断言义务链终点的参数段文本）
    let output = Command::new(env!("CARGO_BIN_EXE_yaoxiang-rs"))
        .arg("check")
        .arg(&file)
        .current_dir(&dir)
        .output()
        .expect("failed to run yaoxiang-rs check");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    // Assert: E4018 且参数段 = 被调证明函数自己形参精化的实参（义务链终点）
    assert!(
        text.contains("E4018"),
        "#537: f(42, 7) violates Check's own param refinement Eq(7, 0) and must be rejected with E4018; zero diagnostics means the nested obligation was silently dropped (issue #537 regression); got:\n{text}"
    );
    assert!(
        text.contains("[Int(7), Int(0)]"),
        "#537: the E4018 must carry the nested obligation's args [Int(7), Int(0)] (Check's param refinement Eq(n, 0) substituted with Check's own args), not the outer call's args [Int(7), Int(42)]; got:\n{text}"
    );
}

/// #537 模块级常量变体锚：精化引用模块级常量（`p: Eq(K, 0)`，`K: Int = 7`）时
/// 嵌套义务同样必须判定——常量值由模块 AST 折叠解析（与函数面缺值闸门的
/// 「非形参名不判」边界不同：这里常量编译期可知，义务可闭合）。
/// 修前红：零诊断（issue 复审登记的第二个丢义务变体）。
#[test]
fn test_check_proof_fn_param_refinement_module_const_obligation_reported() {
    // Arrange: Check 的形参精化引用模块级常量 K=7；Check(7, 0) 的体为真，
    // 嵌套义务 Eq(K, 0) = Eq(7, 0) 为假
    let dir = temp_dir();
    let file = create_yx_file(
        &dir,
        "proof_fn_nested_obligation_module_const.yx",
        "Eq: (a: Int, b: Int) -> Type = { a == b }\n\
         K: Int = 7\n\
         Check: (n: Int, p: Eq(K, 0)) -> Type = { n > -100 }\n\
         f: (n: Int, p: Check(n, 0)) -> Int = { n }\n\
         main: () -> Void = {\n\
         \x20   y: Int = f(7, 0)\n\
         }",
    );

    // Act
    let output = Command::new(env!("CARGO_BIN_EXE_yaoxiang-rs"))
        .arg("check")
        .arg(&file)
        .current_dir(&dir)
        .output()
        .expect("failed to run yaoxiang-rs check");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    // Assert: 嵌套义务 Eq(7, 0)（常量代入后）违反，报 E4018 带义务链终点实参
    assert!(
        text.contains("E4018"),
        "#537: the module-constant variant of the nested obligation (Eq(K, 0) with K=7) must be judged — constants are compile-time known, the obligation closes; got:\n{text}"
    );
    assert!(
        text.contains("[Int(7), Int(0)]"),
        "#537: the module-const variant must report the same obligation-chain endpoint args [Int(7), Int(0)]; got:\n{text}"
    );
}

/// #537 内联形态锚：嵌套精化是**已注册单参谓词**（Checker 把它内联成 Bool 约束，
/// 不留证明调用）时，嵌套义务同样必须判定——闭合 Bool 约束在执行器内直接求值。
/// 修前红：零诊断。
#[test]
fn test_check_proof_fn_param_refinement_inline_predicate_reported() {
    // Arrange: Check2 的形参精化 Positive(n) 走谓词内联面（约束 = n > 0 非 Call）；
    // Check2(0, 0) 的体为真，嵌套义务 0 > 0 为假
    let dir = temp_dir();
    let file = create_yx_file(
        &dir,
        "proof_fn_nested_obligation_inline.yx",
        "Positive: (x: Int) -> Type = { x > 0 }\n\
         Check2: (n: Int, p: Positive(n)) -> Type = { n > -100 }\n\
         f: (n: Int, p: Check2(n, 0)) -> Int = { n }\n\
         main: () -> Void = {\n\
         \x20   y: Int = f(0, 0)\n\
         }",
    );

    // Act
    let output = Command::new(env!("CARGO_BIN_EXE_yaoxiang-rs"))
        .arg("check")
        .arg(&file)
        .current_dir(&dir)
        .output()
        .expect("failed to run yaoxiang-rs check");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    // Assert: 闭合 Bool 约束违反 → E4018，约束文本是代入后的闭合式 (0 > 0)
    assert!(
        text.contains("E4018"),
        "#537: the inline-predicate variant (Positive(n) inlined to a Bool constraint) must also be judged — the substituted constraint (0 > 0) is closed and false; got:\n{text}"
    );
    assert!(
        text.contains("0 > 0"),
        "#537: the inline variant's E4018 must show the substituted closed constraint (0 > 0); got:\n{text}"
    );
}

/// #537 合法形态锚（分流判据第 2 条的控制面）：嵌套义务**成立**的合法程序
/// 零诊断——修复只把「原先非成功」的路径变响亮，不得让合法链开始报错。
#[test]
fn test_check_proof_fn_param_refinement_nested_valid_chain_accepted() {
    // Arrange: 同构于主锚，但实参使嵌套义务成立：Check(0, 0) 的体为真
    //（0 > -100），Check 的形参精化 Eq(0, 0) 也成立
    let dir = temp_dir();
    let file = create_yx_file(
        &dir,
        "proof_fn_nested_obligation_valid.yx",
        "Eq: (a: Int, b: Int) -> Type = { a == b }\n\
         Check: (n: Int, p: Eq(n, 0)) -> Type = { n > -100 }\n\
         f: (n: Int, p: Check(n, 0)) -> Int = { n }\n\
         main: () -> Void = {\n\
         \x20   y: Int = f(0, 0)\n\
         }",
    );

    // Act
    let result = check_file(&file);

    // Assert: 合法义务链零诊断（原先成功的程序不得因修复变失败）
    let errors = result.expect("valid nested obligation chain should check");
    assert_eq!(
        errors, 0,
        "#537: a program whose nested refinement obligations all hold (Eq(0, 0) is true) must stay accepted with 0 diagnostics — the fix may only turn previously-failing obligations loud; got {errors} error(s)"
    );
}

// ---- issue #556：负字面量证明调用实参在 try_proof_fn_call 的折叠 ----
//
// RFC-027 §3.1：`f(5)` 与 `f(-5)` 同为函数调用实参形态（词法 `Literal::Int`
// 为 i128，一元取负是表达式语法的一部分）——折叠面拒收负字面量 ⇒ 义务不进
// 证明管道，合法程序被 E2031 假拒绝（修前实测，3.20.24 探针 p1/p2/p5）。
// 修法：折叠面把一元取负形态交共享 const 求值器（i128 checked_neg，#507 口径）。
//
// 嵌套面无需改动（查证修正 issue 自述）：嵌套义务的实参来自①被调签名约束
//（声明侧负字面量已在类型域经共享项归一化折叠为 `MonoType::Literal`，
// mono.rs `GenericArg::as_type`）与②已折叠的 `ConstValue` 实参——`UnOp`
// 形态在那里不可达；声明侧负字面量的嵌套义务修前已正常判定（探针 p6/p8）。

/// #556 主锚（违反值）：`f(-5, 0)` 使义务 `Eq(-5, 0)` 为假——修前 E2031（合法
/// 义务被假拒绝），修后必须进证明管道报 E4018，且参数段是折叠后的实参。
#[test]
fn test_check_proof_fn_call_neg_literal_violation_reports_e4018() {
    // Arrange: Eq(-5, 0) 为假（a == b 证伪）
    let dir = temp_dir();
    let file = create_yx_file(
        &dir,
        "proof_fn_neg_literal_violation.yx",
        "Eq: (a: Int, b: Int) -> Type = { a == b }\n\
         f: (n: Int, p: Eq(n, 0)) -> Int = { n }\n\
         main: () -> Void = {\n\
         \x20   y: Int = f(-5, 0)\n\
         }",
    );

    // Act: 子进程跑 CLI check（断言诊断码与折叠后的义务实参文本）
    let output = Command::new(env!("CARGO_BIN_EXE_yaoxiang-rs"))
        .arg("check")
        .arg(&file)
        .current_dir(&dir)
        .output()
        .expect("failed to run yaoxiang-rs check");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    // Assert: E4018 且参数段为折叠后的 [Int(-5), Int(0)]——不得再是 E2031
    assert!(
        text.contains("E4018"),
        "#556: f(-5, 0) violates Eq(-5, 0) and must be rejected via the proof pipeline with E4018; a bare E2031 means the negative-literal arg was not folded (issue #556 regression); got:\n{text}"
    );
    assert!(
        text.contains("[Int(-5), Int(0)]"),
        "#556: the E4018 must carry the folded args [Int(-5), Int(0)]; got:\n{text}"
    );
    assert!(
        !text.contains("E2031"),
        "#556: the negative-literal obligation must reach the proof pipeline, not fall through to E2031; got:\n{text}"
    );
}

/// #556 满足值锚：`f(-5, 0)` 使义务 `Eq(-5, -5)` 成立——修前 E2031，修后
/// 零诊断编译通过（合法程序的假拒绝被修复，且不得反向新增其它诊断）。
#[test]
fn test_check_proof_fn_call_neg_literal_satisfied_compiles_clean() {
    // Arrange: Eq(n, n) 代入 -5 得 Eq(-5, -5)，义务成立
    let dir = temp_dir();
    let file = create_yx_file(
        &dir,
        "proof_fn_neg_literal_satisfied.yx",
        "Eq: (a: Int, b: Int) -> Type = { a == b }\n\
         f: (n: Int, p: Eq(n, n)) -> Int = { n }\n\
         main: () -> Void = {\n\
         \x20   y: Int = f(-5, 0)\n\
         }",
    );

    // Act
    let result = check_file(&file);

    // Assert: 零诊断（修前 E2031 的合法程序，修后编译通过）
    let errors = result.expect("satisfied negative-literal proof call should check");
    assert_eq!(
        errors, 0,
        "#556: f(-5, 0) with the always-true obligation Eq(-5, -5) must compile with 0 diagnostics; got {errors} error(s)"
    );
}

/// #556 嵌套面现状锚：被调签名里的声明侧负字面量（`p: Eq(n, -1)`）在嵌套
/// 义务展开处修前已正常判定（声明侧经共享项归一化折叠为字面量值，见上方
/// 查证注记）——本修不动嵌套面，该形态必须保持报 E4018。
#[test]
fn test_check_proof_fn_nested_declared_neg_literal_still_reported() {
    // Arrange: Check 的形参精化 Eq(n, -1) 代入 n=5 得 Eq(5, -1)，为假；
    // Check 的体 n > -100 在 n=5 处为真——E4018 只能来自嵌套义务
    let dir = temp_dir();
    let file = create_yx_file(
        &dir,
        "proof_fn_nested_declared_neg_literal.yx",
        "Eq: (a: Int, b: Int) -> Type = { a == b }\n\
         Check: (n: Int, p: Eq(n, -1)) -> Type = { n > -100 }\n\
         f: (n: Int, p: Check(n, 0)) -> Int = { n }\n\
         main: () -> Void = {\n\
         \x20   y: Int = f(5, 0)\n\
         }",
    );

    // Act
    let output = Command::new(env!("CARGO_BIN_EXE_yaoxiang-rs"))
        .arg("check")
        .arg(&file)
        .current_dir(&dir)
        .output()
        .expect("failed to run yaoxiang-rs check");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    // Assert: 嵌套义务 Eq(5, -1) 判伪报 E4018，参数段 [Int(5), Int(-1)]
    assert!(
        text.contains("E4018"),
        "#556: the nested obligation Eq(5, -1) with a declared negative literal must be judged (declaration-side negatives fold to literal values); got:\n{text}"
    );
    assert!(
        text.contains("[Int(5), Int(-1)]"),
        "#556: the nested E4018 must carry the substituted args [Int(5), Int(-1)]; got:\n{text}"
    );
}

// ---- RFC-004 外部重绑定：fn[pos] 极端位置值（issue #560）----

#[test]
fn test_check_binding_i64_min_position_reports_e1064_not_panic() {
    // Arrange — `T.m = f[-9223372036854775808]`：提取臂裸取负在溢出检查构建下
    // panic（探针 p1 实测 exit 101，panic 逐字 "attempt to negate with
    // overflow"，ast/mod.rs 取负臂）；修后极端负位（从末尾倒数 2^63 位）
    // 落 normalize_binding_positions 既有 E1064 判域（#560）。
    let dir = temp_dir();
    let file = create_yx_file(
        &dir,
        "binding_i64min.yx",
        r#"f: (a: Int, b: Int) -> Int = { return a + b }
T: Type = { x: Int }
T.m = f[-9223372036854775808]
"#,
    );

    // Act
    let result = check_result(&file);

    // Assert
    let codes = error_codes(&result);
    assert!(
        codes.contains(&"E1064".to_string()),
        "i64::MIN 形态绑定位置应报 E1064 而非 panic（#560）: {codes:?}"
    );
}

#[test]
fn test_check_binding_negative_position_valid_compiles() {
    // Arrange — 对照锚：边界内负位 [-1]（RFC-004 附录 A 从末尾计数），
    // 修前修后行为不变。
    let dir = temp_dir();
    let file = create_yx_file(
        &dir,
        "binding_neg1.yx",
        r#"f: (a: Int, b: Int) -> Int = { return a + b }
T: Type = { x: Int }
T.m = f[-1]
"#,
    );

    // Act
    let result = check_file(&file);

    // Assert
    assert!(
        result.is_ok(),
        "[-1] 负位绑定应通过类型检查（RFC-004 附录 A）"
    );
    assert_eq!(result.unwrap(), 0, "[-1] 负位绑定不应产生错误");
}

// ---- RFC-004 外部重绑定：fn[pos] 超 i64 位置字面量与 total=0 窗口（issue #563 甲案）----
//
// 用户 2026-11-20 裁决：超 i64 或超参数范围的位置字面量编译期拒绝（E1064），
// total=0 校验窗口一并收口。位置是结构性参数（决定实参槽位），截断是值字面量
// 的另一条既有语义——不在本判域。

#[test]
fn test_check_binding_out_of_i64_position_reports_e1064_not_truncate() {
    // Arrange — 顶层 `T.m = dist[18446744073709551617]`（2^64+1）+ 匿名风格
    // 参数（`(Int, Int)`）：修前提取器 `as i64` 截断为 [1]、normalize 遇
    // total=0 窗口放行，check 全绿绑定成功（issue #563 披露①实测）；修后
    // 提取器 Err → E1064。
    let dir = temp_dir();
    let file = create_yx_file(
        &dir,
        "binding_out_of_i64.yx",
        r#"dist: (Int, Int) -> Int = (x, y) => x + y
T: Type = { v: Int }
T.m = dist[18446744073709551617]
"#,
    );

    // Act
    let result = check_result(&file);

    // Assert
    let codes = error_codes(&result);
    assert!(
        codes.contains(&"E1064".to_string()),
        "2^64+1 位置字面量应报 E1064 而非静默截断绑定（#563 甲案）: {codes:?}"
    );
}

#[test]
fn test_check_binding_type_body_out_of_i64_position_reports_e1064() {
    // Arrange — 类型体 `m = dist[18446744073709551617]`：parser 提取器
    // （截断点②）修前 `as i64` 截断为 [1]、check 全绿；修后哨兵 i64::MAX
    // 落 normalize 判域报 E1064。
    let dir = temp_dir();
    let file = create_yx_file(
        &dir,
        "binding_type_body_overflow.yx",
        r#"dist: (Int, Int) -> Int = (x, y) => x + y
T: Type = {
    v: Int,
    m = dist[18446744073709551617],
}
"#,
    );

    // Act
    let result = check_result(&file);

    // Assert
    let codes = error_codes(&result);
    assert!(
        codes.contains(&"E1064".to_string()),
        "类型体 2^64+1 位置字面量应报 E1064 而非静默截断绑定（#563 甲案）: {codes:?}"
    );
}

#[test]
fn test_check_binding_negative_overflow_position_reports_e1064() {
    // Arrange — `T.m = dist[-18446744073709551615]`（-(2^64-1)）：修前取负臂
    // `as i64` 截断低 64 位得 [1]、check 全绿；修后提取器 Err → E1064。
    let dir = temp_dir();
    let file = create_yx_file(
        &dir,
        "binding_neg_overflow.yx",
        r#"dist: (Int, Int) -> Int = (x, y) => x + y
T: Type = { v: Int }
T.m = dist[-18446744073709551615]
"#,
    );

    // Act
    let result = check_result(&file);

    // Assert
    let codes = error_codes(&result);
    assert!(
        codes.contains(&"E1064".to_string()),
        "-(2^64-1) 位置字面量应报 E1064 而非静默截断绑定（#563 甲案）: {codes:?}"
    );
}

#[test]
fn test_check_binding_parser_neg_i64_min_form_reports_e1064_not_panic() {
    // Arrange — 类型体 `m = dist[-9223372036854775808]`（-i64::MIN 形态）：
    // parse_binding_positions 修前裸 `-i64::MIN` 溢出 panic（探针实测
    // types.rs:942 "attempt to negate with overflow"，#560 同族遗留）。
    // 用 CLI 子进程跑：panic 会把进程带走，诊断必须来自正常管线。
    let dir = temp_dir();
    let file = create_yx_file(
        &dir,
        "binding_parser_neg_i64min.yx",
        r#"dist: (Int, Int) -> Int = (x, y) => x + y
T: Type = {
    v: Int,
    m = dist[-9223372036854775808],
}
"#,
    );

    // Act
    let output = Command::new(env!("CARGO_BIN_EXE_yaoxiang-rs"))
        .arg("check")
        .arg(&file)
        .current_dir(&dir)
        .output()
        .expect("failed to run yaoxiang-rs check");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    // Assert
    assert!(
        !text.contains("panicked"),
        "-i64::MIN 形态绑定不得 panic（#560 同族遗留，#563 顺路修）: {text}"
    );
    assert!(
        text.contains("E1064"),
        "-i64::MIN 形态绑定应报 E1064（负位极端交 normalize 判域）而非 panic: {text}"
    );
}

#[test]
fn test_check_binding_out_of_range_position_on_unannotated_params_reports_e1064() {
    // Arrange — total=0 窗口绕过构造：匿名风格参数 `(Int, Int)` 的签名登记
    // 丢参（`filter_map(|p| p.ty)`），MonoType 参数虚减为 0，normalize 修前
    // 静默放行 `dist[5]`（CLI 实测 check 全绿）；修后以声明序值形参名表为
    // 元数事实源（2），[5] 越界报 E1064。
    let dir = temp_dir();
    let file = create_yx_file(
        &dir,
        "binding_window_bypass.yx",
        r#"dist: (Int, Int) -> Int = (x, y) => x + y
T: Type = { v: Int }
T.m = dist[5]
"#,
    );

    // Act
    let result = check_result(&file);

    // Assert
    let codes = error_codes(&result);
    assert!(
        codes.contains(&"E1064".to_string()),
        "匿名风格参数下 dist[5] 应报 E1064（RFC-004 pos < params.len()，#563 \
         窗口收口）而非静默绑定: {codes:?}"
    );
}

#[test]
fn test_check_binding_zero_arity_position_reports_e1064() {
    // Arrange — 真零元函数 + 显式位置 [0]：RFC-004 条款下 `0 >= 0` 恒无效，
    // 修前 total=0 窗口静默放行（CLI 实测 check 全绿）；修后 E1064。
    let dir = temp_dir();
    let file = create_yx_file(
        &dir,
        "binding_zero_arity.yx",
        r#"fn0: () -> Int = { return 7 }
T: Type = { v: Int }
T.m = fn0[0]
"#,
    );

    // Act
    let result = check_result(&file);

    // Assert
    let codes = error_codes(&result);
    assert!(
        codes.contains(&"E1064".to_string()),
        "零元函数的显式位置 [0] 应报 E1064（pos >= params.len()，#563 窗口收口）\
         而非静默绑定: {codes:?}"
    );
}

#[test]
fn test_check_binding_legal_positions_on_unannotated_params_still_compile() {
    // Arrange — 对照锚（合法位置逐字零变化）：匿名风格参数 `(Int, Int)` 的
    // [0] / [1] / [-1] 绑定——修前经 total=0 窗口放行，修后以真元数 2 校验
    // 仍放行；窗口收口不得误伤合法位置程序（#563 语义边界硬判据）。
    let dir = temp_dir();
    let file = create_yx_file(
        &dir,
        "binding_legal_unannotated.yx",
        r#"dist: (Int, Int) -> Int = (x, y) => x + y
T: Type = { v: Int }
T.m0 = dist[0]
T.m1 = dist[1]
T.mm1 = dist[-1]
"#,
    );

    // Act
    let result = check_result(&file);

    // Assert
    let codes = error_codes(&result);
    assert!(
        codes.is_empty(),
        "匿名风格参数的合法位置 [0]/[1]/[-1] 绑定应零诊断通过（对照锚），\
         实际: {codes:?}"
    );
}

// ==================== 深表达式资源闸锚（issue #540 / WBS 3.20.30） ====================
//
// 规范来源：RFC-013 错误码规范 E0019/E3014 + D59 C 类崩溃面直修授权
// （用户裁决：大栈线程消解 razor + 解析深度闸 1024 + 新码 E0019）。
//
// razor 区实测基线（debug 构建 / 1MB 主线程栈，修前逐层实测）：
// - 调用嵌套修前编译成功至 123 层、崩溃起点 124 层（exit 0xC00000FD
//   零诊断）——razor（修前崩溃修后成功）自 124 层起；修后（大栈线程）
//   125 层编译成功——修前非成功面，换行为合法；
// - 块嵌套 220 层修前编译成功（exit 0），修后必须逐字不变
//   （「原先成功程序对外表现不变」判据的 razor 锚）。

#[test]
fn test_check_deep_call_razor_boundary_compiles_without_crash() {
    // Arrange：125 层调用嵌套——修前崩溃区内（实测崩溃起点 124 层，
    // exit 127 / 0xC00000FD），修后大栈线程上编译成功（寄存器占用未超
    // E3014 上限）
    let dir = temp_dir();
    let expr = format!("{}1{}", "id(".repeat(125), ")".repeat(125));
    let file = create_yx_file(
        &dir,
        "razor_call.yx",
        &format!(
            "id: (Int) -> Int = (x) => x\nmain = () => {{\n    y = {expr}\n    print(y)\n}}\n"
        ),
    );

    // Act
    let result = check_file(&file);

    // Assert：非崩溃且恰 0 错误（崩溃 → 成功的 C 类转换锚）
    assert!(
        result.is_ok(),
        "125 层调用嵌套不得崩溃（修前 exit 0xC00000FD）"
    );
    assert_eq!(result.unwrap(), 0, "125 层调用嵌套修后编译成功");
}

#[test]
fn test_check_deep_block_razor_boundary_unchanged() {
    // Arrange：220 层块嵌套——修前编译成功（razor 区通过锚），修后
    // 必须保持零诊断（原先成功程序逐字不变）
    let dir = temp_dir();
    let source = format!("main = () => {}1{}\n", "{ ".repeat(220), " }".repeat(220));
    let file = create_yx_file(&dir, "razor_block.yx", &source);

    // Act
    let result = check_file(&file);

    // Assert
    assert!(result.is_ok(), "220 层块嵌套不得报错（修前即编译成功）");
    assert_eq!(result.unwrap(), 0, "修前成功的 razor 形态行为逐字不变");
}

#[test]
fn test_check_normal_depth_expression_zero_diagnostics() {
    // Arrange：50 项二元链实参——远低于一切资源闸的正常形态（对照锚：
    // 限内程序诊断零变化）
    let dir = temp_dir();
    let expr = "1 + ".repeat(50) + "1";
    let file = create_yx_file(
        &dir,
        "normal_depth.yx",
        &format!(
            "id: (Int) -> Int = (x) => x\nmain = () => {{\n    y = id({expr})\n    print(y)\n}}\n"
        ),
    );

    // Act
    let result = check_file(&file);

    // Assert
    assert!(result.is_ok(), "正常深度形态不得报错");
    assert_eq!(result.unwrap(), 0, "对照锚：限内程序零诊断");
}

// ─────────────────────────────────────────────────────────────────────────────
// 3.20.32 / #553：方法面证据检查的剩余形态（E1113，SPEC type-system §7.1）
// ─────────────────────────────────────────────────────────────────────────────
//
// 病史门：issue #553 —— #538（3.20.21）接线了「变量接收者 + 无接收者位声明」
// 形态后，其余方法形态的 q=0（编译期值）槽被运行期值填充仍零诊断：
// ① 显式 self 接收者（`Box.get: (self: &Box, N: Int) -> (n: N) -> Int`）——
//    接收者占签名 params[0]，`method_binding_call_key` 命中后判定整段让路；
//    且柯里化签名的槽位表是全链拍平名，首次调用只填外层位 ⇒ 共享绑定判据的
//    缺参校验把内层位当缺参退回恒等对位，恒等落在接收者槽上（非 const 位）；
// ③ 外部绑定（`Box.get = f`，含类型体 `get = f`）——只登记 `method_bindings`，
//    调用点槽位查表缺项 ⇒ NotTarget。
// 修前红态实测（2026-11-17，CLI 探针，HEAD `72f12e99` 纯净构建）：下列四个
// 主锚夹具的 `mut y` 变体全部 check 0 error；对照夹具（字面量实参）同批实测
// 0 error（合法证据两形态都放行）。
//
// 形态②查证结论（如实登记）：类型体方法带**方法自带 q=0 槽**的注解在当前
// 语言不可构造——方法自己的 q=0 参数名在类型体注解作用域外（E1003，探针
// p2_min_curried）；借类型自身的 q=0 参数名且体未引用时报 E1094（探针
// p2_anon_typeslot）；补引用后构造位报 E1002（探针 p2_anon_typeslot2）。可构造
// 的类型体方法是 External/DefaultExternal 绑定（`{ get = f }`），它与顶层
// `Type.method = f` 走同一登记口 ⇒ 本批由同一条槽位登记路径覆盖（主锚②）。
// 类型体 Anonymous 绑定（`((..)->R)[pos] = lambda`）注册 method_bindings 但
// 无法声明 q=0 槽 ⇒ 无 E1113 面，不登记空条目。无 [pos] 的
// `{ get: FnType = lambda }` 是**默认值字段**而非方法绑定，其调用是记录形态
// 调用（`method_binding_call_key` 依既有判据返回 None），不经方法面链路。
//
// 判据单源：本批不改 E1113 判定（槽位定位、证据判据、发射点沿用 #538 同段）；
// 修法只有两处注入——① 判定入口的接收者位偏移（`method_binding_call_key` 的
// 既有接收者校验为唯一判据，实参对位沿用共享 `bind_args_to_params` 的
// `recv_offset` 形参——与调用侧 `bind_named_args` 的 offset 1 同一口径），
// ③ 外部绑定登记口补写槽位事实（直读被绑函数已登记形态，#539 同纪律）。
//
// 跨模块方法导入不在本批（issue #553 face ④ / #539 轴）：方法槽位事实需随
// `ModuleInfo.method_bindings` 跨界的平行载体（该方法类型本身已跨界——
// `extract_module_info` 克隆 env.method_bindings、driver 注入 env——但槽位表
// `declared_call_site_params` 无对应载体），且方法随 use 的门控粒度属语义设计
// 面 ⇒ 另案登记，本批保持既有不判。

/// #553 主锚①（先红后绿）：显式 `self` 接收者方法的 q=0 槽被**真运行期值**
/// （`mut y`）填充必须报 E1113，点名被调方自己的 binder N——与 #538 锚同判据。
#[test]
fn test_check_method_with_explicit_receiver_runtime_value_in_const_slot_reports_e1113() {
    // Arrange - 形态①：声明含接收者位（params[0] = &Box），q=0 槽 N 在内层
    // 柯里位；`mut y` 按 6.8.5-2a 是真运行期绑定（无编译期值）
    let dir = temp_dir();
    let file = create_yx_file(
        &dir,
        "method_self_recv_const_slot_runtime_value.yx",
        r#"
Box: Type = { v: Int }
Box.get: (self: &Box, N: Int) -> (n: N) -> Int = (n) => n
main: () -> Void = {
    b: Box = Box(v = 1)
    mut y: Int = 3
    r: Int = b.get(y)(y)
}
"#,
    );

    // Act
    let checked = check_result(&file);
    let codes = error_codes(&checked);

    // Assert - 翻面：check 从 0 错（修前红态实测）改为恰 1 条 E1113，
    // 点名被调方 binder N、位置指向填槽实参行
    assert_eq!(
        codes.len(),
        1,
        "#553 form①: filling the q=0 slot of an explicit-receiver method call with a runtime value must be rejected (E1113), not silently accepted; got {codes:?}"
    );
    assert_eq!(
        codes[0], "E1113",
        "#553 form①: the rejection must be E1113 (same code as the function face), got: {codes:?}"
    );
    let diag = &checked.diagnostics[0].diagnostic;
    assert!(
        diag.message.contains("'N'"),
        "#553 form①: the diagnostic must name Box.get's own const binder N; got: {}",
        diag.message
    );
    assert_eq!(
        diag.span.map(|s| s.start.line),
        Some(7),
        "#553 form①: the diagnostic must sit on the slot-filling argument `y` (the `r:` line), not the declaration"
    );
}

/// #553 主锚③a（先红后绿）：顶层外部绑定 `Box.get = f` 的 q=0 槽被运行期值
/// 填充必须报 E1113——绑定登记口补写槽位事实后，与本地方法声明同判。
#[test]
fn test_check_external_binding_runtime_value_in_const_slot_reports_e1113() {
    // Arrange - 形态③（接收者无位绑定）：被绑函数首参不是接收者类型 ⇒
    // `method_binding_call_key` 不命中，实参直对声明槽位（recv_offset 0）
    let dir = temp_dir();
    let file = create_yx_file(
        &dir,
        "external_binding_const_slot_runtime_value.yx",
        r#"
Box: Type = { v: Int }
get_impl: (N: Int) -> (n: N) -> Int = (n) => n
Box.get = get_impl
main: () -> Void = {
    b: Box = Box(v = 1)
    mut y: Int = 3
    r: Int = b.get(y)(y)
}
"#,
    );

    // Act
    let checked = check_result(&file);
    let codes = error_codes(&checked);

    // Assert - 翻面：check 从 0 错（修前红态实测）改为恰 1 条 E1113，
    // 点名被绑函数登记的 q=0 名 N
    assert_eq!(
        codes.len(),
        1,
        "#553 form③: filling the q=0 slot of an externally bound method with a runtime value must be rejected (E1113); got {codes:?}"
    );
    assert_eq!(
        codes[0], "E1113",
        "#553 form③: the rejection must be E1113 (same code as the declaration face), got: {codes:?}"
    );
    let diag = &checked.diagnostics[0].diagnostic;
    assert!(
        diag.message.contains("'N'"),
        "#553 form③: the diagnostic must name the bound function's q=0 name N; got: {}",
        diag.message
    );
}

/// #553 主锚③b（先红后绿）：被绑函数**首参是接收者类型**的外部绑定
/// （`get_impl: (b: Box, ..)`，实例占住 params[0]）——实参从 params[1..] 对位
/// （接收者位偏移 1），q=0 槽被运行期值填充必须报 E1113。
#[test]
fn test_check_external_binding_receiver_typed_fn_runtime_value_reports_e1113() {
    // Arrange - 形态③（接收者占位绑定）：`method_binding_call_key` 的接收者
    // 校验命中（params[0] = Box）⇒ 槽位对位必须跳过接收者槽，实参 y 才对到
    // q=0 名 N
    let dir = temp_dir();
    let file = create_yx_file(
        &dir,
        "external_binding_receiver_fn_const_slot.yx",
        r#"
Box: Type = { v: Int }
get_impl: (b: Box, N: Int) -> (n: N) -> Int = (n) => n
Box.get = get_impl
main: () -> Void = {
    b: Box = Box(v = 1)
    mut y: Int = 3
    r: Int = b.get(y)(y)
}
"#,
    );

    // Act
    let checked = check_result(&file);
    let codes = error_codes(&checked);

    // Assert - 翻面：check 从 0 错（修前红态实测）改为恰 1 条 E1113
    assert_eq!(
        codes.len(),
        1,
        "#553 form③b: a receiver-occupying external binding must align args past the receiver slot and reject a runtime value in the q=0 slot (E1113); got {codes:?}"
    );
    assert_eq!(
        codes[0], "E1113",
        "#553 form③b: the rejection must be E1113, got: {codes:?}"
    );
    let diag = &checked.diagnostics[0].diagnostic;
    assert!(
        diag.message.contains("'N'"),
        "#553 form③b: the diagnostic must name the bound function's q=0 name N (not the receiver b); got: {}",
        diag.message
    );
}

/// #553 主锚②（先红后绿）：类型体外部绑定（`{ get = f }`，DefaultExternal）
/// 与顶层 `Type.method = f` 走同一登记口——q=0 槽被运行期值填充必须报 E1113。
#[test]
fn test_check_type_body_external_binding_runtime_value_in_const_slot_reports_e1113() {
    // Arrange - 形态②的可构造形态：类型体 DefaultExternal 绑定；被绑函数带
    // q=0 槽（柯里化签名，首次调用只填外层位）
    let dir = temp_dir();
    let file = create_yx_file(
        &dir,
        "type_body_external_binding_const_slot.yx",
        r#"
Box: Type = { v: Int, get = get_impl }
get_impl: (N: Int) -> (n: N) -> Int = (n) => n
main: () -> Void = {
    b: Box = Box(v = 1)
    mut y: Int = 3
    r: Int = b.get(y)(y)
}
"#,
    );

    // Act
    let checked = check_result(&file);
    let codes = error_codes(&checked);

    // Assert - 翻面：check 从 0 错（修前红态实测）改为恰 1 条 E1113
    assert_eq!(
        codes.len(),
        1,
        "#553 form②: filling the q=0 slot of a type-body external binding with a runtime value must be rejected (E1113); got {codes:?}"
    );
    assert_eq!(
        codes[0], "E1113",
        "#553 form②: the rejection must be E1113, got: {codes:?}"
    );
    let diag = &checked.diagnostics[0].diagnostic;
    assert!(
        diag.message.contains("'N'"),
        "#553 form②: the diagnostic must name the bound function's q=0 name N; got: {}",
        diag.message
    );
}

/// #553 对照锚①：显式接收者方法的**编译期常量实参**（字面量 / 纯 const 表达式）
/// 修前修后同绿——证据齐备的合法调用不得被新增的接收者位对位误伤。
#[test]
fn test_check_method_with_explicit_receiver_compile_time_args_stay_accepted() {
    // Arrange - 同一形态①声明，q=0 槽填字面量与 const 表达式（修前实测 0 错）
    let dir = temp_dir();
    let file = create_yx_file(
        &dir,
        "method_self_recv_const_slot_legal_args.yx",
        r#"
Box: Type = { v: Int }
Box.get: (self: &Box, N: Int) -> (n: N) -> Int = (n) => n
main: () -> Void = {
    b: Box = Box(v = 1)
    r: Int = b.get(3)(3)
    s: Int = b.get(2+3)(5)
}
"#,
    );

    // Act
    let result = check_file(&file);

    // Assert
    let errors = result.expect("check should not panic on legal receiver-ful const args");
    assert_eq!(
        errors, 0,
        "#553: legal compile-time value arguments must stay accepted on the explicit-receiver method face; got {errors} error(s)"
    );
}

/// #553 对照锚③：外部绑定方法的编译期常量实参修前修后同绿（两条绑定路径
/// 各一形态——接收者无位与接收者占位）。
#[test]
fn test_check_external_binding_compile_time_args_stay_accepted() {
    // Arrange - 形态③两变体的合法形态（修前实测 0 错）
    let dir = temp_dir();
    let file = create_yx_file(
        &dir,
        "external_binding_const_slot_legal_args.yx",
        r#"
Box: Type = { v: Int }
get_impl: (N: Int) -> (n: N) -> Int = (n) => n
get_recv_impl: (b: Box, N: Int) -> (n: N) -> Int = (n) => n
Box.get = get_impl
Box.get2 = get_recv_impl
main: () -> Void = {
    b: Box = Box(v = 1)
    r: Int = b.get(3)(3)
    s: Int = b.get2(2+3)(5)
}
"#,
    );

    // Act
    let result = check_file(&file);

    // Assert
    let errors = result.expect("check should not panic on legal external-binding const args");
    assert_eq!(
        errors, 0,
        "#553: legal compile-time value arguments must stay accepted on both external-binding registration paths; got {errors} error(s)"
    );
}

// ── #403：`List(T)` 注解落点归一（WBS 3.20.33）───────────────────────────
//
// 条款门：SPEC syntax.md §1.6.4 与 type-system.md §4.1.1 关键规则——
// 「裸字面量与 `List(T)` 注解落可增长列表」。当前架构里可增长列表就是
// `Vec(T)`（`MonoType::is_list`），源名 `List` 进类型域统一落 `Vec` 面
// （判据单源 `MonoType::source_container_face`，源名进类型域的门共三处：
// AST 注解 From、调用点显式类型实参 `type_arg_of_expr`、native 签名串解析
// `parse_type_str_with_generics`）。
//
// 病史门：issue #403——修前 `List(T)` 注解落成幽灵面 `Generic{"List"}`（无
// 定义无值），与字面量/构造器/std.list 的 `Vec` 面必败 unify，合法程序被
// E1002 假拒绝（旧行为非成功，D59 C 类，#556/#538 同族判据）。
//
// 修前红态（CLI 探针逐字留档，2026-11-18，dev@8ff9f21d 的 check 输出）：
// - `v: List(Int) = [1, 2]` + `for x in v` →
//   error [E1002] Expected type 'List(int64)', found type 'Vec(int64)'
//     --> …:2:5（注解 vs 字面量）
//   error [E1002] Expected type 'List/Array/Range/String/Dict/Tuple（可迭代）',
//   found type 'never' --> …:3:14（迭代位；另有 E2014 'x' moved 传导次生）
// - `v: List(Int) = []` →
//   error [E1002] Expected type 'List(int64)', found type 'Vec(t37)'
// 本锚块在摘除归一补丁的同一树上复测（stash 摘 4 源文件后重建）：主锚①
// 得 ["E1002","E1002","E2014"]、主锚②得 ["E1002"]、遮挡锚 0 诊断、运行锚
// exit 1 双 E1002——与探针留档逐字一致；对照锚①②绿、对照锚③恰 1 条
// E3004（既有形态，见其注释）。
//
// 遮挡退出（用户裁决 A 案，2026-11-18）：用户自定义 `List` 记录使 `List(T)`
// 注解**不再展开为该记录**——注解统一落 `Vec` 面，记录字段访问报正式诊断
// （恰 1 条 E1053）。修前该形态 0 诊断成功（CLI 探针实测 exit=0）。

/// #403 主锚①（先红后绿）：`v: List(Int) = [1, 2]` 与对它做 `for x in v`
/// 迭代——修前双 E1002（形态见上方留档），修后 0 诊断。
#[test]
fn test_check_list_type_annotation_literal_and_for_iteration_zero_diagnostics() {
    // Arrange - issue #403 主形态：List(Int) 注解 + 字面量初始化 + for 迭代
    let dir = temp_dir();
    let file = create_yx_file(
        &dir,
        "list_annot_literal_for.yx",
        r#"
use std.io
main: () -> Void = {
    v: List(Int) = [1, 2]
    for x in v {
        io.print(x)
    }
}
main()
"#,
    );

    // Act
    let result = check_result(&file);
    let codes = error_codes(&result);

    // Assert - 翻面：check 从 2 条 E1002（修前红态实测）改为 0 诊断
    assert!(
        codes.is_empty(),
        "#403: `v: List(Int) = [1, 2]` + `for x in v` must type-check clean — the List(T) annotation lands on the growable-list face (same face as the literal), not on a ghost Generic{{\"List\"}}; got {codes:?}"
    );
}

/// #403 主锚②（先红后绿）：空字面量构造 `v: List(Int) = []`——修前
/// E1002 'List(int64)' vs 'Vec(t37)'（形态见上方留档），修后 0 诊断。
#[test]
fn test_check_list_type_annotation_empty_literal_zero_diagnostics() {
    // Arrange - 空字面量形态：无元素可推断，注解是唯一落点依据
    let dir = temp_dir();
    let file = create_yx_file(
        &dir,
        "list_annot_empty.yx",
        r#"
use std.io
main: () -> Void = {
    v: List(Int) = []
    io.print(v.length)
}
main()
"#,
    );

    // Act
    let result = check_result(&file);
    let codes = error_codes(&result);

    // Assert - 翻面：check 从 1 条 E1002（修前红态实测）改为 0 诊断
    assert!(
        codes.is_empty(),
        "#403: `v: List(Int) = []` must type-check clean — an empty literal under a List(Int) annotation lands on the growable-list face; got {codes:?}"
    );
}

/// #403 遮挡退出锚（用户裁决 A 案，2026-11-18）：用户自定义 `List` 记录定义
/// 后，`List(Int)` 注解不再展开为该记录（注解统一落 `Vec` 面）——记录字段
/// 访问报恰 1 条正式诊断（E1053）。修前该程序 0 诊断成功（CLI 探针实测）。
#[test]
fn test_check_user_list_definition_shadowing_list_annotation_reports_e1053() {
    // Arrange - 遮挡类程序逐字（用户自定义 List 记录 + 字段访问求和）
    let dir = temp_dir();
    let file = create_yx_file(
        &dir,
        "user_list_def_shadow.yx",
        r#"
use std.io

List: (T: Type) -> Type = {
    data: Vec(T),
    length: Int,
}

sum: (l: List(Int)) -> Int = {
    mut acc = 0
    mut i = 0
    while i < l.length {
        acc = acc + l.data[i]
        i = i + 1
    }
    return acc
}

main: () -> Void = {
    l: List(Int) = List([1, 2, 3], 3)
    total = sum(l)
    io.print(total)
}
main()
"#,
    );

    // Act
    let result = check_result(&file);
    let codes = error_codes(&result);
    let diag = &result.diagnostics[0].diagnostic;

    // Assert - 翻面：check 从 0 诊断（修前成功形态实测）改为恰 1 条 E1053
    assert_eq!(
        codes.len(),
        1,
        "#403 ruling A: a user-defined `List` record must make `List(Int)` annotations report exactly one formal diagnostic (annotation lands on the Vec face, the record is no longer expanded); got {codes:?}"
    );
    assert_eq!(
        codes[0], "E1053",
        "#403 ruling A: the shadow-exit diagnostic must be E1053 (field access on the non-struct growable-list face); got: {codes:?}"
    );
    assert!(
        diag.message.contains("Vec(int64)"),
        "#403 ruling A: the diagnostic must name the landing face Vec(int64); got: {}",
        diag.message
    );
}

/// #403 对照锚①：裸字面量 `for x in [1, 2]` 迭代——修前修后同绿
/// （落点不受注解面影响；修前探针实测无 E1002）。
#[test]
fn test_check_bare_list_literal_for_iteration_stays_accepted() {
    // Arrange - 无注解的裸字面量迭代（#403 的对照面）
    let dir = temp_dir();
    let file = create_yx_file(
        &dir,
        "bare_literal_for.yx",
        r#"
use std.io
main: () -> Void = {
    for x in [1, 2] {
        io.print(x)
    }
}
main()
"#,
    );

    // Act
    let result = check_result(&file);
    let codes = error_codes(&result);

    // Assert - 对照：修前修后逐字不变（0 诊断）
    assert!(
        codes.is_empty(),
        "#403 control: bare literal `for x in [1, 2]` must stay accepted unchanged by the landing-face unification; got {codes:?}"
    );
}

/// #403 对照锚②：`Vec(Int)` 注解 + for 迭代——修前修后同绿
///（修前探针实测 exit=0，CLI 全绿形态逐字）。
#[test]
fn test_check_vec_type_annotation_for_iteration_stays_accepted() {
    // Arrange - Vec(Int) 注解形态（#403 的对照面，修前已绿的同一形态）
    let dir = temp_dir();
    let file = create_yx_file(
        &dir,
        "vec_annot_for.yx",
        r#"
use std.io
main: () -> Void = {
    v: Vec(Int) = [1, 2]
    for x in v {
        io.print(x)
    }
}
main()
"#,
    );

    // Act
    let result = check_result(&file);
    let codes = error_codes(&result);

    // Assert - 对照：修前修后逐字不变（0 诊断）
    assert!(
        codes.is_empty(),
        "#403 control: `v: Vec(Int) = [1, 2]` + `for x in v` must stay accepted unchanged; got {codes:?}"
    );
}

/// #403 对照锚③：`Array(Int, 3)` 注解 + for 迭代——定长数组面不受落点归一
/// 影响。`Array` 作 for 迭代源在 ir_gen 层不支持（E3004 正式诊断）——这是
/// **修前既有形态**（修前探针实测逐字：`error [E3004] Unsupported iterator
/// type: Array(int64, int64::3)`，恰 1 条；此前 CLI 探针只报 E1001 print_int
/// 是变量解析错误挡住了后续阶段），修后逐字不变。
#[test]
fn test_check_array_type_annotation_for_iteration_reports_existing_e3004() {
    // Arrange - Array(Int, 3) 注解形态（#403 的对照面）
    let dir = temp_dir();
    let file = create_yx_file(
        &dir,
        "array_annot_for.yx",
        r#"
use std.io
main: () -> Void = {
    v: Array(Int, 3) = [1, 2, 3]
    for x in v {
        io.print(x)
    }
}
main()
"#,
    );

    // Act
    let result = check_result(&file);
    let codes = error_codes(&result);
    let diag = &result.diagnostics[0].diagnostic;

    // Assert - 对照：修前修后逐字不变（恰 1 条 E3004，点名 Array 面）
    assert_eq!(
        codes.len(),
        1,
        "#403 control: `v: Array(Int, 3) = [1, 2, 3]` + `for x in v` must stay unchanged by the landing-face unification (exactly the pre-existing E3004 — fixed-length arrays are not an ir_gen iterator source); got {codes:?}"
    );
    assert_eq!(
        codes[0], "E3004",
        "#403 control: the sole diagnostic must remain the pre-existing E3004, got: {codes:?}"
    );
    assert!(
        diag.message.contains("Array(int64, int64::3)"),
        "#403 control: E3004 must name the fixed-length face Array(int64, int64::3); got: {}",
        diag.message
    );
}

/// #403 对照锚④：user-List 遮挡下的容器直造形态 `Vec(List(Int))(…)`——
/// 内层 `List(Int)` 作显式类型实参走 lower 侧 `type_arg_of_expr_ir`（#403
/// 单源判据的第四门）。探针实测（2026-11-19，主仓工作树）：用户自定义
/// `List` 和类型 + 直造调用编译通过、运行打印 2——修前修后行为不变（L3
/// 视图不 unify），本锚钉住的是**可接受性**：表示归一（无论 L2 还是 L3 门）
/// 不得把这一原先成功形态变成任何诊断。
#[test]
fn test_check_user_list_shadowed_type_arg_direct_construction_stays_accepted() {
    // Arrange - user-List 遮挡 + Vec(List(Int)) 直造（#403 单源判据第四门的可达面）
    let dir = temp_dir();
    let file = create_yx_file(
        &dir,
        "user_list_direct_constr.yx",
        r#"
use std.io

List: (T: Type) -> Type = {
    item: (T) -> List(T),
    empty: () -> List(T),
}

main: () -> Void = {
    a = List(Int).item(1)
    b = List(Int).empty()
    n = Vec(List(Int))(a, b).length
    io.print(n)
}
main()
"#,
    );

    // Act
    let result = check_result(&file);
    let codes = error_codes(&result);

    // Assert - 对照：修前修后逐字不变（0 诊断）
    assert!(
        codes.is_empty(),
        "#403 control: user-List shadowed `Vec(List(Int))(...)` direct construction must stay accepted (the L3 view normalization in type_arg_of_expr_ir is view-only and must not reject); got {codes:?}"
    );
}

// ==================== 7b.4 槽预算判据（裁决 ②）：5 形态 × 成功面 / 拒绝面 ====================
//
// 规范来源：09a §P7 7b.4 契约 §四（预算判据按用户裁决 ② 改写）+ RFC-013 E3014。
// 7a 的 5 个 >255 探针（函数 / 方法 / lambda / 匿名绑定 / curry 最内层）在
// 7b.4 后按新语义拆成两面：
// - 成功面：300 项扁平链——**值总数 >255，但活跃值是常数级** ⇒ 帧槽 ≤255 ⇒
//   零诊断（7a 的「由 E3014 拒绝」在这面不再成立，D59 C 类假拒绝直修）；
// - 拒绝面：300 个具名常驻绑定——**具名基址 >255，真装不下** ⇒ 恰 1 条 E3014
//   且带函数体 span（闸仍有牙）。

/// `count` 个具名常驻绑定（每条都在语句边界之后，具名槽不复用）。
fn named_bindings(
    count: usize,
    indent: &str,
) -> String {
    (0..count)
        .map(|i| format!("{indent}v{i} = {i}\n"))
        .collect()
}

/// 5 个函数体形态的成功面源码：`chain` 放在该形态的**值模式函数体**里。
fn slot_budget_success_forms(chain: &str) -> Vec<(&'static str, String)> {
    vec![
        (
            "函数体",
            format!("id: (Int) -> Int = (x) => x\nmain = () => {{\n    y = id({chain})\n}}\n"),
        ),
        (
            "方法体",
            format!(
                "T: Type = {{ x: Int }}\nT.m: (self: &T) -> Int = {{\n    y = {chain}\n    y\n}}\n"
            ),
        ),
        (
            "lambda 体",
            format!("main = () => {{\n    g = (x: Int) => {chain}\n}}\n"),
        ),
        (
            "curry 最内层",
            format!("f: (a: Int) -> (b: Int) -> Int = (a) => (b) => {chain}\n"),
        ),
        (
            "匿名绑定体",
            format!(
                "P: Type = {{\n    x: Int,\n    cb: ((a: Int) -> Int)[0] = a => {chain},\n}}\n"
            ),
        ),
    ]
}

/// 5 个函数体形态的拒绝面源码：300 个具名常驻绑定把具名基址推过 255。
fn slot_budget_reject_forms() -> Vec<(&'static str, String)> {
    vec![
        (
            "函数体",
            format!("main = () => {{\n{}}}\n", named_bindings(300, "    ")),
        ),
        (
            "方法体",
            format!(
                "T: Type = {{ x: Int }}\nT.m: (self: &T) -> Void = {{\n{}}}\n",
                named_bindings(300, "    ")
            ),
        ),
        (
            "lambda 体",
            format!(
                "main = () => {{\n    g = () => {{\n{}    }}\n}}\n",
                named_bindings(300, "        ")
            ),
        ),
        (
            "curry 最内层",
            format!(
                "f: (a: Int) -> (b: Int) -> Void = (a) => (b) => {{\n{}}}\n",
                named_bindings(300, "    ")
            ),
        ),
        (
            "匿名绑定体",
            format!(
                "P: Type = {{\n    x: Int,\n    cb: ((a: Int) -> Void)[0] = a => {{\n{}    }},\n}}\n",
                named_bindings(300, "        ")
            ),
        ),
    ]
}

/// 成功面：5 形态的扁平深链一律零诊断（帧槽按活跃区间复用后装得下）。
#[test]
fn test_slot_budget_deep_chain_succeeds_in_five_body_forms() {
    // Arrange
    let chain = "1 + ".repeat(300) + "1";
    let forms = slot_budget_success_forms(&chain);

    // Act & Assert
    for (form, source) in forms {
        let dir = temp_dir();
        let file = create_yx_file(&dir, "success_form.yx", &source);
        let result = check_result(&file);
        let codes = error_codes(&result);
        assert!(
            codes.is_empty(),
            "{form}: 300 项链的活跃值是常数级、帧槽装得下，必须零诊断，实际: {codes:?}"
        );
    }
}

/// 拒绝面：5 形态的 300 个具名绑定一律报恰 1 条 E3014，且带函数体 span。
#[test]
fn test_slot_budget_named_base_overflow_reports_e3014_with_span_in_five_body_forms() {
    // Arrange
    let forms = slot_budget_reject_forms();

    // Act & Assert
    for (form, source) in forms {
        let dir = temp_dir();
        let file = create_yx_file(&dir, "reject_form.yx", &source);
        let result = check_result(&file);
        let overflows: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|entry| entry.diagnostic.code == "E3014")
            .collect();
        assert_eq!(
            overflows.len(),
            1,
            "{form}: 具名基址 300 > 255 必须恰报 1 条 E3014，实际 {} 条",
            overflows.len()
        );
        assert!(
            overflows[0].diagnostic.span.is_some(),
            "{form}: E3014 必须带函数体 span（#271/#324 的发射点要求）"
        );
    }
}
