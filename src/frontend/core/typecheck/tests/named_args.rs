//! #452：命名实参纳入方法级重载决议（WBS 3.14.1）
//!
//! 规范来源:
//! - RFC-011a §3（方法级重载：调用点按 实参兼容 + 期望返回 决议候选）
//! - 用户 2026-10-14 裁决（实参绑定规则，主流约定，禁止静默择一）：
//!   ① 位置实参按声明顺序占前导槽位；② 命名实参按形参名入槽；
//!   ③ 位置与命名争同一槽位 → 报错；④ 命名之后不得再出现位置实参 → 报错；
//!   ⑤ 同一形参重复命名 → 报错；⑥ 重载候选选择必须把命名实参计入匹配。
//!
//! 本文件是四形态（纯命名 / 混合 / 命名与位置冲突 / 命名重复绑定）的判据所在，
//! 并为验收矩阵（值级 oracle）提供逐条钉板。

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;
use crate::frontend::core::typecheck::checker::TypeChecker;
use crate::frontend::module::registry::ModuleRegistry;
use crate::middle::core::ir::Instruction;
use crate::middle::core::ir_gen::AstToIrGenerator;

/// 夹具的收尾：一个可观察的顶层值绑定（解释器全局槽位可见）。
const TAIL: &str = "\nr: Int = main()\n";

/// 方法夹具头：Temp.pick(self, n: Int, a: Int) = n * 100 + a。
/// 值级 oracle 单点可辨：无论命名实参怎么写，正确绑定必得 102。
const PICK: &str = r#"
Temp: Type = { v: Int }
Temp.pick: (self: &Temp, n: Int, a: Int) -> Int = (self, n, a) => { return n * 100 + a }
"#;

struct Probe {
    diagnostics: Vec<String>,
    call_targets: Vec<String>,
    slot_values: Vec<String>,
    exec_error: Option<String>,
}

/// 真实序列探针：lex→parse→typecheck→ir_gen→codegen→解释器。
///
/// 诊断非空时短路（后续阶段的前提不成立），只回诊断。
fn probe(source: &str) -> Probe {
    let tokens = tokenize(source).expect("夹具必须可 tokenize");
    let parsed = parse(&tokens);
    assert!(
        !parsed.has_errors,
        "夹具必须无语法错误: {:?}",
        parsed.errors
    );
    let mut checker = TypeChecker::new("<named-args-probe>");
    let type_result = checker.check_module(&parsed.module);
    let diagnostics: Vec<String> = type_result
        .diagnostics
        .iter()
        .map(|d| d.code.clone())
        .collect();
    let mut generator =
        AstToIrGenerator::new_with_type_result(&type_result, ModuleRegistry::with_std(), None);
    let ir = generator.generate_module_ir(&parsed.module);
    let call_targets: Vec<String> = match &ir {
        Ok(ir) => {
            let mut names: Vec<String> = ir
                .functions
                .iter()
                .flat_map(|func| func.all_instructions())
                .chain(ir.init.iter())
                .filter_map(|inst| match inst {
                    Instruction::Call {
                        func:
                            crate::middle::core::ir::Operand::Const(
                                crate::middle::core::ir::ConstValue::String(name),
                            ),
                        ..
                    } => Some(name.clone()),
                    _ => None,
                })
                .collect();
            names.sort();
            names
        }
        Err(errors) => vec![format!(
            "<IR 错误 {:?}>",
            errors.iter().map(|d| d.code.clone()).collect::<Vec<_>>()
        )],
    };
    // 诊断非空（含 IR 生成期诊断）时短路：后续阶段的前提不成立。
    // IR 生成期诊断同样并入 diagnostics——调用点错误可能只在这一侧响亮。
    let mut diagnostics = diagnostics;
    if ir.is_err() {
        diagnostics.extend(
            ir.as_ref()
                .err()
                .map(|errors| errors.iter().map(|d| d.code.clone()).collect::<Vec<_>>())
                .unwrap_or_default(),
        );
    }
    if !diagnostics.is_empty() {
        return Probe {
            diagnostics,
            call_targets,
            slot_values: Vec::new(),
            exec_error: None,
        };
    }
    let module = ir.expect("诊断为空时 IR 生成必须有产物");
    let mut ctx = crate::middle::passes::codegen::CodegenContext::new(module);
    let file = ctx.generate().expect("codegen 必须成功");
    let bytecode_module = crate::middle::bytecode::BytecodeModule::from(file);
    let mut interp = crate::backends::interpreter::Interpreter::new();
    use crate::backends::Executor as _;
    let exec_error = interp
        .execute_module(&bytecode_module)
        .err()
        .map(|e| format!("{e}"));
    let slot_values: Vec<String> = (0..8)
        .map_while(|i| interp.global_slot(i).map(|v| format!("{v:?}")))
        .collect();
    Probe {
        diagnostics,
        call_targets,
        slot_values,
        exec_error,
    }
}

