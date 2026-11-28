//! #593 判据：调用点结果类型（typecheck → IR 生成，**调用点 PlanId 键控**）。
//!
//! 判据形态：走与快照同一套真实生成序列（lex → parse → typecheck →
//! AstToIrGenerator）后读 main 的值表——Call 指令的 dst 必须带 sema 推断出的
//! 具体类型，而不是显式未知 TypeRef("?")。覆盖四种被调形态：限定名（原生
//! string.len）/ 方法（Point.get）/ 构造器（Point）/ 裸名（mk）。前两者在 IR
//! 侧按被调名反查不到（原生签名与限定名只在 sema 手里），这正是本条要钉的性质。
//!
//! 该入口与 src/middle/ir/tests/snapshots/ 的入库基线同源：不经 mono/分配器，
//! 于是 Call 的 dst 仍是值表里的 Operand::Value，可逐条核对类型。
//!
//! # 门依据（coding-rules 第七部分）
//!
//! **证明门**：性质 =「调用点的结果类型是 sema 的事实，IR 侧不得自行重算、
//! 也不得落空」。换一个正确的替代实现（例如把这张表换成 L3 侧解析）不会让
//! 本条变红——断言的是值表里出现了具体类型，不是表的形状。
//!
//! 边界（不在本条判据内）：未收敛实例化（dict.new() 的 Dict(K, V)）与 lowering
//! 合成调用（for 协议 / 问号解糖）不落表——前者 sema 没有收敛的事实，后者在
//! AST 里没有调用节点；IR 对它们保持显式未知（不猜、不泄漏 TypeVar）。

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;
use crate::frontend::core::sema::checker::TypeChecker;
use crate::frontend::core::sema::probe::TypeLevelNamespace;
use crate::frontend::core::sema::MonoType;
use crate::frontend::module::registry::ModuleRegistry;
use crate::middle::ir::ir::{FunctionBody, Instruction, ModuleIR, Operand};
use crate::middle::lower::AstToIrGenerator;
use crate::util::diagnostic::Severity;

/// 源码 → 真实生成序列（lex → parse → typecheck → ir_gen）的直驱探针。
fn compile_ir(source: &str) -> ModuleIR {
    let tokens = tokenize(source).expect("夹具必须可 tokenize");
    let parsed = parse(&tokens);
    assert!(
        !parsed.has_errors,
        "夹具必须无语法错误: {:?}",
        parsed.errors
    );
    let mut checker = TypeChecker::new("<call-result-types>");
    let type_result = checker.check_module(
        &parsed.module,
        &TypeLevelNamespace::scan_one(&parsed.module),
    );
    let errors: Vec<_> = type_result
        .diagnostics
        .iter()
        .filter(|d| matches!(d.severity, Severity::Error))
        .collect();
    assert!(errors.is_empty(), "夹具必须无类型错误: {errors:?}");
    let mut generator = AstToIrGenerator::new_with_type_result(
        &type_result,
        ModuleRegistry::with_std(),
        None,
        TypeLevelNamespace::scan_one(&parsed.module),
    );
    generator
        .generate_module_ir(&parsed.module)
        .expect("夹具必须生成 IR")
}

/// 显式未知的类型（IR 的「不知道」是 TypeRef("?")，不是 TypeVar）。
fn is_explicit_unknown(ty: &MonoType) -> bool {
    matches!(ty, MonoType::TypeRef(name) if name == "?")
}

/// main 体内每条 Call 的（被调词, 结果类型）；dst 不是值表项时跳过
/// （pinned 到具名槽的形态在快照基线与语料面覆盖）。
fn main_call_types(ir: &ModuleIR) -> Vec<(String, MonoType)> {
    let mut out = Vec::new();
    for function in &ir.functions {
        if function.name != "main" {
            continue;
        }
        let FunctionBody::Code { blocks, values, .. } = &function.body else {
            continue;
        };
        for block in blocks {
            for instr in &block.instructions {
                let Instruction::Call {
                    dst: Some(Operand::Value(id)),
                    func,
                    ..
                } = instr
                else {
                    continue;
                };
                let info = values
                    .get(id.0 as usize)
                    .expect("Call 的 dst 必须落在值表内");
                out.push((format!("{func:?}"), info.ty.clone()));
            }
        }
    }
    out
}

fn describe(calls: &[(String, MonoType)]) -> String {
    calls
        .iter()
        .map(|(callee, ty)| format!("{callee} -> {ty}"))
        .collect::<Vec<_>>()
        .join(" | ")
}

/// 四种被调形态（限定名原生 / 方法 / 构造器 / 裸名）的调用结果都必须落具体类型。
#[test]
fn test_call_result_types_land_in_value_table_for_source_call_sites() {
    // Arrange：覆盖 sema 独有事实的两种（原生 string.len、方法 Point.get）
    // 与 IR 侧本可自行解析的两种（构造器 Point、裸名 mk）。
    let source = r#"
use std.string

Point: Type = { x: Int }
Point.get: (self: &Point) -> Int = (self) => self.x

mk: (n: Int) -> Int = (n) => n + 1

main: () -> Int = {
    mk(Point(1).get()) + string.len("hello")
}

main()
"#;

    // Act
    let ir = compile_ir(source);
    let calls = main_call_types(&ir);

    // Assert
    assert_eq!(
        calls.len(),
        4,
        "夹具必须产出四个调用点（构造器 / 方法 / 裸名 / 限定名）: functions={:?}",
        ir.functions
            .iter()
            .map(|f| f.name.as_str())
            .collect::<Vec<_>>()
    );
    let unknown: Vec<&String> = calls
        .iter()
        .filter(|(_, ty)| is_explicit_unknown(ty))
        .map(|(callee, _)| callee)
        .collect();
    assert!(
        unknown.is_empty(),
        "调用结果不得落显式未知（sema 已产出的事实必须进值表）: unknown={unknown:?} all={}",
        describe(&calls)
    );
    let int_results = calls
        .iter()
        .filter(|(_, ty)| matches!(ty, MonoType::Int(64)))
        .count();
    assert_eq!(
        int_results,
        3,
        "方法 / 裸名 / 限定名三个调用的结果都应是 Int(64): {}",
        describe(&calls)
    );
}
