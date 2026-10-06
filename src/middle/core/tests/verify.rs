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
use crate::middle::core::ir::{
    BasicBlock, ConstValue, FunctionBody, FunctionIR, Instruction, LocalSlot, ModuleIR, Operand,
};
use crate::middle::core::ir_gen::AstToIrGenerator;
use crate::middle::core::verify::{verify_loose, verify_ssa, InvariantKind, VerifyError};
use crate::util::span::Span;

use super::corpus::{corpus_files, reaches_ir_generation};

// =====================
// 构造助手
// =====================

fn temp_slot(ty: MonoType) -> LocalSlot {
    LocalSlot::temp(ty)
}

fn named_slot(
    name: &str,
    ty: MonoType,
) -> LocalSlot {
    LocalSlot {
        name: Some(name.to_string()),
        ty,
        scope_depth: 0,
    }
}

/// 单块函数：块内指令序即展平序（translator 线性铺平语义的直接对应）。
fn func(
    name: &str,
    params: Vec<MonoType>,
    return_type: MonoType,
    locals: Vec<LocalSlot>,
    instrs: Vec<Instruction>,
) -> FunctionIR {
    FunctionIR {
        name: name.to_string(),
        def: None,
        params,
        return_type,
        generic_params: None,
        body: FunctionBody::Code {
            blocks: vec![BasicBlock {
                label: 0,
                instructions: instrs,
                successors: Vec::new(),
            }],
            entry: 0,
            locals,
        },
    }
}

fn module_with(functions: Vec<FunctionIR>) -> ModuleIR {
    ModuleIR {
        functions,
        ..Default::default()
    }
}

fn int_const(n: i128) -> Operand {
    Operand::Const(ConstValue::Int(n))
}

fn move_instr(
    dst: usize,
    src: Operand,
) -> Instruction {
    Instruction::Move {
        dst: Operand::Local(dst),
        src,
        span: Span::dummy(),
    }
}

fn ret_void() -> Instruction {
    Instruction::Ret {
        value: None,
        span: Span::dummy(),
    }
}

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

#[test]
fn jump_target_in_range_passes() {
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
    assert!(verify_loose(&module_with(vec![f])).is_ok());
}

#[test]
fn jump_target_out_of_range_fails() {
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
    let r = verify_loose(&module_with(vec![f]));
    assert_eq!(violations_of(&r, InvariantKind::JumpTarget), 1);
}

// =====================
// 不变量 5：槽位越界
// =====================

#[test]
fn local_slot_out_of_bounds_fails() {
    let f = func(
        "f",
        vec![],
        MonoType::Void,
        vec![temp_slot(MonoType::Int(64))],
        vec![move_instr(9, int_const(1)), ret_void()],
    );
    let r = verify_loose(&module_with(vec![f]));
    assert_eq!(violations_of(&r, InvariantKind::SlotBounds), 1);
}

#[test]
fn global_slot_out_of_bounds_fails() {
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
    let r = verify_loose(&module_with(vec![f]));
    assert_eq!(violations_of(&r, InvariantKind::SlotBounds), 1);
}

// =====================
// 不变量 1：use-before-def 支配性
// =====================

#[test]
fn linear_def_before_use_passes() {
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
    assert!(verify_loose(&module_with(vec![f])).is_ok());
}

/// 分支内定义、汇合后使用：JmpIf 为真路径跳过定义点。
/// 判定面对具名槽（temp 槽的 Void 预初始化语义不在支配性判定面内，
/// 见 verify.rs check_dominance 注释）。
#[test]
fn branch_local_def_does_not_dominate_merge_use() {
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
            // 0: 条件（入口定义外的读——Local(0) 未定义先读，支配性另报；
            //     此处专注分支定义不支配汇合使用的形态，故先给 0 一个定义）
            move_instr(0, Operand::Const(ConstValue::Bool(true))),
            // 1: JmpIf Local(0) -> 3（跳过定义点 2）
            Instruction::JmpIf {
                cond: Operand::Local(0),
                target: 3,
                span: Span::dummy(),
            },
            // 2: 定义 Local(1)（仅 fall-through 路径执行）
            move_instr(1, int_const(42)),
            // 3: 汇合点读 Local(1)——定义 2 不支配 3（Dom(3)={0,1,3}）
            move_instr(2, Operand::Local(1)),
            ret_void(),
        ],
    );
    let r = verify_loose(&module_with(vec![f]));
    assert_eq!(violations_of(&r, InvariantKind::Dominance), 1);
}