/// 夹具包装：main 体给一句 return。
fn fixture(body: &str) -> String {
    format!("{PICK}main: () -> Int = {{\n    t = Temp(1)\n    {body}\n}}\n{TAIL}")
}

/// 夹具里的计算结果必须真算出来（值级 oracle）。
fn assert_value(
    source: &str,
    expected: &str,
    label: &str,
) {
    let p = probe(source);
    assert!(
        p.diagnostics.is_empty(),
        "{label}: 不得报编译期诊断 {:?}",
        p.diagnostics
    );
    assert!(
        p.exec_error.is_none(),
        "{label}: 执行不得报错 {:?}",
        p.exec_error
    );
    assert!(
        p.slot_values.iter().any(|v| v == expected),
        "{label}: 期望算得 {expected}，实际全局槽位 {:?}",
        p.slot_values
    );
}

// ── 验收矩阵 1：纯位置实参（回归：不得因本轮改动而变） ──────────────
#[test]
fn test_named_args_positional_only_still_binds_in_order() {
    // Arrange
    let source = fixture("return t.pick(1, 2)");
    // Act / Assert
    assert_value(&source, "Int(102)", "纯位置实参");
}

// ── 验收矩阵 2：混合（位置 + 命名；位置在前、命名不争其槽位） ──────
#[test]
fn test_named_args_mixed_positional_and_named_binds_both() {
    // Arrange：1 → n（前导槽位），a = 2 → a
    let source = fixture("return t.pick(1, a = 2)");
    // Act
    let p = probe(&source);
    // Assert：算对（102 要求 n=1 与 a=2 同时到位——旧路径会丢位置实参）
    assert_value(&source, "Int(102)", "混合实参（位置 + 命名）");
    assert!(
        p.call_targets.iter().any(|n| n == "Temp.pick"),
        "无重载时按裸名调用: {:?}",
        p.call_targets
    );
}

// ── 验收矩阵 3：纯命名，书写序无关 ────────────────────────────────
#[test]
fn test_named_args_named_only_binds_by_name_regardless_of_order() {
    // Arrange
    let forward = fixture("return t.pick(n = 1, a = 2)");
    let reversed = fixture("return t.pick(a = 2, n = 1)");
    // Act / Assert
    assert_value(&forward, "Int(102)", "纯命名（声明序）");
    assert_value(&reversed, "Int(102)", "纯命名（逆序）");
}

// ── 验收矩阵 4：缺实参必须报错（不得补 0、不得把命名实参当位置实参） ─
#[test]
fn test_named_args_missing_required_reports_argument_count() {
    // Arrange
    let source = fixture("return t.pick(a = 2)");
    // Act
    let p = probe(&source);
    // Assert
    assert!(
        p.diagnostics.contains(&"E1010".to_string()),
        "缺 n 必须报 E1010（不得静默补 0）：{:?}",
        p.diagnostics
    );
}

