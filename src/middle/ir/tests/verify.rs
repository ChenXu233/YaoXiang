//! WBS 2.1 / RFC-039 P2：IR 静态校验器判据（07-equivalence-oracle 第一层）
//!
//! 三层结构：
//! 1. 单元测试——7 项不变量逐项正反例（手工构造 IR，正例过 / 反例必报对应类别）
//! 2. SSA 模式负测试——现有多次定义策略下 verify_ssa 必红（D38：唯一定义是
//!    SSA 化要消除的能力依赖，红是正确性证据而非缺陷）
//! 3. test_verify_loose_corpus_green（WBS 2.1.2 硬门槛判据）——全语料真实生成
//!    序列产出的 ModuleIR 上 verify_loose 必须跑绿，且校验面非空（9.6 门禁的
//!    防假门禁断言：函数数与指令数必须 > 0）。

use crate::frontend::compiler::Compiler;
use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;
use crate::frontend::core::sema::checker::TypeChecker;
use crate::frontend::core::sema::probe::TypeLevelNamespace;
use crate::frontend::core::sema::MonoType;
use crate::frontend::core::types::identity::Nominal;
use crate::frontend::module::registry::ModuleRegistry;
use crate::middle::ir::ir::{
    ConstValue, FrameId, Instruction, LocalSlot, ModuleIR, Operand, ValueInfo,
};
use crate::middle::ir::verify::{verify_loose, verify_ssa, InvariantKind, VerifyError};
use crate::middle::lower::AstToIrGenerator;
use crate::util::span::Span;

use super::corpus::{corpus_files, reaches_ir_generation};
use super::ir_build::{
    block, func, func_with_blocks, int_const, jmp, module_with, move_instr, named_slot,
    named_slot_in_frame, phi_instr, ret_void, ssa_value, temp_slot, value_info,
};

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

