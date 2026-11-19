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
use crate::frontend::module::registry::ModuleRegistry;
use crate::middle::ir::ir::{ConstValue, FrameId, Instruction, LocalSlot, Operand};
use crate::middle::ir::verify::{verify_loose, verify_ssa, InvariantKind, VerifyError};
use crate::middle::lower::AstToIrGenerator;
use crate::util::span::Span;

use super::corpus::{corpus_files, reaches_ir_generation};
use super::ir_build::{
    func, int_const, module_with, move_instr, named_slot, named_slot_in_frame, ret_void, temp_slot,
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
