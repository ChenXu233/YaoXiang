//! 诊断文案与 locales 同源——issue #509 残留①②③的渲染级锚（CLI 子进程级）
//!
//! 判据来源：issue #509（WBS 3.21.5，#469 后续残留）——诊断 message/参数位
//! 文案在发射点不硬编成品串，文案住 `locales/*.json`、随 `error_lang()` 渲染；
//! zh 形态必须与迁移前硬编文案**逐字节一致**（只换载体不换文面）。
//!
//! 覆盖三个迁移面：
//! - 残留②：E4018 proof_false 形态的 constraint 参数位句式
//!   （`E4018.proof_false.constraint`，proof_execution.rs 发射）；
//! - 残留③：E1002 二元算术的 expected 参数接受集文（两个遍历器同源常量
//!   `E1002_ARITH_EXPECTED`）——语句检查器臂（无后缀）与表达式推断器臂
//!   （带「或实现对应算术接口」后缀）两条路由各自断言，后缀差异即路由证据。
//!
//! 语言钉法：CLI 子进程 + `YAOXIANG_LANG` 环境变量（error_lang 解析的第一优先
//! 级）——进程级隔离，无进程内 env 竞争（命令形态先例：check.rs 的 #537 锚）。
//! 探针实测（2026-11-20，dev 工作树）：zh/en 逐字形态见各断言。

use std::path::{Path, PathBuf};
use std::process::Command;

/// 证明义务触发源：返回位义务，`SumUpTo(3, 7)` 的体 `s == n * 2` 判 false
/// ⇒ 证明执行器 proof_false 臂（proof_obligations.rs 的 RETURN_POSITION 同源）。
const PROOF_FALSE_SOURCE: &str = concat!(
    "SumUpTo: (n: Int, s: Int) -> Type = { s == n * 2 }\n",
    "f: () -> (r: SumUpTo(3, r)) = {\n",
    "    return 7\n",
    "}\n",
    "\n",
    "main: () -> Void = {}\n",
);

/// E1002 语句检查器臂路由：赋值初始化式里的二元运算由 StatementChecker 直接
/// 处理（expected = 接受集基文，无接口后缀）。语料同源：
/// tests/yaoxiang/06-compile-errors/types/binop_type_err.yx。
const STMT_PATH_SOURCE: &str = "main: () -> Void = {\n    x = 1 + \"a\"\n}\n";

/// E1002 表达式推断器臂路由：spawn 体经 infer_block 走 infer_stmt（#313），
/// 二元运算落入 `ExpressionInferrer::infer_binary`（expected = 基文 + 既有
/// 「或实现对应算术接口」后缀）。语料形态同源：
/// tests/yaoxiang/06-compile-errors/control-flow/spawn_body_type_err.yx。
const EXPR_PATH_SOURCE: &str =
    "main: () -> Void = {\n    spawn {\n        x = 1 + \"a\"\n    }\n}\n";

/// 把源码落成临时 .yx 文件，返回文件路径。
fn write_yx(
    dir: &tempfile::TempDir,
    name: &str,
    source: &str,
) -> PathBuf {
    let file = dir.path().join(name);
    std::fs::write(&file, source).expect("write probe .yx");
    file
}

/// 以指定语言跑 CLI `check`，返回合并输出（stdout + stderr）。
fn check_output(
    file: &Path,
    lang: &str,
) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_yaoxiang-rs"))
        .arg("check")
        .arg(file)
        .env("YAOXIANG_LANG", lang)
        .output()
        .expect("failed to run yaoxiang-rs check");
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// 残留②（zh）：E4018 proof_false 的 constraint 参数位与反例注记必须来自
/// locales 键渲染，zh 形态与迁移前 `format!` 硬编文案逐字节一致。
#[test]
fn test_proof_false_diagnostic_renders_zh_byte_identical_to_pre_migration() {
    // Arrange
    let dir = tempfile::tempdir().expect("create tempdir");
    let file = write_yx(&dir, "proof_false_zh.yx", PROOF_FALSE_SOURCE);

    // Act
    let text = check_output(&file, "zh");

    // Assert
    assert!(
        text.contains("E4018"),
        "证明义务判伪必须报 E4018；got:\n{text}"
    );
    assert!(
        text.contains("精化谓词违反：证明函数 'SumUpTo' 返回 false，约束不满足（参数: [Int(3), Int(7)]）；检查约束条件或修改传入值"),
        "zh constraint 参数位必须来自 E4018.proof_false.constraint 键渲染且与迁移前硬编文案逐字节一致（issue #509 残留②）；got:\n{text}"
    );
    assert!(
        text.contains("（证明函数返回 false，无具体反例）"),
        "zh 反例注记必须来自 E4018.proof_false.note 键渲染；got:\n{text}"
    );
}