/// Local 多次写是存储语义：Loose 与 SSA 都必须放行（值唯一性只判 Value）。
#[test]
fn test_local_multiple_writes_loose_and_ssa_pass() {
    // Arrange：槽 0 写两次（instr#0/#1），instr#2 读之
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

    // Act
    let loose = verify_loose(&module_with(vec![mk()]));
    let ssa = verify_ssa(&module_with(vec![mk()]));

    // Assert
    assert!(
        loose.is_ok(),
        "Loose 模式应放行 Local 多次写：{:?}",
        loose.err().map(|e| e.to_string())
    );
    assert!(
        ssa.is_ok(),
        "Local 多次写是存储语义，SSA 模式也必须放行：{:?}",
        ssa.err().map(|e| e.to_string())
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
// 不变量 7：内层隔离（WBS 2.1.4 起按**槽身份**判定：LocalSlot::origin）
// =====================

/// 不变量 7 负控（手工 IR 直驱）：一张槽位表混入**两个生成帧** ⇒ 必红。
///
/// 身份证据而非名字：两个槽都是 temp（连名字都没有），只有 `LocalSlot::origin`
/// 不同；且全模块**没有 MakeClosure** —— 判据不再从「谁引用了谁」反推词法嵌套。
#[test]
fn test_inner_isolation_foreign_frame_slot_rejected() {
    // Arrange：同一函数的表里，槽 0 来自帧 1、槽 1 来自帧 2
    let leaked = func(
        "closure_0",
        vec![],
        MonoType::Void,
        vec![
            LocalSlot::temp_in(MonoType::Int(64), FrameId(1)),
            LocalSlot::temp_in(MonoType::Int(64), FrameId(2)),
        ],
        vec![ret_void()],
    );
    let m = module_with(vec![leaked]);

    // Act
    let r = verify_loose(&m);
    let detail = r.as_ref().err().map(|e| e.to_string()).unwrap_or_default();

    // Assert：负控必须红，且 IR 里没有任何 MakeClosure（判据不依赖嵌套推断）
    assert!(
        m.functions.iter().all(|f| !f
            .all_instructions()
            .any(|i| matches!(i, Instruction::MakeClosure { .. }))),
        "负控前置：被判定的 IR 不得含 MakeClosure（否则证明不了判据已脱离嵌套推断）"
    );
    assert_eq!(
        violations_of(&r, InvariantKind::InnerIsolation),
        1,
        "同一张槽位表混入两个生成帧必须报 1 项 InnerIsolation：{detail}"
    );
}

/// 不变量 7 负控（真泄漏形态）：内层函数的表里带着**外层帧**的具名槽。
///
/// 这是 lower 的 `cur_locals` save/restore 失败后 IR 的真实样子（2.1.4 的 M2
/// 变异实测：闭包表里出现外层的 bb/cc/ys）。违规归属**混入帧的那张表**，
/// 不是外层函数。
#[test]
fn test_inner_isolation_inherited_outer_slot_rejected() {
    // Arrange：外层帧 1 登记 outer_var；闭包表里既有外层帧的槽，也有本帧（2）的参数槽
    let outer = func(
        "outer",
        vec![],
        MonoType::Void,
        vec![
            temp_slot(MonoType::Int(64)),
            named_slot_in_frame("outer_var", MonoType::Int(64), FrameId(1)),
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
            named_slot_in_frame("outer_var", MonoType::Int(64), FrameId(1)),
            named_slot_in_frame("x", MonoType::Int(64), FrameId(2)),
        ],
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
        "继承外层槽的内层表必须报 1 项 InnerIsolation：{detail}"
    );
    assert!(
        r.as_ref()
            .err()
            .is_some_and(|e| e.violations.iter().any(|v| v.function == "closure_0")),
        "违规必须归属混入帧的表（closure_0）而非外层：{detail}"
    );
}

/// 不变量 7 正例（假阳性回归，手工 IR）：两个**独立声明**用同一个源码名、
/// 各自一个生成帧 ⇒ 绿。
///
/// 这正是 2.1.4 的两处误报形态：`main` 的局部 `r` 与 `std.json.parse` 自己的 `r`。
/// 旧判据（MakeClosure 反推嵌套 + 名字交集）报红，身份判据不报。
#[test]
fn test_inner_isolation_same_local_name_across_frames_pass() {
    // Arrange：外层帧 1 的 `r` 与「作值引用的函数」（帧 2）自己的 `r` 同名
    let outer = func(
        "main",
        vec![],
        MonoType::Void,
        vec![
            temp_slot(MonoType::Int(64)),
            named_slot_in_frame("r", MonoType::Int(64), FrameId(1)),
        ],
        vec![
            Instruction::MakeClosure {
                dst: Operand::Local(0),
                func: "std.json.parse".to_string(),
                def: None,
                env: vec![],
                span: Span::dummy(),
            },
            ret_void(),
        ],
    );
    let parse = func(
        "std.json.parse",
        vec![MonoType::Int(64)],
        MonoType::Void,
        vec![
            named_slot_in_frame("s", MonoType::Int(64), FrameId(2)),
            named_slot_in_frame("r", MonoType::Int(64), FrameId(2)),
        ],
        vec![ret_void()],
    );
    let m = module_with(vec![outer, parse]);

    // Act
    let result = verify_loose(&m);

    // Assert
    assert!(
        result.is_ok(),
        "同名不同绑定（各自一个生成帧）不得报内层隔离：{:?}",
        result.err().map(|e| e.to_string())
    );
}

// =====================
// 不变量 7 定向回归（WBS 2.1.4）：两语料形态走**生产入口**后 post-mono 必须绿
// =====================

/// 走生产单文件入口（lex→parse→typecheck→ir_gen→合并嵌入 std→mono→post-mono DCE）
/// 拿到 post-mono `ModuleIR`。
///
/// 为什么不用 ir_gen 直驱：本组判据的失效点（MakeClosure 作函数值、std 模块与入口同表）
/// 只在 std 合并后出现，直驱 IR 看不到。
fn compile_post_mono(
    name: &str,
    source: &str,
) -> crate::middle::ModuleIR {
    Compiler::new().compile(name, source).unwrap_or_else(|e| {
        panic!(
            "夹具源码必须通过生产入口编译（{name}）：
{source}
错误：{e}"
        )
    })
}

/// 定向回归（语料形态 1「同名局部跨帧复用」）：两个独立函数各自用同名局部，
/// 其中一个**作函数值**传递 —— post-mono `verify_loose` 必须绿。
///
/// 病史：2.1.4 的两处红正是这条形态（旧的名字交集判据把名字复用当泄漏）。
///
/// 两个夹具缺一不可：只测撞名，判不出「结论是否取决于名字」——第二个夹具逐字相同、
/// 只把外层局部改名，两者必须同判。
#[test]
fn test_inner_isolation_independent_functions_same_local_name_pass() {
    // Arrange：撞名夹具 + 只改外层局部名的对照夹具
    let collision = r#"
use std.io

calc: (x: Int) -> Int = (x) => {
    acc = x + 1
    acc
}

main: () -> Void = {
    acc = 3
    g = calc
    n = g(acc)
    io.println("done")
}

main()
"#;
    let renamed = r#"
use std.io

calc: (x: Int) -> Int = (x) => {
    acc = x + 1
    acc
}

main: () -> Void = {
    total = 3
    g = calc
    n = g(total)
    io.println("done")
}

main()
"#;

    // Act
    let collision_ir = compile_post_mono("independent_same_local_name.yx", collision);
    let renamed_ir = compile_post_mono("independent_same_local_name_renamed.yx", renamed);

    // Assert：两者同判——判据不得由名字决定
    assert!(
        verify_loose(&collision_ir).is_ok(),
        "同名局部跨帧复用不是泄漏，post-mono 必须绿：{:?}",
        verify_loose(&collision_ir).err()
    );
    assert!(
        verify_loose(&renamed_ir).is_ok(),
        "同结构只改外层局部名后同样必须绿（判据与名字无关）：{:?}",
        verify_loose(&renamed_ir).err()
    );
}

/// 定向回归（假阳性族补充）：被提升为具名函数的 lambda 体里，for 循环变量与外层
/// 局部同名 —— 那是内层帧自己的声明（槽号与作用域深度都不同），不是泄漏。
///
/// 该提升函数再**作函数值**传递（MakeClosure），旧判据在这里会误报。
#[test]
fn test_inner_isolation_lifted_lambda_for_var_name_reuse_pass() {
    // Arrange
    let source = r#"
use std.io
use std.list

main: () -> Void = {
    item = 1
    f: () -> Int = () => {
        mut total = 0
        for item in [1, 2, 3] {
            total = total + item
        }
        total
    }
    g = f
    x = g()
    io.println("done")
}

main()
"#;

    // Act
    let m = compile_post_mono("lifted_lambda_for_var.yx", source);

    // Assert
    assert!(
        verify_loose(&m).is_ok(),
        "提升函数的 for 变量与外层同名不是泄漏，post-mono 必须绿：{:?}",
        verify_loose(&m).err()
    );
}

/// 定向回归（语料形态 2「库解析函数作值」）：两语料本体（`io.println` 原生成员 /
/// `json.parse`、`parse` 纯 yx 导出作值）post-mono `verify_loose` 必须绿。
#[test]
fn test_inner_isolation_library_fn_as_value_post_mono_pass() {
    // Arrange：语料本体（include_str! 绑定语料，避免夹具与语料漂移）
    let member = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/yaoxiang/03-modules/native_fn_member_value.yx"
    ));
    let pure = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/yaoxiang/03-modules/pureyx_export_fn_value.yx"
    ));

    // Act
    let member_ir = compile_post_mono("native_fn_member_value.yx", member);
    let pure_ir = compile_post_mono("pureyx_export_fn_value.yx", pure);

    // Assert
    assert!(
        verify_loose(&member_ir).is_ok(),
        "native_fn_member_value post-mono 必须绿：{:?}",
        verify_loose(&member_ir).err()
    );
    assert!(
        verify_loose(&pure_ir).is_ok(),
        "pureyx_export_fn_value post-mono 必须绿：{:?}",
        verify_loose(&pure_ir).err()
    );
}

