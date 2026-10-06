//! WBS 2.1 / RFC-039 P2：IR 静态校验器判据（07-equivalence-oracle 第一层）
//!
//! 三层结构：
//! 1. 单元测试——7 项不变量逐项正反例（手工构造 IR，正例过 / 反例必报对应类别）
//! 2. SSA 模式负测试——现有多次定义策略下 verify_ssa 必红（D38：唯一定义是
//!    SSA 化要消除的能力依赖，红是正确性证据而非缺陷）
//! 3. test_verify_loose_corpus_green（WBS 2.1.2 硬门槛判据）——全语料真实生成
//!    序列产出的 ModuleIR 上 verify_loose 必须跑绿，且校验面非空（9.6 门禁的
//!    防假门禁断言：函数数与指令数必须 > 0）。

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;
use crate::frontend::core::typecheck::MonoType;
use crate::frontend::core::typecheck::checker::TypeChecker;
use crate::frontend::module::registry::ModuleRegistry;
use crate::middle::core::ir::{ConstValue, Instruction, Operand};
use crate::middle::core::ir_gen::AstToIrGenerator;
use crate::middle::core::verify::{verify_loose, verify_ssa, InvariantKind, VerifyError};
use crate::util::span::Span;

use super::corpus::{corpus_files, reaches_ir_generation};
use super::ir_build::{func, int_const, module_with, move_instr, named_slot, ret_void, temp_slot};

fn violations_of(
    result: &Result<(), VerifyError>,
    kind: InvariantKind,
) -> usize {
    match result {
        Ok(()) => 0,
        Err(e) => e.violations.iter().filter(|v| v.invariant == kind).count(),
    }
}

// =====================
// 不变量 4：jump 目标存在
// =====================

/// 不变量 4 正例：跳转目标在指令总数内合法（07 §第一层 jump 目标存在）。
#[test]
fn test_jump_target_in_range_passes() {
    // Arrange：Jmp -> 1（合法），随后 Ret
    let f = func(
        "f",
        vec![],
        MonoType::Void,
        vec![temp_slot(MonoType::Int(64))],
        vec![
            Instruction::Jmp {
                target: 1,
                span: Span::dummy(),
            },
            ret_void(),
        ],
    );
    let m = module_with(vec![f]);

    // Act
    let result = verify_loose(&m);

    // Assert
    assert!(
        result.is_ok(),
        "合法跳转目标不应产生违规：{:?}",
        result.err().map(|e| e.to_string())
    );
}

/// 不变量 4 反例：跳转目标 99 超出指令总数（2），必报 JumpTarget。
#[test]
fn test_jump_target_out_of_range_fails() {
    // Arrange：Jmp -> 99（越界）
    let f = func(
        "f",
        vec![],
        MonoType::Void,
        vec![],
        vec![
            Instruction::Jmp {
                target: 99,
                span: Span::dummy(),
            },
            ret_void(),
        ],
    );
    let m = module_with(vec![f]);

    // Act
    let r = verify_loose(&m);
    let detail = r.as_ref().err().map(|e| e.to_string()).unwrap_or_default();

    // Assert：恰好 1 项 JumpTarget 违规
    assert_eq!(
        violations_of(&r, InvariantKind::JumpTarget),
        1,
        "越界跳转应恰好报 1 项 JumpTarget 违规：{detail}"
    );
}

// =====================
// 不变量 5：槽位越界（Local/Temp/Arg/Global）
// =====================

/// 不变量 5 反例：Local(9) 超出 locals 表长（1），必报 SlotBounds。
#[test]
fn test_local_slot_out_of_bounds_fails() {
    // Arrange：locals 表长 1，指令写 Local(9)
    let f = func(
        "f",
        vec![],
        MonoType::Void,
        vec![temp_slot(MonoType::Int(64))],
        vec![move_instr(9, int_const(1)), ret_void()],
    );
    let m = module_with(vec![f]);

    // Act
    let r = verify_loose(&m);
    let detail = r.as_ref().err().map(|e| e.to_string()).unwrap_or_default();

    // Assert
    assert_eq!(
        violations_of(&r, InvariantKind::SlotBounds),
        1,
        "Local(9) 越界应恰好报 1 项 SlotBounds 违规：{detail}"
    );
}

