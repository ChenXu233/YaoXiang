//! WBS 3.4.6 判据 C2：警告面归一——**多文件与单文件两条形态**对同一份源码的
//! W 码集合逐条一致。
//!
//! 修复前缺口（红态来源）：`MULTI_FILE_STAGES` 无 `RoleClassification` /
//! `DeadCodeAnalysis` 两阶段，且 `typecheck_orchestrated` 的 FailFast 分支
//! 从不读 `result.warnings` ⇒ 多文件 `run` 恒零警告（02-stage-contract
//! §11 处阶段覆盖不一致表第 2/4/5 行；决策 B1 已于 WBS 3.4.6 撤销）。
//!
//! 判据口径（07 §三层判据的诊断列表归一规则）：W 码集合按 `(码, 行)` 排序后
//! 逐条比对——**消息文本不参与比对**；文件身份两形态构造性不同（单文件是内存
//! 源码 `test.yx`，多文件是落盘路径），故文件维度不进判据。
//!
//! 两形态的警告 sink 现状（本模块的存在理由就是让它们给出同一集合）：
//! - `SingleFile`（pipeline 通道）：`CompilationResult.warnings`；
//! - `MultiFile`（编排通道）：逐文件收集 `DriverOutcome.check_diagnostics`。

use std::path::{Path, PathBuf};

use crate::driver::{Driver, DriverError, DriverOutcome, Program, ProgramKind, Unit};
use crate::frontend::config::CompileConfig;

/// 合法 manifest（`[package]` 可解析 ⇒ 角色上下文可用）
const MANIFEST: &str = "[package]\nname = \"warn-parity\"\nversion = \"0.1.0\"\n";

/// 以 SingleFile 形态跑 Driver（`lib::run_file` / `Compiler::compile` 的库路径形态）
fn run_single(source: &str) -> Result<DriverOutcome, DriverError> {
    let program = Program::new(
        ProgramKind::SingleFile,
        vec![Unit::new("test", "test.yx", source)],
        CompileConfig::default(),
    );
    Driver::new().run(program)
}

/// 以 MultiFile 形态跑 Driver（`compile_project`，即 `yaoxiang run` 的形态）
fn run_multi(entry: &Path) -> Result<DriverOutcome, DriverError> {
    let program = Program::new(
        ProgramKind::MultiFile,
        vec![Unit::new("main", entry, "")],
        CompileConfig::default(),
    );
    Driver::new().run(program)
}

/// 落一个多文件夹具（入口约定 `main.yx`）
fn write_project(files: &[(&str, &str)]) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("create tempdir");
    std::fs::write(dir.path().join("yaoxiang.toml"), MANIFEST).expect("write manifest");
    let mut entry = PathBuf::new();
    for (name, content) in files {
        let path = dir.path().join(name);
        std::fs::write(&path, content).expect("write project file");
        if *name == "main.yx" {
            entry = path;
        }
    }
    assert!(entry.is_file(), "夹具必须含入口 main.yx");
    (dir, entry)
}

/// 取一条诊断的 W 码判定：只有 `W` 前缀的 Warning 码进判据面
/// （E 级与 W1090 义务账本码不属本判据——后者是阶段表尾部的后置条件）。
fn is_warning_code(code: &str) -> bool {
    code.starts_with('W') && code != crate::driver::OBLIGATION_UNDRAINED_CODE
}

/// 从一个诊断集合里取 `(码, 行)` 列表并排序（行号 0 = 无位置）。
fn w_codes<'a>(
    diagnostics: impl Iterator<Item = &'a crate::util::diagnostic::Diagnostic>
) -> Vec<(String, usize)> {
    let mut out: Vec<(String, usize)> = diagnostics
        .filter(|d| is_warning_code(&d.code))
        .map(|d| (d.code.clone(), d.span.map(|s| s.start.line).unwrap_or(0)))
        .collect();
    out.sort();
    out
}

/// 单文件形态的警告面：`CompilationResult.warnings`
fn single_w_codes(outcome: &DriverOutcome) -> Vec<(String, usize)> {
    w_codes(outcome.result.warnings.iter())
}

/// 多文件形态的警告面：逐文件收集通道（`check_diagnostics`）的全部 W 码
fn multi_w_codes(outcome: &DriverOutcome) -> Vec<(String, usize)> {
    w_codes(
        outcome
            .check_diagnostics
            .iter()
            .flat_map(|(_, ds)| ds.iter()),
    )
}