/// 定向回归（真闭包卫生，save/restore 契约的正向守卫）：lambda 表达式作实参时
/// 生成 `closure_N`，它的槽位表只含本帧 —— 外层 `main` 的具名局部不得混进去。
///
/// 它是负控（变异驱动）的正向对应：把 `NestedBodyGuard` 的「摘走父帧槽位表」
/// 改坏，本测试必须红。
#[test]
fn test_inner_isolation_nested_lambda_frames_isolated() {
    // Arrange
    let source = r#"
use std.io
use std.list

main: () -> Void = {
    aa = 1
    bb = 2
    cc = 3
    ys = list.map([1, 2, 3], (x: Int) => {
        x + 1
    })
    r = list.len(ys)
    io.println("done")
}

main()
"#;

    // Act
    let m = compile_post_mono("nested_lambda.yx", source);

    // Assert：夹具必须真的生成嵌套体（否则判据空转），且 post-mono 绿
    assert!(
        m.functions.iter().any(|f| f.name.starts_with("closure_")),
        "夹具必须真的生成 closure_N（否则本条判据空转）；实际函数表：{:?}",
        m.functions.iter().map(|f| &f.name).collect::<Vec<_>>()
    );
    assert!(
        verify_loose(&m).is_ok(),
        "真闭包与外层各自一帧，post-mono 必须绿：{:?}",
        verify_loose(&m).err()
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
/// 全语料真实生成序列的统一驱动器：逐文件走 lex→parse→typecheck→ir_gen，
/// 对产出的 ModuleIR 调用给定校验函数并收集违规。返回 (files,fns,instrs,failures)。
fn run_corpus_verifier(
    verify_fn: fn(&ModuleIR) -> Result<(), VerifyError>
) -> (usize, usize, usize, Vec<String>) {
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
        let type_result = checker.check_module(
            &parsed.module,
            &TypeLevelNamespace::scan_one(&parsed.module),
        );
        if !type_result.diagnostics.is_empty() {
            continue;
        }
        let mut generator = AstToIrGenerator::new_with_type_result(
            &type_result,
            ModuleRegistry::with_std(),
            None,
            TypeLevelNamespace::scan_one(&parsed.module),
        );
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

        if let Err(e) = verify_fn(&module_ir) {
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
    (files_checked, fns_checked, instrs_checked, failures)
}

/// 全语料（tests/yaoxiang/ + src/std/tests/，剔 skip / compile-error）产出的
/// ModuleIR 必须过 verify_loose，且校验面非空。
#[test]
fn test_verify_loose_corpus_green() {
    let (files_checked, fns_checked, instrs_checked, failures) = run_corpus_verifier(verify_loose);

    assert!(
        files_checked > 200 && fns_checked > 0 && instrs_checked > 0,
        "校验面异常收窄（files={files_checked}, fns={fns_checked}, instrs={instrs_checked}）——判据失效"
    );
    assert!(
        failures.is_empty(),
        "verify_loose 在现有 IR 上跑红（files={files_checked} fns={fns_checked} instrs={instrs_checked}）：\n{}",
        failures.join("\n")
    );
}

/// 全语料产出的 ModuleIR 必须过 verify_ssa（7f.1 生产 SSA 后的门禁），
/// 且校验面非空——与 verify_loose 同样防「测试没跑到」的假绿。
#[test]
fn test_verify_ssa_corpus_green() {
    let (files_checked, fns_checked, instrs_checked, failures) = run_corpus_verifier(verify_ssa);

    assert!(
        files_checked > 200 && fns_checked > 0 && instrs_checked > 0,
        "校验面异常收窄（files={files_checked}, fns={fns_checked}, instrs={instrs_checked}）——判据失效"
    );
    assert!(
        failures.is_empty(),
        "verify_ssa 在生产语料上跑红（files={files_checked} fns={fns_checked} instrs={instrs_checked}）：\n{}",
        failures.join("\n")
    );
}

// =====================
// 不变量 8：量词事实自洽（D62 / issue #533）
// =====================

/// 不变量 8 反例：**临时槽**不得携带 q=0 事实——量词是声明绑定的事实，
/// 没有具名绑定的槽携带它就是投影串槽（静默错配）。
#[test]
fn test_quantity_zero_on_temp_slot_is_rejected() {
    // Arrange — 参数区外的一个临时槽被写入 q=0
    let mut slot = temp_slot(MonoType::Int(64));
    slot.quantity = Some(crate::ast::Quantity::Zero);
    let m = module_with(vec![func(
        "f",
        vec![MonoType::Int(64)],
        MonoType::Void,
        vec![temp_slot(MonoType::Int(64)), slot],
        vec![ret_void()],
    )]);

    // Act
    let result = verify_loose(&m);

    // Assert
    assert_eq!(
        violations_of(&result, InvariantKind::QuantityFact),
        1,
        "临时槽带 q=0 必须被量词事实自洽判据拒绝；实际: {result:?}"
    );
}

/// 不变量 8 正例：参数区的**具名**槽携带 q=0 是合法形态（`(N: Int) -> (n: N)` 的 N）。
#[test]
fn test_quantity_zero_on_named_param_slot_passes() {
    // Arrange — 参数槽 0 具名且带 q=0
    let mut param_slot = named_slot("N", MonoType::Int(64));
    param_slot.quantity = Some(crate::ast::Quantity::Zero);
    let m = module_with(vec![func(
        "f",
        vec![MonoType::Int(64)],
        MonoType::Void,
        vec![param_slot],
        vec![ret_void()],
    )]);

    // Act
    let result = verify_loose(&m);

    // Assert
    assert_eq!(
        violations_of(&result, InvariantKind::QuantityFact),
        0,
        "参数区具名槽的 q=0 是合法绑定事实；实际: {result:?}"
    );
}

// =====================
// 不变量 3：Phi 一致性（仅 Ssa；7b.1a 两条 + 7b.1b 三条残项）
// =====================

/// 二前驱 + 汇合块的最小模块：块 0 与块 1 的 `successors` 都指向块 label 3，
/// 块 3 放给定 Phi ⇒ 该块前驱集 = {0, 1}（块级，非指令下标）。
fn two_pred_phi_module(
    phi: Instruction,
    values: Vec<ValueInfo>,
) -> ModuleIR {
    let f = func_with_blocks(
        "f",
        vec![],
        MonoType::Void,
        vec![],
        values,
        vec![
            block(0, vec![jmp(2)], vec![3]),
            block(1, vec![jmp(2)], vec![3]),
            block(3, vec![phi, ret_void()], vec![]),
        ],
    );
    module_with(vec![f])
}

/// SSA 值形态的最小双前驱菱形：入口块 0 条件跳到块 1/2，两分支各定义
/// ValueId(1)/ValueId(2)，块 3 放给定 Phi 并 Ret 其 dst。
/// 所有 Value 单定义、定义块在 values 中登记；用于 SSA 判据的正反例。
fn value_load(
    id: u32,
    n: i128,
) -> Instruction {
    Instruction::Load {
        dst: ssa_value(id),
        src: int_const(n),
        span: Span::dummy(),
    }
}

fn value_ret(id: u32) -> Instruction {
    Instruction::Ret {
        value: Some(ssa_value(id)),
        span: Span::dummy(),
    }
}

fn two_pred_value_module(
    phi: Instruction,
    values: Vec<ValueInfo>,
) -> ModuleIR {
    let f = func_with_blocks(
        "f",
        vec![],
        MonoType::Void,
        vec![temp_slot(MonoType::Int(64))],
        values,
        vec![
            block(
                0,
                vec![
                    value_load(3, 1),
                    Instruction::JmpIf {
                        cond: ssa_value(3),
                        target: 6,
                        span: Span::dummy(),
                    },
                ],
                vec![1, 2],
            ),
            block(1, vec![value_load(1, 1), jmp(6)], vec![3]),
            block(2, vec![value_load(2, 2), jmp(6)], vec![3]),
            block(3, vec![phi, value_ret(0)], vec![]),
        ],
    );
    module_with(vec![f])
}

fn two_pred_values() -> Vec<ValueInfo> {
    vec![
        value_info(MonoType::Int(64), 3),
        value_info(MonoType::Int(64), 1),
        value_info(MonoType::Int(64), 2),
        value_info(MonoType::Bool, 0),
    ]
}

/// 正例：菱形两块各定义 incoming，Phi 在汇合块读取，全部值单定义。
#[test]
fn test_ssa_phi_with_matching_predecessors_passes() {
    // Arrange
    let phi = phi_instr(ssa_value(0), vec![(1, ssa_value(1)), (2, ssa_value(2))]);
    let m = two_pred_value_module(phi, two_pred_values());

    // Act
    let r = verify_ssa(&m);

    // Assert
    assert!(
        r.is_ok(),
        "前驱集与定义均合法的 Phi 不应产生违规：{:?}",
        r.err().map(|e| e.to_string())
    );
}

#[test]
fn test_ssa_phi_with_matching_types_passes() {
    // Arrange：V1=Int(64)、V2=Int(64)（SSA 值表要求同型），定义块真实。
    let phi = phi_instr(ssa_value(0), vec![(1, ssa_value(1)), (2, ssa_value(2))]);
    let m = two_pred_value_module(
        phi,
        vec![
            value_info(MonoType::Int(64), 3),
            value_info(MonoType::Int(64), 1),
            value_info(MonoType::Int(64), 2),
            value_info(MonoType::Bool, 0),
        ],
    );

    // Act
    let r = verify_ssa(&m);

    // Assert
    assert!(
        r.is_ok(),
        "逐输入类型相容的 Phi 不应产生违规：{:?}",
        r.err().map(|e| e.to_string())
    );
}

/// 7b.1a 反例：Phi 的 dst 不是 Operand::Value，必报 1 项 PhiConsistency。
#[test]
fn test_ssa_phi_with_slot_dst_fails() {
    // Arrange：dst 落存储槽 Local(0)，不是 SSA 值；前驱集与 incoming 对齐，
    // 使本测试只命中 dst 判据（单缺陷可归因）。
    let phi = phi_instr(
        Operand::Local(0),
        vec![(0, ssa_value(1)), (1, ssa_value(2))],
    );
    let f = func_with_blocks(
        "f",
        vec![],
        MonoType::Void,
        vec![temp_slot(MonoType::Int(64))],
        Vec::new(),
        vec![
            block(0, vec![jmp(2)], vec![3]),
            block(1, vec![jmp(2)], vec![3]),
            block(3, vec![phi, ret_void()], vec![]),
        ],
    );
    let m = module_with(vec![f]);

    // Act
    let r = verify_ssa(&m);
    let detail = r.as_ref().err().map(|e| e.to_string()).unwrap_or_default();

    // Assert
    assert_eq!(
        violations_of(&r, InvariantKind::PhiConsistency),
        1,
        "非 Value 的 Phi dst 应恰好报 1 项 PhiConsistency 违规：{detail}"
    );
}

/// 7b.1a 反例：incoming 的 label 未严格升序，必报 1 项 PhiConsistency
/// （label 集合仍等于前驱集 {0,1}，使本测试只命中升序判据）。
#[test]
fn test_ssa_phi_incoming_not_ascending_fails() {
    // Arrange：label 顺序为 1、0（降序但集合相同）
    let phi = phi_instr(ssa_value(0), vec![(1, ssa_value(1)), (0, ssa_value(2))]);
    let m = two_pred_phi_module(phi, Vec::new());

    // Act
    let r = verify_ssa(&m);
    let detail = r.as_ref().err().map(|e| e.to_string()).unwrap_or_default();

    // Assert
    assert_eq!(
        violations_of(&r, InvariantKind::PhiConsistency),
        1,
        "未升序的 Phi incoming 应恰好报 1 项 PhiConsistency 违规：{detail}"
    );
}

/// 7b.1b 反例（前驱数）：incoming 少一个前驱 ⇒ 必报 1 项 PhiConsistency。
#[test]
fn test_ssa_phi_predecessor_count_mismatch_fails() {
    // Arrange：块 3 前驱集 {0,1}（2 个），incoming 只有 label 0（1 个）
    let phi = phi_instr(ssa_value(0), vec![(0, ssa_value(1))]);
    let m = two_pred_phi_module(phi, Vec::new());

    // Act
    let r = verify_ssa(&m);
    let detail = r.as_ref().err().map(|e| e.to_string()).unwrap_or_default();

    // Assert：计数不等只报计数（集合差异同一根因，不双报）
    assert_eq!(
        violations_of(&r, InvariantKind::PhiConsistency),
        1,
        "incoming 数与前驱数不等应恰好报 1 项 PhiConsistency：{detail}"
    );
    assert!(
        detail.contains("前驱数"),
        "归因消息应点名前驱数差异：{detail}"
    );
}

/// 7b.1b 反例（前驱集合）：incoming 数与判决相同但 label 集合不同
/// ⇒ 必报 1 项 PhiConsistency。
#[test]
fn test_ssa_phi_predecessor_set_mismatch_fails() {
    // Arrange：块 3 前驱集 {0,1}，incoming label 集合 {0,2}（数量相同）
    let phi = phi_instr(ssa_value(0), vec![(0, ssa_value(1)), (2, ssa_value(2))]);
    let m = two_pred_phi_module(phi, Vec::new());

    // Act
    let r = verify_ssa(&m);
    let detail = r.as_ref().err().map(|e| e.to_string()).unwrap_or_default();

    // Assert
    assert_eq!(
        violations_of(&r, InvariantKind::PhiConsistency),
        1,
        "incoming label 集合与前驱集合不一致应恰好报 1 项 PhiConsistency：{detail}"
    );
    assert!(
        detail.contains("前驱集合"),
        "归因消息应点名前驱集合差异：{detail}"
    );
}

/// 7b.1b 反例（类型一致）：全部已知且 Int/Bool 冲突 ⇒ 1 项 PhiConsistency。
#[test]
fn test_ssa_phi_incoming_type_mismatch_fails() {
    // Arrange：V1=Int(64)、V2=Bool，定义块真实。
    let phi = phi_instr(ssa_value(0), vec![(1, ssa_value(1)), (2, ssa_value(2))]);
    let m = two_pred_value_module(
        phi,
        vec![
            value_info(MonoType::Int(64), 3),
            value_info(MonoType::Int(64), 1),
            value_info(MonoType::Bool, 2),
            value_info(MonoType::Bool, 0),
        ],
    );

    // Act
    let r = verify_ssa(&m);
    let detail = r.as_ref().err().map(|e| e.to_string()).unwrap_or_default();

    // Assert
    assert_eq!(
        violations_of(&r, InvariantKind::PhiConsistency),
        1,
        "incoming 类型不一致应恰好报 1 项 PhiConsistency：{detail}"
    );
    assert!(
        detail.contains("类型不一致"),
        "归因消息应点名类型不一致：{detail}"
    );
    assert!(
        detail.contains("前驱 1") && detail.contains("前驱 2"),
        "归因消息应点名冲突前驱 label：{detail}"
    );
}

/// 7b.1b 反例（无块结构）：Phi 出现在模块 init 指令流（无基本块）⇒ 显式报违规。
#[test]
fn test_ssa_phi_in_init_without_blocks_is_reported() {
    // Arrange：init 里放一条 Phi（模块 init 无 BasicBlock，前驱集不可判定）
    let mut m = module_with(vec![]);
    m.init = vec![phi_instr(ssa_value(0), vec![(0, ssa_value(1))])];

    // Act
    let r = verify_ssa(&m);
    let detail = r.as_ref().err().map(|e| e.to_string()).unwrap_or_default();

    // Assert
    assert_eq!(
        violations_of(&r, InvariantKind::PhiConsistency),
        1,
        "init 里的 Phi 应显式报 1 项「无基本块结构」：{detail}"
    );
    assert!(
        detail.contains("无基本块结构"),
        "归因消息应点名无基本块结构：{detail}"
    );
}
// =====================
// 7f.2：verify_ssa 新判据（Value 唯一性 / 支配 / defining_block / 悬空值）
// =====================

/// 正例：一个 Value 单定义、使用在其定义之后，SSA 绿。
#[test]
fn test_ssa_value_single_definition_passes() {
    // Arrange
    let f = func_with_blocks(
        "f",
        vec![],
        MonoType::Void,
        vec![],
        vec![value_info(MonoType::Int(64), 0)],
        vec![block(0, vec![value_load(0, 1), value_ret(0)], vec![])],
    );

    // Act
    let r = verify_ssa(&module_with(vec![f]));

    // Assert
    assert!(
        r.is_ok(),
        "单定义且使用受支配的 Value 不应报错：{:?}",
        r.err().map(|e| e.to_string())
    );
}

/// 反例：同一 ValueId 在块内定义两次，必报 UniqueDefinition。
#[test]
fn test_ssa_value_double_definition_fails() {
    // Arrange
    let f = func_with_blocks(
        "f",
        vec![],
        MonoType::Void,
        vec![],
        vec![value_info(MonoType::Int(64), 0)],
        vec![block(
            0,
            vec![value_load(0, 1), value_load(0, 2), value_ret(0)],
            vec![],
        )],
    );

    // Act
    let r = verify_ssa(&module_with(vec![f]));

    // Assert
    assert_eq!(
        violations_of(&r, InvariantKind::UniqueDefinition),
        1,
        "同一 ValueId 两次定义应报 1 项 UniqueDefinition：{:?}",
        r.err().map(|e| e.to_string())
    );
}

/// 反例：Value 只在一条分支定义、汇合块使用——使用不受定义支配。
#[test]
fn test_ssa_value_use_not_dominated_fails() {
    // Arrange：块 1 定义 v0，块 2 不定义，块 3 使用 v0
    let f = func_with_blocks(
        "f",
        vec![],
        MonoType::Void,
        vec![],
        vec![
            value_info(MonoType::Int(64), 1),
            value_info(MonoType::Bool, 0),
        ],
        vec![
            block(
                0,
                vec![
                    value_load(1, 1),
                    Instruction::JmpIf {
                        cond: ssa_value(1),
                        target: 5,
                        span: Span::dummy(),
                    },
                ],
                vec![1, 2],
            ),
            block(1, vec![value_load(0, 1), jmp(5)], vec![3]),
            block(2, vec![jmp(5)], vec![3]),
            block(3, vec![value_ret(0)], vec![]),
        ],
    );

    // Act
    let r = verify_ssa(&module_with(vec![f]));

    // Assert
    assert_eq!(
        violations_of(&r, InvariantKind::Dominance),
        1,
        "只有一条路径定义的值在汇合块使用应报 Dominance：{:?}",
        r.err().map(|e| e.to_string())
    );
}

/// Phi incoming 源未受支配夹具：v0 只在块 2 定义，块 3 的 Phi 同时从
/// 前驱 1、2 读取它；前驱 1 出口处 v0 没有支配定义。
fn phi_source_undominated_module() -> ModuleIR {
    let phi = phi_instr(ssa_value(1), vec![(1, ssa_value(0)), (2, ssa_value(0))]);
    let f = func_with_blocks(
        "f",
        vec![],
        MonoType::Void,
        vec![],
        vec![
            value_info(MonoType::Int(64), 2),
            value_info(MonoType::Int(64), 3),
            value_info(MonoType::Bool, 0),
        ],
        vec![
            block(
                0,
                vec![
                    value_load(2, 1),
                    Instruction::JmpIf {
                        cond: ssa_value(2),
                        target: 5,
                        span: Span::dummy(),
                    },
                ],
                vec![1, 2],
            ),
            block(1, vec![jmp(5)], vec![3]),
            block(2, vec![value_load(0, 1), jmp(5)], vec![3]),
            block(3, vec![phi, value_ret(1)], vec![]),
        ],
    );
    module_with(vec![f])
}

/// 反例：Phi incoming 的源只在另一前驱定义——在所选前驱出口不受支配。
#[test]
fn test_ssa_phi_incoming_source_not_dominated_fails() {
    // Arrange：v0 只在块 2 定义，Phi 从前驱 1、2 读取；helpers 见上。
    let m = phi_source_undominated_module();

    // Act
    let r = verify_ssa(&m);

    // Assert
    assert_eq!(
        violations_of(&r, InvariantKind::Dominance),
        1,
        "Phi incoming 源在前驱出口未受支配应报 Dominance：{:?}",
        r.err().map(|e| e.to_string())
    );
}

/// 反例：ValueInfo.defining_block 与真实定义块不一致，必报登记类违规。
#[test]
fn test_ssa_defining_block_mismatch_fails() {
    // Arrange：v0 实际定义在块 0，元数据却写块 1
    let f = func_with_blocks(
        "f",
        vec![],
        MonoType::Void,
        vec![],
        vec![value_info(MonoType::Int(64), 1)],
        vec![block(0, vec![value_load(0, 1), ret_void()], vec![])],
    );

    // Act
    let r = verify_ssa(&module_with(vec![f]));
    let detail = r.as_ref().err().map(|e| e.to_string()).unwrap_or_default();

    // Assert
    assert_eq!(
        violations_of(&r, InvariantKind::UniqueDefinition),
        1,
        "defining_block 不一致应报 1 项 UniqueDefinition：{detail}"
    );
    assert!(
        detail.contains("defining_block"),
        "归因消息应点名 defining_block：{detail}"
    );
}

/// 正例：无定义、无使用的悬空 ValueInfo 条目合法。
#[test]
fn test_ssa_dangling_unused_value_passes() {
    // Arrange
    let f = func_with_blocks(
        "f",
        vec![],
        MonoType::Void,
        vec![],
        vec![value_info(MonoType::Int(64), 0)],
        vec![block(0, vec![ret_void()], vec![])],
    );

    // Act
    let r = verify_ssa(&module_with(vec![f]));

    // Assert
    assert!(
        r.is_ok(),
        "无定义无使用的悬空条目必须合法：{:?}",
        r.err().map(|e| e.to_string())
    );
}

/// 正例：无定义、仅不可达块使用的悬空条目合法。
#[test]
fn test_ssa_dangling_only_unreachable_use_passes() {
    // Arrange：块 1 不可达，使用无定义的 Value0
    let f = func_with_blocks(
        "f",
        vec![],
        MonoType::Void,
        vec![],
        vec![value_info(MonoType::Int(64), 0)],
        vec![
            block(0, vec![ret_void()], vec![]),
            block(1, vec![value_ret(0)], vec![]),
        ],
    );

    // Act
    let r = verify_ssa(&module_with(vec![f]));

    // Assert
    assert!(
        r.is_ok(),
        "仅不可达块使用的无定义 Value 必须豁免：{:?}",
        r.err().map(|e| e.to_string())
    );
}

/// 反例：可达块使用无定义的 Value，必报 Dominance。
#[test]
fn test_ssa_reachable_use_without_definition_fails() {
    // Arrange
    let f = func_with_blocks(
        "f",
        vec![],
        MonoType::Void,
        vec![],
        vec![value_info(MonoType::Int(64), 0)],
        vec![block(0, vec![value_ret(0)], vec![])],
    );

    // Act
    let r = verify_ssa(&module_with(vec![f]));

    // Assert
    assert_eq!(
        violations_of(&r, InvariantKind::Dominance),
        1,
        "可达使用无定义值应报 1 项 Dominance：{:?}",
        r.err().map(|e| e.to_string())
    );
}

/// 三前驱菱形：块 0 分叉 1/2，块 2 再分叉 3；块 4 放 Phi 并 Ret。
/// v1=Int(64)、v2=Bool、v3=TypeRef("?")，用于钉住「跳过未知后已知冲突仍报」。
fn three_pred_value_module(
    phi: Instruction,
    values: Vec<ValueInfo>,
) -> ModuleIR {
    let f = func_with_blocks(
        "f",
        vec![],
        MonoType::Void,
        vec![temp_slot(MonoType::Int(64))],
        values,
        vec![
            block(
                0,
                vec![
                    value_load(4, 1),
                    Instruction::JmpIf {
                        cond: ssa_value(4),
                        target: 4,
                        span: Span::dummy(),
                    },
                ],
                vec![1, 2],
            ),
            block(1, vec![value_load(1, 1), jmp(9)], vec![4]),
            block(
                2,
                vec![
                    value_load(5, 1),
                    value_load(2, 0),
                    Instruction::JmpIf {
                        cond: ssa_value(5),
                        target: 9,
                        span: Span::dummy(),
                    },
                ],
                vec![3, 4],
            ),
            block(3, vec![value_load(3, 1), jmp(9)], vec![4]),
            block(4, vec![phi, value_ret(0)], vec![]),
        ],
    );
    module_with(vec![f])
}

#[test]
fn test_ssa_phi_known_conflict_with_unknown_still_fails() {
    // Arrange：incoming 依次 Int(64)、Bool、未知 ?；两个已知输入冲突。
    let phi = phi_instr(
        ssa_value(0),
        vec![(1, ssa_value(1)), (2, ssa_value(2)), (3, ssa_value(3))],
    );
    let m = three_pred_value_module(
        phi,
        vec![
            value_info(MonoType::Int(64), 4),
            value_info(MonoType::Int(64), 1),
            value_info(MonoType::Bool, 2),
            value_info(
                MonoType::TypeRef(Nominal::flat("?").expect("'?' is a valid bare type name")),
                3,
            ),
            value_info(MonoType::Bool, 0),
            value_info(MonoType::Bool, 2),
        ],
    );

    // Act
    let r = verify_ssa(&m);
    let detail = r.as_ref().err().map(|e| e.to_string()).unwrap_or_default();

    // Assert
    assert_eq!(
        violations_of(&r, InvariantKind::PhiConsistency),
        1,
        "未知输入不得掩盖已知 Int/Bool 冲突：{detail}"
    );
    assert!(
        detail.contains("类型不一致"),
        "归因消息应点名类型不一致：{detail}"
    );
    assert!(
        detail.contains("前驱 1") && detail.contains("前驱 2"),
        "归因消息应点名冲突前驱 label：{detail}"
    );
}

/// 正例：未知输入被跳过，只剩一个已知输入时不构成冲突。
#[test]
fn test_ssa_phi_single_known_plus_unknown_passes() {
    // Arrange：v1=Int(64)、v2=显式未知 ?；v1 与 v2 都定义在真实块。
    let phi = phi_instr(ssa_value(0), vec![(1, ssa_value(1)), (2, ssa_value(2))]);
    let m = two_pred_value_module(
        phi,
        vec![
            value_info(MonoType::Int(64), 3),
            value_info(MonoType::Int(64), 1),
            value_info(
                MonoType::TypeRef(Nominal::flat("?").expect("'?' is a valid bare type name")),
                2,
            ),
            value_info(MonoType::Bool, 0),
        ],
    );

    // Act
    let r = verify_ssa(&m);

    // Assert
    assert!(
        r.is_ok(),
        "未知被跳过且只剩一个已知输入时不应报类型冲突：{:?}",
        r.err().map(|e| e.to_string())
    );
}

/// 正例：未知被跳过，两个同型已知输入之间仍不冲突。
#[test]
fn test_ssa_phi_same_known_plus_unknown_passes() {
    // Arrange：v1/v2=Int(64)、v3=显式未知 ?；Phi 覆盖三个前驱。
    let phi = phi_instr(
        ssa_value(0),
        vec![(1, ssa_value(1)), (2, ssa_value(2)), (3, ssa_value(3))],
    );
    let m = three_pred_value_module(
        phi,
        vec![
            value_info(MonoType::Int(64), 4),
            value_info(MonoType::Int(64), 1),
            value_info(MonoType::Int(64), 2),
            value_info(
                MonoType::TypeRef(Nominal::flat("?").expect("'?' is a valid bare type name")),
                3,
            ),
            value_info(MonoType::Bool, 0),
            value_info(MonoType::Bool, 2),
        ],
    );

    // Act
    let r = verify_ssa(&m);

    // Assert
    assert!(
        r.is_ok(),
        "同型已知输入 + 未知应绿（未知被跳过，不制造冲突）：{:?}",
        r.err().map(|e| e.to_string())
    );
}

/// 反例：Phi 全部已知且 Int/Bool 冲突，必报 PhiConsistency。
#[test]
fn test_ssa_phi_known_type_conflict_fails() {
    // Arrange
    let phi = phi_instr(ssa_value(0), vec![(1, ssa_value(1)), (2, ssa_value(2))]);
    let m = two_pred_value_module(
        phi,
        vec![
            value_info(MonoType::Int(64), 3),
            value_info(MonoType::Int(64), 1),
            value_info(MonoType::Bool, 2),
            value_info(MonoType::Bool, 0),
        ],
    );

    // Act
    let r = verify_ssa(&m);
    let detail = r.as_ref().err().map(|e| e.to_string()).unwrap_or_default();

    // Assert
    assert_eq!(
        violations_of(&r, InvariantKind::PhiConsistency),
        1,
        "全部已知的 Int/Bool Phi 输入应报 1 项 PhiConsistency：{detail}"
    );
    assert!(
        detail.contains("类型不一致"),
        "归因消息应点名类型不一致：{detail}"
    );
    assert!(
        detail.contains("前驱 1") && detail.contains("前驱 2"),
        "归因消息应点名冲突前驱 label：{detail}"
    );
}

/// 门禁负例自证：六类坏 IR 必须分别被 verify_ssa 拒绝且归到正确不变量。
/// 这是 check-ir-verifier.sh 第三条判据的直驱测试——破坏任一断言都会让脚本红。
#[test]
fn test_verify_ssa_negative_controls() {
    // Arrange：六个独立合成 ModuleIR，每个只制造一种 SSA 违规。
    let double_definition = module_with(vec![func_with_blocks(
        "double_definition",
        vec![],
        MonoType::Void,
        vec![],
        vec![value_info(MonoType::Int(64), 0)],
        vec![block(
            0,
            vec![value_load(0, 1), value_load(0, 2), value_ret(0)],
            vec![],
        )],
    )]);

    let undominated_use = module_with(vec![func_with_blocks(
        "undominated_use",
        vec![],
        MonoType::Void,
        vec![],
        vec![
            value_info(MonoType::Int(64), 1),
            value_info(MonoType::Bool, 0),
        ],
        vec![
            block(
                0,
                vec![
                    value_load(1, 1),
                    Instruction::JmpIf {
                        cond: ssa_value(1),
                        target: 5,
                        span: Span::dummy(),
                    },
                ],
                vec![1, 2],
            ),
            block(1, vec![value_load(0, 1), jmp(5)], vec![3]),
            block(2, vec![jmp(5)], vec![3]),
            block(3, vec![value_ret(0)], vec![]),
        ],
    )]);

    let phi_conflict_with_unknown = three_pred_value_module(
        phi_instr(
            ssa_value(0),
            vec![(1, ssa_value(1)), (2, ssa_value(2)), (3, ssa_value(3))],
        ),
        vec![
            value_info(MonoType::Int(64), 4),
            value_info(MonoType::Int(64), 1),
            value_info(MonoType::Bool, 2),
            value_info(
                MonoType::TypeRef(Nominal::flat("?").expect("'?' is a valid bare type name")),
                3,
            ),
            value_info(MonoType::Bool, 0),
            value_info(MonoType::Bool, 2),
        ],
    );

    let defining_block_mismatch = module_with(vec![func_with_blocks(
        "defining_block_mismatch",
        vec![],
        MonoType::Void,
        vec![],
        vec![value_info(MonoType::Int(64), 1)],
        vec![block(0, vec![value_load(0, 1), ret_void()], vec![])],
    )]);

    let reachable_undefined = module_with(vec![func_with_blocks(
        "reachable_undefined",
        vec![],
        MonoType::Void,
        vec![],
        vec![value_info(MonoType::Int(64), 0)],
        vec![block(0, vec![value_ret(0)], vec![])],
    )]);

    let phi_source_undominated = phi_source_undominated_module();

    // Act
    let double_err = verify_ssa(&double_definition).expect_err("Value 双定义必须报错");
    let undominated_err = verify_ssa(&undominated_use).expect_err("未受支配使用必须报错");
    let phi_err =
        verify_ssa(&phi_conflict_with_unknown).expect_err("含未知输入的已知 Phi 冲突必须报错");
    let defining_err =
        verify_ssa(&defining_block_mismatch).expect_err("defining_block 不一致必须报错");
    let undefined_err = verify_ssa(&reachable_undefined).expect_err("可达使用无定义必须报错");
    let phi_source_err =
        verify_ssa(&phi_source_undominated).expect_err("Phi incoming 源未受支配必须报错");

    // Assert
    assert_eq!(
        double_err.violations.len(),
        1,
        "Value 双定义夹具只应产生 1 项总违规：{double_err}"
    );
    assert_eq!(
        undominated_err.violations.len(),
        1,
        "未受支配使用夹具只应产生 1 项总违规：{undominated_err}"
    );
    assert_eq!(
        phi_err.violations.len(),
        1,
        "Phi 已知冲突夹具只应产生 1 项总违规：{phi_err}"
    );
    assert_eq!(
        defining_err.violations.len(),
        1,
        "defining_block 不一致夹具只应产生 1 项总违规：{defining_err}"
    );
    assert_eq!(
        undefined_err.violations.len(),
        1,
        "可达使用无定义夹具只应产生 1 项总违规：{undefined_err}"
    );
    assert_eq!(
        phi_source_err.violations.len(),
        1,
        "Phi incoming 源未受支配夹具只应产生 1 项总违规：{phi_source_err}"
    );
    assert_eq!(
        double_err
            .violations
            .iter()
            .filter(|v| v.invariant == InvariantKind::UniqueDefinition)
            .count(),
        1,
        "Value 双定义应报 1 项 UniqueDefinition：{double_err}"
    );
    assert!(
        double_err.to_string().contains("ValueId(0)"),
        "双定义诊断应点名 ValueId(0)：{double_err}"
    );
    assert_eq!(
        undominated_err
            .violations
            .iter()
            .filter(|v| v.invariant == InvariantKind::Dominance)
            .count(),
        1,
        "未受支配使用应报 1 项 Dominance：{undominated_err}"
    );
    assert!(
        undominated_err.to_string().contains("没有支配"),
        "未受支配诊断应点名无支配定义：{undominated_err}"
    );
    assert_eq!(
        phi_err
            .violations
            .iter()
            .filter(|v| v.invariant == InvariantKind::PhiConsistency)
            .count(),
        1,
        "含未知输入的已知 Phi 冲突应报 1 项 PhiConsistency：{phi_err}"
    );
    assert!(
        phi_err.to_string().contains("前驱 1") && phi_err.to_string().contains("前驱 2"),
        "Phi 冲突诊断应点名冲突前驱 label：{phi_err}"
    );
    assert_eq!(
        defining_err
            .violations
            .iter()
            .filter(|v| v.invariant == InvariantKind::UniqueDefinition)
            .count(),
        1,
        "defining_block 不一致应报 1 项 UniqueDefinition：{defining_err}"
    );
    assert!(
        defining_err.to_string().contains("defining_block"),
        "defining_block 诊断应点名字段：{defining_err}"
    );
    assert_eq!(
        undefined_err
            .violations
            .iter()
            .filter(|v| v.invariant == InvariantKind::Dominance)
            .count(),
        1,
        "可达使用无定义应报 1 项 Dominance：{undefined_err}"
    );
    assert!(
        undefined_err.to_string().contains("没有支配"),
        "可达无定义诊断应点名无支配定义：{undefined_err}"
    );
    assert_eq!(
        phi_source_err
            .violations
            .iter()
            .filter(|v| v.invariant == InvariantKind::Dominance)
            .count(),
        1,
        "Phi incoming 源未受支配应报 1 项 Dominance：{phi_source_err}"
    );
    assert!(
        phi_source_err
            .to_string()
            .contains("Phi incoming 源 ValueId(0)")
            && phi_source_err.to_string().contains("前驱块 1"),
        "Phi 源支配诊断应点名值与前驱块：{phi_source_err}"
    );
}