/// 不变量 5 反例：Global(7) 超出 globals 表长（0），必报 SlotBounds。
#[test]
fn test_global_slot_out_of_bounds_fails() {
    // Arrange：globals 为空，指令读 Global(7)
    let f = func(
        "f",
        vec![],
        MonoType::Void,
        vec![temp_slot(MonoType::Int(64))],
        vec![
            Instruction::Load {
                dst: Operand::Local(0),
                src: Operand::Global(7),
                span: Span::dummy(),
            },
            ret_void(),
        ],
    );
    let m = module_with(vec![f]);

    // Act
    let r = verify_loose(&m);
    let detail = r.as_ref().err().map(|e| e.to_string()).unwrap_or_default();

    // Assert
    assert_eq!(
        violations_of(&r, InvariantKind::SlotBounds),
        1,
        "Global(7) 越界应恰好报 1 项 SlotBounds 违规：{detail}"
    );
}

// =====================
// 不变量 1：use-before-def 支配性（具名槽 must-defined）
// =====================

/// 不变量 1 正例：直线序列先定义后使用，must-defined 满足。
#[test]
fn test_linear_def_before_use_passes() {
    // Arrange：槽 0 定义于 instr#0，instr#1 读之
    let f = func(
        "f",
        vec![],
        MonoType::Void,
        vec![temp_slot(MonoType::Int(64)), temp_slot(MonoType::Int(64))],
        vec![
            move_instr(0, int_const(1)),
            move_instr(1, Operand::Local(0)),
            ret_void(),
        ],
    );
    let m = module_with(vec![f]);

    // Act
    let result = verify_loose(&m);

    // Assert
    assert!(
        result.is_ok(),
        "先定义后使用不应产生违规：{:?}",
        result.err().map(|e| e.to_string())
    );
}

/// 不变量 1 反例：具名槽在分支内定义、汇合后使用——JmpIf 为真路径跳过定义点，
/// DefIn(汇合) 不含该槽（must-defined 语义）。判定面对具名槽（temp 槽的 Void
/// 预初始化语义不在支配性判定面内，见 verify.rs check_dominance 注释）。
#[test]
fn test_branch_local_def_does_not_dominate_merge_use() {
    // Arrange：instr#1 JmpIf 为真时跳过定义点 instr#2 直达汇合 instr#3
    let f = func(
        "f",
        vec![],
        MonoType::Void,
        vec![
            named_slot("c", MonoType::Bool),
            named_slot("v", MonoType::Int(64)),
            temp_slot(MonoType::Int(64)),
        ],
        vec![
            // 0: 定义条件槽 c
            move_instr(0, Operand::Const(ConstValue::Bool(true))),
            // 1: JmpIf Local(0) -> 3（跳过定义点 2）
            Instruction::JmpIf {
                cond: Operand::Local(0),
                target: 3,
                span: Span::dummy(),
            },
            // 2: 定义具名槽 v（仅 fall-through 路径执行）
            move_instr(1, int_const(42)),
            // 3: 汇合点读 v——DefIn(3) 不含槽 1（路径 0→1→3 无定义）
            move_instr(2, Operand::Local(1)),
            ret_void(),
        ],
    );
    let m = module_with(vec![f]);

    // Act
    let r = verify_loose(&m);
    let detail = r.as_ref().err().map(|e| e.to_string()).unwrap_or_default();

    // Assert
    assert_eq!(
        violations_of(&r, InvariantKind::Dominance),
        1,
        "分支定义不支配汇合使用应恰好报 1 项 Dominance 违规：{detail}"
    );
}