/// 循环回边：循环体内的自更新读被迭代支配集正确处理（定义在循环头前）。
#[test]
fn loop_back_edge_self_update_passes() {
    let f = func(
        "f",
        vec![],
        MonoType::Void,
        vec![temp_slot(MonoType::Int(64))],
        vec![
            move_instr(0, int_const(0)), // 0: i = 0
            // 1: if !i goto 4
            Instruction::JmpIfNot {
                cond: Operand::Local(0),
                target: 4,
                span: Span::dummy(),
            },
            move_instr(0, Operand::Local(0)), // 2: i = i（自更新，读被 0 支配）
            Instruction::Jmp {
                target: 1,
                span: Span::dummy(),
            }, // 3: 回跳
            ret_void(),                       // 4
        ],
    );
    assert!(verify_loose(&module_with(vec![f])).is_ok());
}

/// 参数槽是入口定义：读参数无需指令级定义点。
#[test]
fn param_slot_read_passes_without_instr_def() {
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
    assert!(verify_loose(&module_with(vec![f])).is_ok());
}

// =====================
// 不变量 6：类型一致
// =====================

/// 可信源：参数槽签名类型 + 常量。Int 参数与 Int 常量双端互容。
#[test]
fn type_consistent_arith_passes() {
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
    assert!(verify_loose(&module_with(vec![f])).is_ok());
}

/// Int 参数与 Bool 常量双端不相容——可信源冲突必报。
#[test]
fn type_inconsistent_arith_fails() {
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
    let r = verify_loose(&module_with(vec![f]));
    assert_eq!(violations_of(&r, InvariantKind::TypeConsistency), 1);
}

/// Ret 常量值与 return_type 不相容必报（return_type 来自签名，可信）。
#[test]
fn ret_const_type_mismatch_fails() {
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
    let r = verify_loose(&module_with(vec![f]));
    assert_eq!(violations_of(&r, InvariantKind::TypeConsistency), 1);
}

/// 非参数槽的 ty 是占位符（ir_gen 不写类型表）：Bool 常量入具名 Int 槽
/// 不报——不是豁免，是判定依据不存在（verify.rs 文件头登记）。
#[test]
fn untyped_local_slot_not_type_checked() {
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
    assert!(verify_loose(&module_with(vec![f])).is_ok());
}

/// 宽度族互容：Int(32) 参数与 Int(64) 常量双端相容（P5 收敛前的现行表示）。
#[test]
fn int_width_family_compatible() {
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
    assert!(verify_loose(&module_with(vec![f])).is_ok());
}

// =====================
// 不变量 2：值定义唯一性（SSA 模式语义）
// =====================

/// 同槽两次定义：Loose 放行（现行语句级回收策略），Ssa 必报——
/// 这是 D38 红性的直接证据：verify_ssa 在现有分配策略下红是设计预期。
#[test]
fn duplicate_definition_loose_passes_ssa_fails() {
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
    assert!(verify_loose(&module_with(vec![mk()])).is_ok());
    let r = verify_ssa(&module_with(vec![mk()]));
    assert_eq!(violations_of(&r, InvariantKind::UniqueDefinition), 1);
}

/// SSA 形态正例：每槽单定义，参数槽不被重写。
#[test]
fn ssa_shaped_ir_passes_verify_ssa() {
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
    assert!(verify_ssa(&module_with(vec![f])).is_ok());
}

// =====================
// 不变量 7：内层隔离
// =====================

/// 外层 MakeClosure 引用的闭包，其自有局部名混入外层 locals = save/restore 违反。
#[test]
fn inner_local_name_leaking_to_outer_fails() {
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
    let r = verify_loose(&module_with(vec![outer, closure]));
    assert_eq!(violations_of(&r, InvariantKind::InnerIsolation), 1);
}

/// 合法情形：闭包自有局部名与外层不相交（同名参数槽位不参与比较）。
#[test]
fn disjoint_inner_locals_pass() {
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
            // 同名但处于参数槽位（idx 0 < params.len()）——遮蔽是合法语义
            named_slot("outer_var", MonoType::Int(64)),
            named_slot("inner_only", MonoType::Int(64)),
        ],
        vec![ret_void()],
    );
    assert!(verify_loose(&module_with(vec![outer, closure])).is_ok());
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
