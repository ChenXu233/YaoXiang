//! RFC-039 P2 漏洞专门判据：证明义务静默丢弃（先红后绿）
//!
//! 规范来源:
//! - docs/src/dev/architecture/07-equivalence-oracle.md §修复正确性漏洞的专门判据
//! - docs/src/dev/architecture/02-stage-contract.md §正确性漏洞：证明义务被静默丢弃
//!   （完整证据链）/ RFC-027 §4/§9「Unproven → 编译错误，无降级、无 silent pass」
//!
//! 漏洞形状：checker 的三处同构分支（checker.rs:5179/5318/5448）对
//! `ProofResult::Unproven { proof_calls, .. }` 只做 `ctx.proof_calls.extend(calls)`
//! 而不发诊断，设计上假定「一定有人会来读 proof_calls」。全仓唯一读取点在
//! pipeline.rs:187（单文件路径）；orchestrator 的 compile_project / check_project /
//! check_source_in_project / compile_embedded_module 四入口均不经过该层——
//! 多文件 `run` / `check` / LSP 路径下证明函数从未被调用，约束静默通过。
//!
//! 本文件两个判据**必须先为红**（WBS 2.4.1/2.4.3 验收）：
//! P3（消费端修复，02 §S3）落地前运行本文件应见失败；P3 落地后转绿，
//! 且「故意撤掉修复必须重新变红」。
//!
//! 红态证据（2026-10-06 实测，`cargo test --test integration proof_obligations`）：
//! - test_multifile_proof_obligation_not_dropped：单文件 ["E4018"] vs
//!   compile_project [] → FAILED（符合预期）
//! - test_no_silent_pass_on_unproven：返回位触发源在 compile_project 下静默 →
//!   FAILED（符合预期）
//!
//! 两个测试带 #[ignore] 常驻（2.4.5 红骨架先例）：默认套件保持全绿，
//! 红态经 `cargo test --test integration proof_obligations -- --ignored` 复现；
//! P3 修复落地时**移除此属性**即为转绿（WBS P3 验收）。
//!
//! 与 tests/yaoxiang-multifile/ 的分工：语料层是 P4 的行为差分基线（全绿夹具），
//! 本文件是漏洞专门判据（库级、红→绿），两者不混（D48）。

use std::path::{Path, PathBuf};

use yaoxiang::frontend::module::orchestrator::{check_project, compile_project, OrchestratorError};
use yaoxiang::frontend::Pipeline;
use yaoxiang::util::diagnostic::{Diagnostic, Severity};

/// 证明义务触发源公共前缀：二元谓词 `SumUpTo(n, s)` ⇔ `s == n * 2`。
///
/// 约束形态是 `ConstExpr::Call`——证明管道四级分派的前三级（求值/假设栈/SMT）
/// 都无法处理用户定义的类型函数调用，必然落到第四级：
/// `Unproven + proof_calls=[SumUpTo(3, 7)]`（rfc027_phase25_proof_fn.rs 钉死）。
const SUM_UP_TO: &str = "SumUpTo: (n: Int, s: Int) -> Type = { s == n * 2 }\n";

/// 触发源 1：返回位义务（命中 checker.rs:5448 的 extend 分支）。
/// 实参 [Int(3), Int(7)] 由字面量 3 与 return 代入的 7 构成，全可取值；
/// 证明函数执行 7 == 3*2 为 false ⇒ 单文件路径报 E4018（rfc027_return_refinement.rs:337 钉死）。
const RETURN_POSITION: &str =
    "f: () -> (r: SumUpTo(3, r)) = {\n    return 7\n}\n\nmain: () -> Void = {}\n";

/// 触发源 2：调用点实参义务（命中 checker.rs:5318 的 extend 分支）。
/// 调用须写成语句层赋值右侧——嵌套在 assert 里的调用不进检查（#395，
/// tests/yaoxiang/02-type-system/call_arg_evidence.yx 的注释钉死）。
const CALL_ARG: &str =
    "h: (s: SumUpTo(3, s)) -> Int = { s }\n\nmain: () -> Void = {\n    z: Int = h(7)\n}\n";

/// 触发源 3：绑定位注解义务（`y: SumUpTo(3, y) = 7` 的证明调用形态）。
const BINDING_ANNOTATION: &str = "main: () -> Void = {\n    y: SumUpTo(3, y) = 7\n}\n";

/// 单文件路径（pipeline.rs，proof_calls 唯一消费端所在）的错误诊断码集（排序）。
fn single_file_error_codes(source: &str) -> Vec<String> {
    let mut pipeline = Pipeline::default();
    let result = pipeline.run("proof_obligation.yx", source);
    let mut codes: Vec<String> = result
        .errors
        .iter()
        .filter_map(|e| e.diagnostic().map(|d| d.code))
        .collect();
    codes.sort();
    codes
}

/// 把源码落成带 manifest 的最小项目（Bin 角色），返回 (tempdir 守卫, 入口路径)。
fn write_project(source: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("create tempdir");
    std::fs::write(
        dir.path().join("yaoxiang.toml"),
        "[package]\nname = \"proof-obligation-probe\"\nversion = \"0.1.0\"\n",
    )
    .expect("write yaoxiang.toml");
    let entry = dir.path().join("main.yx");
    std::fs::write(&entry, source).expect("write main.yx");
    (dir, entry)
}