/// 不变量 1 正例：循环回边的自更新读被迭代数据流正确处理（定义在循环头前）。
#[test]
fn test_loop_back_edge_self_update_passes() {
    // Arrange：i=0；循环头 JmpIfNot；体内 i=i；回跳
    let f = func(
        "f",
        vec![],
        MonoType::Void,
        vec![temp_slot(MonoType::Int(64))],
        vec![
            move_instr(0, int_const(0)), // 0: i = 0
            Instruction::JmpIfNot {
                cond: Operand::Local(0),
                target: 4,
                span: Span::dummy(),
            },
            move_instr(0, Operand::Local(0)), // 2: i = i（自更新，读被 #0 覆盖）
            Instruction::Jmp {
                target: 1,
                span: Span::dummy(),
            }, // 3: 回跳
            ret_void(),                       // 4
        ],
    );
    let m = module_with(vec![f]);

    // Act
    let result = verify_loose(&m);

    // Assert
    assert!(
        result.is_ok(),
        "循环自更新（定义先于回边）不应产生违规：{:?}",
        result.err().map(|e| e.to_string())
    );
}

/// 不变量 1 正例：参数槽是入口定义（运行时铺入帧前段），读参数无需指令级定义点。
#[test]
fn test_param_slot_read_passes_without_instr_def() {
    // Arrange：单参数函数，instr#0 读参数槽 0
    let f = func(
        "f",
        vec![MonoType::Int(64)],
        MonoType::Void,
        vec![
            named_slot("x", MonoType::Int(64)),
            temp_slot(MonoType::Int(64)),
        ],
        vec![move_instr(1, Operand::Local(0)), ret_void()],
    );
    let m = module_with(vec![f]);

    // Act
    let result = verify_loose(&m);

    // Assert
    assert!(
        result.is_ok(),
        "读参数槽（入口定义）不应产生违规：{:?}",
        result.err().map(|e| e.to_string())
    );
}
// =====================
// 不变量 6：类型一致（可信源：参数槽签名 + 常量）
// =====================

/// 不变量 6 正例：Int 参数与 Int 常量双端互容（可信源：参数槽签名 + 常量）。
#[test]
fn test_type_consistent_arith_passes() {
    // Arrange：Add temp <- param(Int64), int(1)
    let f = func(
        "f",
        vec![MonoType::Int(64)],
        MonoType::Int(64),
        vec![
            named_slot("x", MonoType::Int(64)),
            temp_slot(MonoType::Int(64)),
        ],
        vec![
            Instruction::Add {
                dst: Operand::Local(1),
                lhs: Operand::Local(0),
                rhs: int_const(1),
                span: Span::dummy(),
            },
            Instruction::Ret {
                value: Some(Operand::Local(1)),
                span: Span::dummy(),
            },
        ],
    );
    let m = module_with(vec![f]);

    // Act
    let result = verify_loose(&m);

    // Assert
    assert!(
        result.is_ok(),
        "Int 参数 + Int 常量双端互容不应产生违规：{:?}",
        result.err().map(|e| e.to_string())
    );
}

/// 不变量 6 反例：Int 参数与 Bool 常量双端不相容——可信源冲突必报。
#[test]
fn test_type_inconsistent_arith_fails() {
    // Arrange：Add temp <- param(Int64), bool(true)
    let f = func(
        "f",
        vec![MonoType::Int(64)],
        MonoType::Int(64),
        vec![
            named_slot("x", MonoType::Int(64)),
            temp_slot(MonoType::Int(64)),
        ],
        vec![
            Instruction::Add {
                dst: Operand::Local(1),
                lhs: Operand::Local(0),
                rhs: Operand::Const(ConstValue::Bool(true)),
                span: Span::dummy(),
            },
            ret_void(),
        ],
    );
    let m = module_with(vec![f]);

    // Act
    let r = verify_loose(&m);
    let detail = r.as_ref().err().map(|e| e.to_string()).unwrap_or_default();

    // Assert
    assert_eq!(
        violations_of(&r, InvariantKind::TypeConsistency),
        1,
        "Int 参数 + Bool 常量应恰好报 1 项 TypeConsistency 违规：{detail}"
    );
}