/// 残留②（en）：同一发射面在 en 下渲染 en 真文案——迁移前 en 用户看到的
/// 是硬编中文，迁移后按语言渲染（本次迁移的目的本身）。
#[test]
fn test_proof_false_diagnostic_renders_en_from_locales() {
    // Arrange
    let dir = tempfile::tempdir().expect("create tempdir");
    let file = write_yx(&dir, "proof_false_en.yx", PROOF_FALSE_SOURCE);

    // Act
    let text = check_output(&file, "en");

    // Assert
    assert!(
        text.contains(
            "Refinement predicate violated: Proof function 'SumUpTo' returned false: constraint not satisfied (args: [Int(3), Int(7)]); check the constraint or change the arguments"
        ),
        "en constraint 参数位必须来自 locales 键渲染（issue #509 残留②）；got:\n{text}"
    );
    assert!(
        text.contains("(Proof function returned false; no concrete counterexample.)"),
        "en 反例注记必须来自 E4018.proof_false.note 键渲染；got:\n{text}"
    );
}

/// 残留③（语句检查器臂，zh+en）：expected 参数来自共享常量基文，**无**接口
/// 后缀——无后缀本身即「本程序走 StatementChecker 臂」的路由证据。
#[test]
fn test_e1002_expected_statement_path_renders_shared_text_without_interface_suffix() {
    // Arrange
    let dir = tempfile::tempdir().expect("create tempdir");
    let file = write_yx(&dir, "binop_stmt.yx", STMT_PATH_SOURCE);

    // Act
    let zh = check_output(&file, "zh");
    let en = check_output(&file, "en");

    // Assert：zh 模板 + 共享基文逐字；en 模板 + 同一基文（参数值语言无关）
    assert!(
        zh.contains("期望类型 'Int/Float/String/List（两侧同型）'，实际类型"),
        "zh E1002 模板必须与共享常量基文渲染（issue #509 残留③）；got:\n{zh}"
    );
    assert!(
        !zh.contains("或实现对应算术接口"),
        "语句检查器臂不得带接口后缀（带后缀 = 程序误入表达式推断器臂，路由漂移）；got:\n{zh}"
    );
    assert!(
        en.contains("Expected type 'Int/Float/String/List（两侧同型）'"),
        "en E1002 模板必须与共享常量基文渲染；got:\n{en}"
    );
}

/// 残留③（表达式推断器臂，zh+en）：spawn 体的二元运算走
/// `ExpressionInferrer::infer_binary`，expected = 基文 + 既有后缀（文面逐字
/// 保持迁移前形态）。
#[test]
fn test_e1002_expected_expression_path_renders_shared_text_with_interface_suffix() {
    // Arrange
    let dir = tempfile::tempdir().expect("create tempdir");
    let file = write_yx(&dir, "binop_spawn.yx", EXPR_PATH_SOURCE);

    // Act
    let zh = check_output(&file, "zh");
    let en = check_output(&file, "en");

    // Assert：基文 + 既有后缀（两臂由同一常量提供基文，后缀为表达式臂既有文面）
    assert!(
        zh.contains("期望类型 'Int/Float/String/List（两侧同型）或实现对应算术接口'"),
        "zh 表达式推断器臂必须渲染基文+既有后缀（issue #509 残留③）；got:\n{zh}"
    );
    assert!(
        en.contains("Expected type 'Int/Float/String/List（两侧同型）或实现对应算术接口'"),
        "en 表达式推断器臂必须渲染基文+既有后缀；got:\n{en}"
    );
}