/// C2 主判据：同一份源码（单文件可表达的形状）在两条形态下的 W 码集合逐条一致。
///
/// 夹具同时覆盖三个码：W1001（未使用函数）/ W1002（未使用类型）/
/// W1003（未使用导入）——正是 WBS 3.4.6 表行点名的三个。
#[test]
fn test_multifile_warning_set_matches_single_file() {
    // Arrange：同一份源码——未使用导入 + 未使用函数 + 未使用类型
    let source = "use std.io.{println}\n\
                  \n\
                  unused_fn: (x: Int) -> Int = (x) => x + 1\n\
                  \n\
                  UnusedT: Type = { a: Int }\n\
                  \n\
                  main: () -> Void = {}\n";
    let (_dir, entry) = write_project(&[("main.yx", source)]);

    // Act
    let single = run_single(source).expect("SingleFile 形态必须返回产物");
    let multi = run_multi(&entry).expect("MultiFile 形态必须返回产物");

    // Assert：两条形态的 W 码集合逐条一致（码 + 行，排序后比对）
    let single_codes = single_w_codes(&single);
    let multi_codes = multi_w_codes(&multi);
    assert!(
        !single_codes.is_empty(),
        "单文件形态必须产出 W 码（否则本判据空转）：{single_codes:?}"
    );
    assert_eq!(
        multi_codes, single_codes,
        "多文件形态的 W 码集合必须与单文件逐条一致（WBS 3.4.6 判据 C2）；\
         多文件 {multi_codes:?} vs 单文件 {single_codes:?}"
    );
}

/// 对照组（未修分支）：多文件夹具里**被真实消费**的定义不得报 W 码——
/// 防修复把警告面整体放大（只证"新行为对"不足以放行）。
#[test]
fn test_multifile_consumed_definition_stays_silent() {
    // Arrange：lib.helper 被 main 真实调用 ⇒ 不报 W1001；main 本身是入口 ⇒ 不报
    let lib = "helper: () -> Void = {}\n";
    let main = "use lib.{helper}\n\nmain: () -> Void = {\n    helper()\n}\n";
    let (_dir, entry) = write_project(&[("lib.yx", lib), ("main.yx", main)]);

    // Act
    let multi = run_multi(&entry).expect("MultiFile 形态必须返回产物");

    // Assert：零 W 码（跨文件消费被引用池承认）
    let codes = multi_w_codes(&multi);
    assert!(
        codes.is_empty(),
        "被跨文件消费的定义不得报 W 码（引用池豁免必须生效）：{codes:?}"
    );
}

/// 逐文件归因：警告必须落在**定义所在文件**上，不能串到入口文件。
#[test]
fn test_multifile_warning_is_attributed_to_defining_file() {
    // Arrange：lib.yx 有一个被消费的 `used`（第 2 行）与一个死代码
    // `dead_fn`（第 4 行）；main.yx 真实调用 `used()` ⇒ 自身零警告。
    // main 必须 `use` lib——Discovery 沿 use 图追踪（RFC-029 #247），
    // 未被 use 的文件不进编译集；导入项也必须真实使用，否则另报 W1003。
    let lib =
        "// lib.yx —— 被导入模块\nused: () -> Int = () => 1\n\ndead_fn: () -> Int = () => 2\n";
    let main = "use lib.{used}\n\nmain: () -> Void = {\n    x = used()\n}\n";
    let (_dir, entry) = write_project(&[("lib.yx", lib), ("main.yx", main)]);

    // Act
    let multi = run_multi(&entry).expect("MultiFile 形态必须返回产物");

    // Assert：W1001 挂在 lib.yx 的条目上，main.yx 条目里没有它
    let lib_codes: Vec<(String, usize)> = multi
        .check_diagnostics
        .iter()
        .find(|(p, _)| p.ends_with("lib.yx"))
        .map(|(_, ds)| w_codes(ds.iter()))
        .unwrap_or_else(|| panic!("lib.yx 必须进逐文件收集通道"));
    let main_codes: Vec<(String, usize)> = multi
        .check_diagnostics
        .iter()
        .find(|(p, _)| p.ends_with("main.yx"))
        .map(|(_, ds)| w_codes(ds.iter()))
        .unwrap_or_else(|| panic!("main.yx 必须进逐文件收集通道"));
    assert_eq!(
        lib_codes,
        vec![("W1001".to_string(), 4)],
        "死代码警告必须归因到定义所在文件 lib.yx 第 4 行"
    );
    assert!(
        main_codes.is_empty(),
        "入口文件干净时不得串入他文件的警告：{main_codes:?}"
    );
}