/// 不变量 6 反例：Ret 常量值 Bool 与 return_type Int 不相容（return_type 来自签名，可信）。
#[test]
fn test_ret_const_type_mismatch_fails() {
    // Arrange：fn() -> Int 返回 bool(true)
    let f = func(
        "f",
        vec![],
        MonoType::Int(64),
        vec![],
        vec![Instruction::Ret {
            value: Some(Operand::Const(ConstValue::Bool(true))),
            span: Span::dummy(),
        }],
    );
    let m = module_with(vec![f]);

    // Act
    let r = verify_loose(&m);
    let detail = r.as_ref().err().map(|e| e.to_string()).unwrap_or_default();

    // Assert
    assert_eq!(
        violations_of(&r, InvariantKind::TypeConsistency),
        1,
        "Ret Bool vs Int 返回类型应恰好报 1 项 TypeConsistency 违规：{detail}"
    );
}

/// 不变量 6 边界：非参数槽的 ty 是占位符（ir_gen 不写类型表），Bool 常量入具名 Int 槽
/// 不报——不是豁免，是判定依据不存在（verify.rs 文件头登记；完整面归 P7 值表）。
#[test]
fn test_untyped_local_slot_not_type_checked() {
    // Arrange：Bool 常量写入具名 Int 槽（槽类型是占位 Int(64)，不参与比对）
    let f = func(
        "f",
        vec![],
        MonoType::Void,
        vec![named_slot("x", MonoType::Int(64))],
        vec![
            move_instr(0, Operand::Const(ConstValue::Bool(true))),
            ret_void(),
        ],
    );
    let m = module_with(vec![f]);

    // Act
    let result = verify_loose(&m);

    // Assert
    assert!(
        result.is_ok(),
        "非参数槽类型占位不参与比对（判定依据不存在）：{:?}",
        result.err().map(|e| e.to_string())
    );
}

/// 不变量 6 边界：Int(32) 参数与 Int(64) 常量双端相容（宽度族互容，P5 收敛前的现行表示）。
#[test]
fn test_int_width_family_compatible() {
    // Arrange：Add temp <- param(Int32), int(1)
    let f = func(
        "f",
        vec![MonoType::Int(32)],
        MonoType::Int(64),
        vec![
            named_slot("x", MonoType::Int(32)),
            temp_slot(MonoType::Int(64)),
        ],
        vec![
            Instruction::Add {
                dst: Operand::Local(1),
                lhs: Operand::Local(0),
                rhs: int_const(1),
                span: Span::dummy(),
            },
            ret_void(),
        ],
    );
    let m = module_with(vec![f]);

    // Act
    let result = verify_loose(&m);

    // Assert
    assert!(
        result.is_ok(),
        "Int 宽度族互容不应产生违规：{:?}",
        result.err().map(|e| e.to_string())
    );
}

// =====================
// 不变量 2：值定义唯一性（SSA 模式语义）
// =====================

