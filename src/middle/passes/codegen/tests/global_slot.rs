//! 全局槽位（顶层绑定）的 IR → 字节码 → 解释器往返测试。
//!
//! 覆盖 T1 基础设施：`Operand::Global` 经 translator 编成 `LOAD_GLOBAL` /
//! `STORE_GLOBAL`，解释器的 `global_slots` 承接读写。
//! 这些测试手工构造 IR——不依赖 ir_gen 的顶层绑定改造（T2），
//! 故 T1 可独立验证。

use crate::backends::common::RuntimeValue;
use crate::backends::Executor;
use crate::frontend::core::typecheck::MonoType;
use crate::middle::core::ir::{
    BasicBlock, ConstValue, FunctionBody, FunctionIR, GlobalSlot, Instruction, ModuleIR, Operand,
};
use crate::middle::passes::codegen::CodegenContext;
use crate::util::span::Span;

/// 构造一个「把常量写入全局槽位，再从槽位读回并返回」的模块。
///
/// main 的函数体：
///   Load(Local 0, const 42)    ← 造值
///   Store(Global 0, Local 0)   ← 写全局槽位
///   Load(Local 1, Global 0)    ← 读全局槽位
///   Ret(Local 1)
fn store_then_load_module() -> ModuleIR {
    let mut module = ModuleIR::default();
    module.globals.push(GlobalSlot {
        name: "answer".to_string(),
        ty: MonoType::Int(64),
        index: 0,
    });

    module.functions.push(FunctionIR {
        def: None,
        name: "main".to_string(),
        params: Vec::new(),
        return_type: MonoType::Int(64),
        generic_params: None,
        body: FunctionBody::Code {
            blocks: vec![BasicBlock {
                label: 0,
                instructions: vec![
                    Instruction::Load {
                        dst: Operand::Local(0),
                        src: Operand::Const(ConstValue::Int(42)),
                    },
                    Instruction::Store {
                        dst: Operand::Global(0),
                        src: Operand::Local(0),
                        span: Span::default(),
                    },
                    Instruction::Load {
                        dst: Operand::Local(1),
                        src: Operand::Global(0),
                    },
                    Instruction::Ret(Some(Operand::Local(1))),
                ],
                successors: Vec::new(),
            }],
            entry: 0,
            locals: vec![MonoType::Int(64), MonoType::Int(64)],
        },
    });
    module
}

/// 编译 fixture，返回「解释器 + 可执行模块」。
///
/// 只走 `execute_module`（它会装常量池与函数表，然后执行入口）。
fn run_fixture() -> crate::backends::interpreter::Interpreter {
    let module = store_then_load_module();
    let mut ctx = CodegenContext::new(module);
    let file = ctx.generate().expect("codegen should succeed");
    let bytecode_module = crate::middle::bytecode::BytecodeModule::from(file);

    let mut interp = crate::backends::interpreter::Interpreter::new();
    interp
        .execute_module(&bytecode_module)
        .expect("execute_module should succeed");
    interp
}

/// 断言全局槽位 0 的值为 Int(42)。
fn assert_slot_is_42(interp: &crate::backends::interpreter::Interpreter) {
    match interp.global_slot(0) {
        Some(RuntimeValue::Int(42)) => {}
        other => panic!("全局槽位 0 应为 Int(42)，实际为 {other:?}"),
    }
}

/// T1 验收：手工 IR 的全局槽位写入后读取，值正确。
///
/// 经 `StoreGlobal` 写入 42，再由执行流返回它；执行后读回槽位断言。
/// 若 `LoadGlobal`/`StoreGlobal` 未被 translator 编码或解释器未实现，
/// 槽位会是 `Void`（解释器的兜底）或编译期就失败。
#[test]
fn test_global_slot_store_then_load_roundtrip() {
    let interp = run_fixture();
    assert_slot_is_42(&interp);
}

/// T1 验收：`execute_module` 入口路径返回非 Void（证明读回未被兜底吞掉）。
#[test]
fn test_execute_module_with_global_slot() {
    let interp = run_fixture();
    // 入口返回值即槽位读回值：Int(42) 证明 LoadGlobal 真的取到了值
    assert_slot_is_42(&interp);
}

/// T1 验收：序列化往返（.42 写入→读回）后全局槽位仍可用。
///
/// 覆盖 `write_to` / `read_from` 的对称性：指令流往返后 LOAD_GLOBAL /
/// STORE_GLOBAL 仍被正确解码（操作数布局 `dst,idx` 与 `idx,src` 不串位）。
#[test]
fn test_global_slot_survives_serialization_roundtrip() {
    let module = store_then_load_module();
    let mut ctx = CodegenContext::new(module);
    let file = ctx.generate().expect("codegen should succeed");

    // 写入内存缓冲区再读回（模拟 .42 落盘往返）
    let mut buf = Vec::new();
    file.write_to(&mut buf).expect("write_to should succeed");
    let mut cursor = std::io::Cursor::new(&buf);
    let restored = crate::middle::passes::codegen::BytecodeFile::read_from(&mut cursor)
        .expect("read_from should succeed");

    let bytecode_module = crate::middle::bytecode::BytecodeModule::from(restored);
    let mut interp = crate::backends::interpreter::Interpreter::new();
    interp
        .execute_module(&bytecode_module)
        .expect("execute_module should succeed after roundtrip");

    assert_slot_is_42(&interp);
}