// ── 验收矩阵 5：位置实参与命名争同一槽位 → 报错（不得静默择一） ────
#[test]
fn test_named_args_positional_slot_conflict_reports_duplicate() {
    // Arrange：单值形参方法上 pick2(1, a = 2) —— 1 已占 a 的槽位
    let source = format!(
        "Temp: Type = {{ v: Int }}\n\
         Temp.pick2: (self: &Temp, a: Int) -> Int = (self, a) => a\n\
         main: () -> Int = {{\n    t = Temp(1)\n    return t.pick2(1, a = 2)\n}}\n{TAIL}"
    );
    // Act
    let p = probe(&source);
    // Assert
    assert!(
        p.diagnostics.contains(&"E1015".to_string()),
        "位置与命名争同一槽位必须报 E1015：{:?}",
        p.diagnostics
    );
}

// ── 验收矩阵 6：同一形参重复命名 → 报错 ───────────────────────────
#[test]
fn test_named_args_repeated_name_reports_duplicate() {
    // Arrange
    let source = fixture("return t.pick(n = 1, n = 2)");
    // Act
    let p = probe(&source);
    // Assert
    assert!(
        p.diagnostics.contains(&"E1015".to_string()),
        "同一形参重复命名必须报 E1015：{:?}",
        p.diagnostics
    );
}

// ── 验收矩阵 7：命名之后不得再出现位置实参 → 报错 ─────────────────
#[test]
fn test_named_args_positional_after_named_reports_error() {
    // Arrange
    let source = fixture("return t.pick(a = 2, 1)");
    // Act
    let p = probe(&source);
    // Assert
    assert!(
        !p.diagnostics.is_empty(),
        "命名实参之后的位置实参必须报错（不得静默择一）：{:?}",
        p.diagnostics
    );
}

// ── 验收矩阵 8：重载选择必须把命名实参计入（元数不同 → 名字只在一条签名上） ──
#[test]
fn test_named_args_select_overload_by_parameter_name() {
    // Arrange：两个候选元数不同（1 值形参 / 2 值形参），实参名 a 只与一元候选相容；
    // 若命名实参不计入匹配，一元候选会因「实参数 0 ≠ 形参数」被淘汰，
    // 退回默认绑定（二候选）→ 运行期 E6007。
    let source = format!(
        "Temp: Type = {{ v: Int }}\n\
         Temp.pick2: (self: &Temp, a: Int) -> Int = (self, a) => a * 10\n\
         Temp.pick2: (self: &Temp, x: Int, a: Int) -> Int = (self, x, a) => a\n\
         main: () -> Int = {{\n    t = Temp(1)\n    return t.pick2(a = 2)\n}}\n{TAIL}"
    );
    // Act
    let p = probe(&source);
    // Assert
    assert!(
        p.diagnostics.is_empty(),
        "命名实参相容的一元候选必须被选中：{:?}",
        p.diagnostics
    );
    assert!(
        p.slot_values.iter().any(|v| v == "Int(20)"),
        "应命中形参名含 a 的一元候选（2 * 10 = 20），实际 {:?}（说明命名实参未计入匹配）",
        p.slot_values
    );
}

// ── 重载 + 混合：位置实参与命名实参共同决定候选 ───────────────────
#[test]
fn test_named_args_overload_with_mixed_arguments_selects_by_both() {
    // Arrange：候选按首值形参类型分（Int / String），命名实参 a 固定。
    // 位置实参 1 只与 Int 候选相容——两候选都被计入匹配时决议唯一定。
    let source = format!(
        "Temp: Type = {{ v: Int }}\n\
         Temp.pick: (self: &Temp, n: Int, a: Int) -> Int = (self, n, a) => n * 100 + a\n\
         Temp.pick: (self: &Temp, n: String, a: Int) -> Int = (self, n, a) => a * 1000\n\
         main: () -> Int = {{\n    t = Temp(1)\n    return t.pick(1, a = 2)\n}}\n{TAIL}"
    );
    // Act
    let p = probe(&source);
    // Assert
    assert!(
        p.diagnostics.is_empty(),
        "混合实参必须决议出 Int 候选：{:?}",
        p.diagnostics
    );
    assert_value(&source, "Int(102)", "重载 + 混合实参");
}