/// 不变量 2（SSA 模式语义）：同槽两次定义——Loose 放行（现行语句级回收策略），
/// Ssa 必报。这是 D38 红性的直接证据：verify_ssa 在现有分配策略下红是设计预期。
#[test]
fn test_duplicate_definition_loose_passes_ssa_fails() {
    // Arrange：槽 0 定义两次（instr#0/#1），instr#2 读之
    let mk = || {
        func(
            "f",
            vec![],
            MonoType::Void,
            vec![temp_slot(MonoType::Int(64)), temp_slot(MonoType::Int(64))],
            vec![
                move_instr(0, int_const(1)),
                move_instr(0, int_const(2)),
                move_instr(1, Operand::Local(0)),
                ret_void(),
            ],
        )
    };

    // Act：同一 IR 分别跑两种模式
    let loose = verify_loose(&module_with(vec![mk()]));
    let ssa = verify_ssa(&module_with(vec![mk()]));
    let detail = ssa
        .as_ref()
        .err()
        .map(|e| e.to_string())
        .unwrap_or_default();

    // Assert：Loose 绿（允许多次定义）；Ssa 恰好 1 项唯一定义违规
    assert!(
        loose.is_ok(),
        "Loose 模式应放行多次定义：{:?}",
        loose.err().map(|e| e.to_string())
    );
    assert_eq!(
        violations_of(&ssa, InvariantKind::UniqueDefinition),
        1,
        "Ssa 模式对双定义槽应恰好报 1 项 UniqueDefinition 违规：{detail}"
    );
}

/// 不变量 2 正例：每槽单定义、参数槽不被重写的 SSA 形态，verify_ssa 绿。
#[test]
fn test_ssa_shaped_ir_passes_verify_ssa() {
    // Arrange：参数槽 0 只读，结果写槽 1（单定义）
    let f = func(
        "f",
        vec![MonoType::Int(64)],
        MonoType::Int(64),
        vec![
            named_slot("x", MonoType::Int(64)),
            temp_slot(MonoType::Int(64)),
        ],
        vec![
            move_instr(1, Operand::Local(0)),
            Instruction::Ret {
                value: Some(Operand::Local(1)),
                span: Span::dummy(),
            },
        ],
    );
    let m = module_with(vec![f]);

    // Act
    let result = verify_ssa(&m);

    // Assert
    assert!(
        result.is_ok(),
        "SSA 形态 IR 应过 verify_ssa：{:?}",
        result.err().map(|e| e.to_string())
    );
}

// =====================
// 不变量 7：内层隔离
// =====================

/// 不变量 7 反例：外层 MakeClosure 引用的闭包，其自有局部名混入外层 locals——
/// save/restore 契约（ir_gen.rs:252-253）违反形态。
#[test]
fn test_inner_local_name_leaking_to_outer_fails() {
    // Arrange：外层 locals 含 inner_var；闭包自有局部也叫 inner_var
    let outer = func(
        "outer",
        vec![],
        MonoType::Void,
        vec![
            temp_slot(MonoType::Int(64)),
            named_slot("inner_var", MonoType::Int(64)),
        ],
        vec![
            Instruction::MakeClosure {
                dst: Operand::Local(0),
                func: "closure_0".to_string(),
                def: None,
                env: vec![],
                span: Span::dummy(),
            },
            ret_void(),
        ],
    );
    let closure = func(
        "closure_0",
        vec![],
        MonoType::Void,
        vec![named_slot("inner_var", MonoType::Int(64))],
        vec![ret_void()],
    );
    let m = module_with(vec![outer, closure]);

    // Act
    let r = verify_loose(&m);
    let detail = r.as_ref().err().map(|e| e.to_string()).unwrap_or_default();

    // Assert
    assert_eq!(
        violations_of(&r, InvariantKind::InnerIsolation),
        1,
        "内层局部名串入外层应恰好报 1 项 InnerIsolation 违规：{detail}"
    );
}

