//! WBS 3.13.1 / issue #451（RFC-039 D59 A 类）：方法绑定无签名标注不得 panic
//!
//! 规范来源:
//! - issue [#451](https://github.com/ChenXu233/YaoXiang/issues/451)（现象 / 最小复现 /
//!   根因 / 判据要求：「不 panic 且产出诊断」，**不得**用 `#[should_panic]` 把崩溃钉成期望行为）
//! - docs/src/dev/architecture/09-execution-wbs.md 3.13 行（`ir_gen.rs` 的 MethodBind 分支）
//!
//! # 改前实测（红态，2026-10-14）
//!
//! 夹具 `Temp.pick2 = Temp.pick[0]`（方法组当值再绑定、且无签名标注）走真实链路
//! （lex→parse→typecheck→ir_gen）时：
//!
//! ```text
//! thread 'main' panicked at src\middle\core\ir_gen.rs:1678:50:
//! called `Option::unwrap()` on a `None` value
//! exit=101
//! ```
//!
//! 根因：`ir_gen` 的 rebind 识别（`Type.m = fn[位置]`）要求 `Index` 基底是 `Expr::Var`；
//! 本例基底是 `FieldAccess(Temp, pick)` ⇒ 识别不命中 ⇒ 落到 `// MethodBind` 分支，
//! 对**无标注**赋值执行 `type_annotation.as_ref().unwrap()`。
//!
//! # 改后判据
//!
//! 该形态在**编译期**被拒绝（E1110），进程不崩；IR 生成返回诊断而非 panic。
//! typecheck 层对该形态不报错（本夹具即其证据）——诊断的落点就是唯一的崩溃点。

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;
use crate::frontend::core::typecheck::checker::TypeChecker;
use crate::frontend::module::registry::ModuleRegistry;
use crate::middle::core::ir_gen::AstToIrGenerator;

/// 真实链路探针：类型检查诊断码 / IR 生成结果（诊断码集合）。
struct MethodBindProbe {
    typecheck_codes: Vec<String>,
    ir_codes: Vec<String>,
}

/// 源码 → lex→parse→typecheck→ir_gen 真实生成序列（与语料判据同一路径）。
fn probe_method_bind(source: &str) -> MethodBindProbe {
    let tokens = tokenize(source).expect("夹具必须可 tokenize");
    let parsed = parse(&tokens);
    assert!(
        !parsed.has_errors,
        "夹具必须无语法错误: {:?}",
        parsed.errors
    );
    let mut checker = TypeChecker::new("<method-bind-probe>");
    let type_result = checker.check_module(&parsed.module);
    let typecheck_codes: Vec<String> = type_result
        .diagnostics
        .iter()
        .map(|d| d.code.clone())
        .collect();
    let mut generator =
        AstToIrGenerator::new_with_type_result(&type_result, ModuleRegistry::with_std(), None);
    let ir_codes = match generator.generate_module_ir(&parsed.module) {
        Ok(_) => Vec::new(),
        Err(diags) => diags.iter().map(|d| d.code.clone()).collect(),
    };
    MethodBindProbe {
        typecheck_codes,
        ir_codes,
    }
}

/// issue #451 的最小复现：方法组当值再绑定（基数是 `FieldAccess(Temp, pick)`）、无签名标注。
const METHOD_GROUP_REBIND_FIXTURE: &str = r#"
Temp: Type = {
    v: Int,
}
Temp.pick: (self: &Temp, a: Int) -> Int = (self, a) => { return a }
Temp.pick: (self: &Temp, a: String) -> String = (self, a) => { return a }
Temp.pick2 = Temp.pick[0]
main: () -> Void = {
    t = Temp(1)
    r: Int = t.pick(1)
}
main()
"#;

/// 无签名标注的方法绑定：不得 panic（改前 exit=101），必须产出 E1110 诊断。
#[test]
fn test_method_binding_without_signature_reports_diagnostic_instead_of_panicking() {
    // Arrange / Act：真实链路——改前此调用会让进程 panic（Option::unwrap on None）
    let probe = probe_method_bind(METHOD_GROUP_REBIND_FIXTURE);

    // Assert：typecheck 放行（本形态的缺陷不在类型检查层），IR 生成公开拒绝
    assert!(
        probe.typecheck_codes.is_empty(),
        "该夹具在 typecheck 层无诊断（诊断落点是唯一崩溃点 ir_gen）: {:?}",
        probe.typecheck_codes
    );
    assert_eq!(
        probe.ir_codes,
        vec!["E1110".to_string()],
        "无签名标注的方法绑定必须产出 E1110（改前是 unwrap panic）"
    );
}

/// 同一形态的第二种写法：类型体外的**裸 lambda** 方法绑定（无标注）同样落 E1110，
/// 不得因「有 Lambda 值」而静默走另一条分支。
const BARE_LAMBDA_BIND_FIXTURE: &str = r#"
Temp: Type = { v: Int }
Temp.pick = (self: &Temp, a: Int) => { return a }
main: () -> Void = { t = Temp(1) }
main()
"#;

/// 裸 lambda + 无标注：与字段下标形态同一根因（MethodBind 分支拿到 None 标注）。
#[test]
fn test_method_binding_bare_lambda_without_signature_reports_diagnostic() {
    // Arrange / Act
    let probe = probe_method_bind(BARE_LAMBDA_BIND_FIXTURE);

    // Assert
    assert!(
        probe.typecheck_codes.is_empty(),
        "该夹具在 typecheck 层无诊断: {:?}",
        probe.typecheck_codes
    );
    assert_eq!(
        probe.ir_codes,
        vec!["E1110".to_string()],
        "裸 lambda 的无标注方法绑定同样必须报 E1110"
    );
}

/// 反例（阴性对照）：**有**签名标注的既有形态不得被新诊断误伤——重绑定识别
/// 命中 `Index(Var)` 时不进 MethodBind 分支，行为与改动前一致（无诊断）。
const ANNOTATED_REBIND_FIXTURE: &str = r#"
Temp: Type = { v: Int }
fa: (self: &Temp, a: Int) -> Int = (self, a) => { return a * 10 }
Temp.pick: (self: &Temp, a: Int) -> Int = fa[0]
main: () -> Void = {
    t = Temp(1)
    r: Int = t.pick(2)
}
main()
"#;

/// 有标注的重绑定：零诊断（新诊断不得扫到既有合法形态）。
#[test]
fn test_annotated_method_rebind_still_compiles_without_diagnostics() {
    // Arrange / Act
    let probe = probe_method_bind(ANNOTATED_REBIND_FIXTURE);

    // Assert
    assert!(
        probe.typecheck_codes.is_empty() && probe.ir_codes.is_empty(),
        "有签名标注的重绑定必须照常编译: typecheck={:?} ir={:?}",
        probe.typecheck_codes,
        probe.ir_codes
    );
}