/// 从 OrchestratorError 提取错误诊断码（排序）。
fn orchestrator_error_codes(e: &OrchestratorError) -> Vec<String> {
    let diags: &[Diagnostic] = match e {
        OrchestratorError::TypeCheck { diagnostics, .. } => diagnostics,
        OrchestratorError::Compile { diagnostics, .. } => diagnostics,
        // 无结构化诊断的变体不是本判据关注的形态——原样露出便于定位
        other => return vec![format!("ORCHESTRATOR_ERROR:{other}")],
    };
    let mut codes: Vec<String> = diags
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .map(|d| d.code.clone())
        .collect();
    codes.sort();
    codes
}

/// 多文件 run 路径（compile_project，路径 3a）的错误诊断码集；编译通过为空集。
fn compile_project_error_codes(entry: &Path) -> Vec<String> {
    match compile_project(entry) {
        Ok(_) => Vec::new(),
        Err(e) => orchestrator_error_codes(&e),
    }
}

/// 多文件 check 路径（check_project，路径 3b）的错误诊断码集（收集全部模式）。
fn check_project_error_codes(entry: &Path) -> Vec<String> {
    match check_project(entry) {
        Ok(files) => {
            let mut codes: Vec<String> = files
                .iter()
                .flat_map(|(_, diags)| diags)
                .filter(|d| d.severity == Severity::Error)
                .map(|d| d.code.clone())
                .collect();
            codes.sort();
            codes
        }
        Err(e) => orchestrator_error_codes(&e),
    }
}

/// WBS 2.4.1：同一份含证明义务的源码，单文件与多文件两条路径的**错误诊断码集
/// 必须相等**（C2 判据）。
///
/// 规范化说明（对 07 §三层判据 (code, file, line) 比对表的有意偏离）：
/// - file：两条路径的文件身份构造性不同（内存 source_name vs 临时项目落盘路径），
///   参与比对必假红，剔除；
/// - line：本判据钉「义务不丢」这一行为；span 质量由 #324 体系与 P4 阶段统一保证；
/// - 只比 error severity：W1001/W1002 警告在两条路径产出不对称（02 表第 2/4/5 行），
///   其统一属 P4 阶段契约范畴（4.1.4 判据）——混入会让 P3 止血修复无法单独转绿
///   本测试（止血通道裁决：漏洞判据随 P3 转绿）。
#[test]
#[ignore = "red-by-design（WBS 2.4.1）：多文件 proof_calls 无消费端；P3（02 §S3）修复后移除此属性"]
fn test_multifile_proof_obligation_not_dropped() {
    // Arrange
    let source = format!("{SUM_UP_TO}\n{RETURN_POSITION}");
    let single = single_file_error_codes(&source);
    assert_eq!(
        single,
        vec!["E4018".to_string()],
        "单文件路径基线：证明函数 SumUpTo(3, 7) 返回 false 应报 E4018；实际: {single:?}"
    );

    let (_guard, entry) = write_project(&source);

    // Act & Assert：多文件 run 路径
    let via_compile = compile_project_error_codes(&entry);
    assert_eq!(
        single, via_compile,
        "compile_project（多文件 run）丢失证明义务诊断：单文件 {single:?} vs 多文件 {via_compile:?}"
    );

    // Act & Assert：多文件 check 路径
    let via_check = check_project_error_codes(&entry);
    assert_eq!(
        single, via_check,
        "check_project（多文件 check）丢失证明义务诊断：单文件 {single:?} vs 多文件 {via_check:?}"
    );
}

/// WBS 2.4.3：`ProofResult::Unproven` 在任一编译路径下都必须产生诊断——
/// 拦住 checker.rs:5179/5318/5448 三处「extend proof_calls 但不发诊断」分支
/// 在无消费端路径上造成的静默通过。
///
/// 三种触发源覆盖三类义务位置；每个触发源 × 每条路径都断言「至少一条 error 诊断」。
/// 单文件 pipeline 断言同时是回归钉（消费端被误删时立即变红）。
///
/// 注：checker.rs:1313（ownership 层 Unproven 空 match 臂，3.2.1 的修复点）同属
/// 本判据名，但 layers/ownership.rs 当前不构造 Unproven（全仓 grep 零命中），
/// 该臂暂不可从源码触发；3.2.1 修复时若确认可触发形态应在此补用例。
#[test]
#[ignore = "red-by-design（WBS 2.4.3）：三处 extend 分支在无消费端路径静默；P3 修复后移除此属性"]
fn test_no_silent_pass_on_unproven() {
    for (name, trigger) in [
        ("返回位", RETURN_POSITION),
        ("调用点实参", CALL_ARG),
        ("绑定位注解", BINDING_ANNOTATION),
    ] {
        // Arrange
        let source = format!("{SUM_UP_TO}\n{trigger}");
        let single = single_file_error_codes(&source);
        assert!(
            !single.is_empty(),
            "{name}触发源在单文件 pipeline 下应产诊断（消费端基线）；实际为空"
        );

        let (_guard, entry) = write_project(&source);

        // Act & Assert：任一模式不得静默
        let via_compile = compile_project_error_codes(&entry);
        assert!(
            !via_compile.is_empty(),
            "{name}触发源在 compile_project 下 Unproven 被静默丢弃（单文件基线: {single:?}）"
        );
        let via_check = check_project_error_codes(&entry);
        assert!(
            !via_check.is_empty(),
            "{name}触发源在 check_project 下 Unproven 被静默丢弃（单文件基线: {single:?}）"
        );
    }
}