/// 不变量 7 正例：闭包自有局部名与外层不相交；同名名处于参数槽位不参与比较
///（参数遮蔽是合法语义）。
#[test]
fn test_disjoint_inner_locals_pass() {
    // Arrange：外层局部 outer_var；闭包参数槽同名（idx 0 < params.len() 排除），
    // 自有局部 inner_only 不相交
    let outer = func(
        "outer",
        vec![],
        MonoType::Void,
        vec![
            temp_slot(MonoType::Int(64)),
            named_slot("outer_var", MonoType::Int(64)),
        ],
        vec![
            Instruction::MakeClosure {
                dst: Operand::Local(0),
                func: "closure_0".to_string(),
                def: None,
                env: vec![],
                span: Span::dummy(),
            },
            ret_void(),
        ],
    );
    let closure = func(
        "closure_0",
        vec![MonoType::Int(64)],
        MonoType::Void,
        vec![
            named_slot("outer_var", MonoType::Int(64)),
            named_slot("inner_only", MonoType::Int(64)),
        ],
        vec![ret_void()],
    );
    let m = module_with(vec![outer, closure]);

    // Act
    let result = verify_loose(&m);

    // Assert
    assert!(
        result.is_ok(),
        "内外层局部名不相交（同名参数属合法遮蔽）不应产生违规：{:?}",
        result.err().map(|e| e.to_string())
    );
}
// =====================
// WBS 2.1.2 判据：verify_loose 在现有非 SSA IR 上跑绿（P7 批 a 准入硬门槛）
// =====================

/// 全语料（tests/yaoxiang/ + src/std/tests/，剔 skip / compile-error——
/// 不到达 IR 生成的文件不在合同面）逐文件走 lex→parse→typecheck→ir_gen
/// 真实生成序列，产出的 ModuleIR 必须过 verify_loose。
///
/// D38：跑不绿不开豁免——跑不绿说明 ir_gen 存在隐式「同槽多次写」类
/// 依赖，是必须登记的缺陷。本判据红 = P7 批 a  blocked，如实报告。
///
/// 非空断言（门禁 9.6 防假门禁）：校验的函数数与指令数必须 > 0，
/// 防止「校验器根本没跑到」的假绿。
#[test]
fn test_verify_loose_corpus_green() {
    let mut files_checked = 0usize;
    let mut fns_checked = 0usize;
    let mut instrs_checked = 0usize;
    let mut failures: Vec<String> = Vec::new();

    for file in corpus_files() {
        if !reaches_ir_generation(&file) {
            continue;
        }
        let relative = file
            .strip_prefix(env!("CARGO_MANIFEST_DIR"))
            .unwrap_or(&file)
            .display()
            .to_string();
        // 生成前置阶段失败的文件不构成判据证据（同 2.4.4 合同面判定）
        let Ok(source) = std::fs::read_to_string(&file) else {
            continue;
        };
        let Ok(tokens) = tokenize(&source) else {
            continue;
        };
        let parsed = parse(&tokens);
        if parsed.has_errors {
            continue;
        }
        let mut checker = TypeChecker::new(&file.display().to_string());
        let type_result = checker.check_module(&parsed.module);
        if !type_result.diagnostics.is_empty() {
            continue;
        }
        let mut generator =
            AstToIrGenerator::new_with_type_result(&type_result, ModuleRegistry::with_std(), None);
        let Ok(module_ir) = generator.generate_module_ir(&parsed.module) else {
            continue;
        };

        files_checked += 1;
        fns_checked += module_ir.functions.len();
        instrs_checked += module_ir
            .functions
            .iter()
            .map(|f| f.all_instructions().count())
            .sum::<usize>()
            + module_ir.init.len();

        if let Err(e) = verify_loose(&module_ir) {
            for v in e.violations {
                failures.push(format!(
                    "{relative}: [{}] {}: {}",
                    v.invariant.label(),
                    v.function,
                    v.message
                ));
            }
        }
    }

    // 非空断言：校验面必须真实覆盖语料（防「没跑到」的假绿）
    assert!(
        files_checked > 200 && fns_checked > 0 && instrs_checked > 0,
        "校验面异常收窄（files={files_checked}, fns={fns_checked}, instrs={instrs_checked}）——判据失效"
    );
    assert!(
        failures.is_empty(),
        "verify_loose 在现有 IR 上跑红（D38：不开豁免，红 = ir_gen 缺陷，
         files={files_checked} fns={fns_checked} instrs={instrs_checked}）：\n{}",
        failures.join("\n")
    );
}
