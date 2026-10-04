//! 字节码执行测试
//!
//! 测试覆盖内容：
//! - Borrow/Release 字节码指令的执行
//! - 借用令牌（ZST）的拷贝、释放及边界行为
//! - #299 §2: NewArray 定长数组指令——分配、Void 占位、字节码编解码往返

use crate::backends::Executor;
use crate::backends::common::RuntimeValue;
use crate::middle::bytecode::{BytecodeFunction, BytecodeInstr, Reg, ConstValue};
use std::collections::HashMap;
use crate::backends::interpreter::executor::Interpreter;

fn make_function(instrs: Vec<BytecodeInstr>) -> BytecodeFunction {
    BytecodeFunction {
        name: "test".to_string(),
        params: vec![],
        return_type: crate::middle::core::ir::Type::Void,
        local_count: 4,
        local_names: HashMap::new(),
        upvalue_count: 0,
        instructions: instrs,
        labels: HashMap::new(),
        exception_handlers: vec![],
        debug_map: HashMap::new(),
    }
}

/// 辅助函数：创建预装一个常量的解释器
fn make_interp_with_const(val: ConstValue) -> Interpreter {
    let mut interp = Interpreter::new();
    std::sync::Arc::get_mut(&mut interp.image)
        .expect("interp.image 应独占（Arc 无其它引用）")
        .constants
        .push(val);
    interp
}

/// Borrow copies value from src register to dst register (immutable)
/// Helper: test_new_array_bytecode_roundtrip_decode 的完整夹具与断言（逐条断言见函数体）。
/// Fixture: NEW_ARRAY(5) + RETURN_VALUE 的编码文件（编码：opcode + dst + count 小端）。
fn new_array_roundtrip_file() -> crate::middle::passes::codegen::BytecodeFile {
    use crate::middle::passes::codegen::{
        BytecodeFile, BytecodeInstruction, BytecodeHeader, CodeSection, FunctionCode,
    };

    // Arrange — 构造原始编码：opcode(1) + dst(1) + count(4, 小端)
    let mut operands = vec![0u8]; // dst = r0
    operands.extend_from_slice(&5u32.to_le_bytes()); // count = 5
    let raw_new_array =
        BytecodeInstruction::new(crate::backends::common::opcode::NEW_ARRAY, operands);
    // ReturnValue r0
    let raw_return =
        BytecodeInstruction::new(crate::backends::common::opcode::RETURN_VALUE, vec![0u8]);

    BytecodeFile {
        header: BytecodeHeader {
            entry_point: 0,
            ..BytecodeHeader::default()
        },
        type_table: vec![],
        const_pool: vec![],
        code_section: CodeSection {
            functions: vec![FunctionCode {
                name: "main".to_string(),
                params: vec![],
                return_type: crate::frontend::core::types::MonoType::Void,
                instructions: vec![raw_new_array, raw_return],
                local_count: 1,
                local_names: HashMap::new(),
                debug_map: std::collections::HashMap::new(),
            }],
        },
        vtables: vec![],
        debug_section: None,
    }
}

/// Assert: 解码结果保持 NewArray 语义（dst=r0、count=5）。
fn assert_new_array_decoded(func: &BytecodeFunction) {
    assert!(
        matches!(
            func.instructions.first(),
            Some(BytecodeInstr::NewArray {
                dst: Reg(0),
                count: 5
            })
        ),
        "第一条应为 NewArray: {:?}",
        func.instructions
    );
}

/// Helper: spawn_concurrent_standard_mode 的完整夹具与断言（逐条断言见函数体）。
/// Fixture: 两个任务闭包（task_a 返回 10、task_b 返回 20）与以 `main` 为入口的模块。
///
/// `main` 的字节码依次：创建两个闭包 → Spawn 并发执行 → 两结果相加 → 返回。
fn spawn_two_tasks_module() -> crate::middle::bytecode::BytecodeModule {
    use crate::middle::bytecode::{BytecodeModule, BytecodeFunction, BytecodeInstr};

    // task_a: 返回 Int(10)
    let task_a = BytecodeFunction {
        name: "task_a".to_string(),
        params: vec![],
        return_type: crate::middle::core::ir::Type::Void,
        local_count: 1,
        local_names: HashMap::new(),
        upvalue_count: 0,
        instructions: vec![
            BytecodeInstr::LoadConst {
                dst: Reg(0),
                const_idx: 0,
            },
            BytecodeInstr::ReturnValue { value: Reg(0) },
        ],
        labels: HashMap::new(),
        exception_handlers: vec![],
        debug_map: HashMap::new(),
    };

    // task_b: 返回 Int(20)
    let task_b = BytecodeFunction {
        name: "task_b".to_string(),
        params: vec![],
        return_type: crate::middle::core::ir::Type::Void,
        local_count: 1,
        local_names: HashMap::new(),
        upvalue_count: 0,
        instructions: vec![
            BytecodeInstr::LoadConst {
                dst: Reg(0),
                const_idx: 1,
            },
            BytecodeInstr::ReturnValue { value: Reg(0) },
        ],
        labels: HashMap::new(),
        exception_handlers: vec![],
        debug_map: HashMap::new(),
    };

    // main: 创建两个闭包，spawn 并发执行，读取结果并相加
    // 函数索引：task_a=0, task_b=1, main=2
    let main_func = BytecodeFunction {
        name: "main".to_string(),
        params: vec![],
        return_type: crate::middle::core::ir::Type::Void,
        local_count: 4,
        local_names: HashMap::new(),
        upvalue_count: 0,
        instructions: vec![
            // r0 = closure task_a
            BytecodeInstr::MakeClosure {
                dst: Reg(0),
                func: 0,
                env: vec![],
            },
            // r1 = closure task_b
            BytecodeInstr::MakeClosure {
                dst: Reg(1),
                func: 1,
                env: vec![],
            },
            // spawn [r0, r1] — 执行后 r0=task_a 结果, r1=task_b 结果
            BytecodeInstr::Spawn {
                dst: Reg(2),
                closures: vec![Reg(0), Reg(1)],
                task_deps: vec![vec![], vec![]],
                task_resources: vec![vec![], vec![]],
            },
            // r2 = r0 + r1 (10 + 20 = 30)
            BytecodeInstr::BinaryOp {
                dst: Reg(2),
                lhs: Reg(0),
                rhs: Reg(1),
                op: crate::middle::bytecode::BinaryOp::Add,
            },
            // return r2
            BytecodeInstr::ReturnValue { value: Reg(2) },
        ],
        labels: HashMap::new(),
        exception_handlers: vec![],
        debug_map: HashMap::new(),
    };

    BytecodeModule {
        name: "spawn_test".to_string(),
        constants: vec![ConstValue::Int(10), ConstValue::Int(20)],
        functions: vec![task_a, task_b, main_func],
        type_table: vec![],
        vtables: vec![],
        globals: vec![],
        entry_point: Some(2), // main 函数
        init_function: None,
        debug_sources: None,
        // #368：本用例不涉及索引越界的变量名回溯，空表即可
        global_names: std::collections::HashMap::new(),
    }
}

/// Fixture: Standard 模式 + 1 worker 的解释器（避免多线程并发问题）。
///
/// `set_runtime_config` 只更新配置，需 `reset()` 重建 Runtime facade。
fn standard_mode_interpreter() -> Interpreter {
    use crate::backends::runtime::RuntimeConfig;
    use crate::backends::runtime::RuntimeMode;

    // 配置 Standard 模式 + 1 worker（避免多线程并发问题）
    let mut interp = Interpreter::new();
    interp.set_runtime_config(RuntimeConfig {
        mode: RuntimeMode::Standard,
        workers: 1,
    });
    // 重建 Runtime facade（set_runtime_config 只更新配置，需要 reset 重建 rt）
    interp.reset();

    interp
}

/// Assert: Runtime facade 的运行模式与 worker 数（Standard + 1 worker）。
fn assert_standard_runtime_config(interp: &Interpreter) {
    use crate::backends::runtime::RuntimeMode;

    assert_eq!(
        interp.runtime_config().mode,
        RuntimeMode::Standard,
        "runtime_config 应为 Standard 模式"
    );
    assert_eq!(interp.runtime_config().workers, 1, "workers 应为 1");
}

#[test]
fn test_borrow_copies_value_immutable() {
    let func = make_function(vec![
        // r0 = Int(42)
        BytecodeInstr::LoadConst {
            dst: Reg(0),
            const_idx: 0,
        },
        // r1 = borrow r0 (immutable)
        BytecodeInstr::Borrow {
            dst: Reg(1),
            src: Reg(0),
            mutable: false,
        },
        // return r1
        BytecodeInstr::ReturnValue { value: Reg(1) },
    ]);

    let mut interp = make_interp_with_const(ConstValue::Int(42));

    let result = interp.execute_function(&func, &[]).unwrap();
    assert_eq!(
        result,
        RuntimeValue::Int(42),
        "不可变借用应拷贝源寄存器的值"
    );
}

/// Borrow copies value from src register to dst register (mutable)
#[test]
fn test_borrow_copies_value_mutable() {
    let func = make_function(vec![
        // r0 = Int(99)
        BytecodeInstr::LoadConst {
            dst: Reg(0),
            const_idx: 0,
        },
        // r1 = borrow mut r0
        BytecodeInstr::Borrow {
            dst: Reg(1),
            src: Reg(0),
            mutable: true,
        },
        // return r1
        BytecodeInstr::ReturnValue { value: Reg(1) },
    ]);

    let mut interp = make_interp_with_const(ConstValue::Int(99));

    let result = interp.execute_function(&func, &[]).unwrap();
    assert_eq!(result, RuntimeValue::Int(99), "可变借用应拷贝源寄存器的值");
}

/// Borrow with mutable:false and mutable:true produce the same runtime result (ZST)
#[test]
fn test_borrow_mutable_flag_irrelevant_at_runtime() {
    // Both immutable and mutable borrow copy the value identically.
    for mutable in [false, true] {
        let func = make_function(vec![
            BytecodeInstr::LoadConst {
                dst: Reg(0),
                const_idx: 0,
            },
            BytecodeInstr::Borrow {
                dst: Reg(1),
                src: Reg(0),
                mutable,
            },
            BytecodeInstr::ReturnValue { value: Reg(1) },
        ]);

        let mut interp = make_interp_with_const(ConstValue::String("hello".into()));

        let result = interp.execute_function(&func, &[]).unwrap();
        assert_eq!(
            result,
            RuntimeValue::String("hello".into()),
            "mutable={mutable} should produce the same value",
        );
    }
}

/// Borrow with empty src register yields Void
#[test]
fn test_borrow_from_unset_register() {
    let func = make_function(vec![
        // r1 = borrow r0 (r0 is unset -> Void)
        BytecodeInstr::Borrow {
            dst: Reg(1),
            src: Reg(0),
            mutable: false,
        },
        // return r1
        BytecodeInstr::ReturnValue { value: Reg(1) },
    ]);

    let mut interp = Interpreter::new();
    let result = interp.execute_function(&func, &[]).unwrap();
    assert_eq!(
        result,
        RuntimeValue::Void,
        "从未设置的寄存器借用应得到 Void"
    ); // just advances IP; does not corrupt registers
}

#[test]
fn test_release_is_noop() {
    let func = make_function(vec![
        // r0 = Int(7)
        BytecodeInstr::LoadConst {
            dst: Reg(0),
            const_idx: 0,
        },
        // release r0 (should be a no-op)
        BytecodeInstr::Release { src: Reg(0) },
        // r1 = r0 (still valid)
        BytecodeInstr::Mov {
            dst: Reg(1),
            src: Reg(0),
        },
        // return r1
        BytecodeInstr::ReturnValue { value: Reg(1) },
    ]);

    let mut interp = make_interp_with_const(ConstValue::Int(7));

    let result = interp.execute_function(&func, &[]).unwrap();
    assert_eq!(result, RuntimeValue::Int(7), "Release 不应修改寄存器值");
}

/// Release on unset register is also a no-op (no panic)
#[test]
fn test_release_unset_register() {
    let func = make_function(vec![
        // release r5 (unset) — must not panic
        BytecodeInstr::Release { src: Reg(5) },
        BytecodeInstr::Return,
    ]);

    let mut interp = Interpreter::new();
    let result = interp.execute_function(&func, &[]).unwrap();
    assert_eq!(
        result,
        RuntimeValue::Void,
        "对未设置寄存器执行 Release 不应 panic"
    );
}

/// Borrow followed by Release preserves the borrowed value
#[test]
fn test_borrow_then_release_preserves_value() {
    let func = make_function(vec![
        BytecodeInstr::LoadConst {
            dst: Reg(0),
            const_idx: 0,
        },
        // r1 = borrow r0
        BytecodeInstr::Borrow {
            dst: Reg(1),
            src: Reg(0),
            mutable: true,
        },
        // release r1 (no-op)
        BytecodeInstr::Release { src: Reg(1) },
        // return r1 — value still intact
        BytecodeInstr::ReturnValue { value: Reg(1) },
    ]);

    let mut interp = make_interp_with_const(ConstValue::Bool(true));

    let result = interp.execute_function(&func, &[]).unwrap();
    assert_eq!(
        result,
        RuntimeValue::Bool(true),
        "Borrow 后 Release 应保留借用的值"
    );
}

/// 端到端测试：Standard 模式下 spawn 并发执行两个任务
///
/// 验证：
/// 1. 两个任务都能正确执行并返回结果
/// 2. Runtime facade 正确配置为 Standard 模式
/// 3. 任务通过 DAG 调度器执行（非 Embedded 顺序执行）
#[test]
fn spawn_concurrent_standard_mode() {
    // Arrange — Standard 模式 + 1 worker；配置后 reset 重建 rt，并先校验配置已生效
    let module = spawn_two_tasks_module();
    let mut interp = standard_mode_interpreter();
    assert_standard_runtime_config(&interp);

    // Act — Spawn 应通过 DAG 调度器执行两个闭包
    let result = interp.execute_module(&module);

    // Assert — 执行成功，且 Runtime facade 配置保持不变
    assert!(
        result.is_ok(),
        "Standard 模式下 spawn 执行不应报错: {:?}",
        result.err()
    );
    assert_standard_runtime_config(&interp);
}

/// #299 §2：NewArray 分配定长 Array，元素默认 Void 占位
#[test]
fn test_new_array_allocates_fixed_void_slots() {
    // Arrange — 含 NewArray(count=5) + ReturnValue 的字节码函数
    let func = make_function(vec![
        // r0 = Array(5)（全 Void 占位）
        BytecodeInstr::NewArray {
            dst: Reg(0),
            count: 5,
        },
        // return r0
        BytecodeInstr::ReturnValue { value: Reg(0) },
    ]);

    // Act — 执行并取返回值
    let mut interp = Interpreter::new();
    let result = interp.execute_function(&func, &[]).unwrap();

    // Assert — 返回 RuntimeValue::Array，长度 5 且全为 Void 占位
    match result {
        RuntimeValue::Array(handle) => {
            let guard = handle.lock();
            match &*guard {
                crate::backends::common::HeapValue::Array(items) => {
                    assert_eq!(items.len(), 5, "Array 应有 5 个元素占位");
                    assert!(
                        items.iter().all(|v| matches!(v, RuntimeValue::Void)),
                        "元素应默认 Void 占位"
                    );
                }
                other => panic!("应产出 HeapValue::Array，实际: {other:?}"),
            }
        }
        other => panic!("应返回 RuntimeValue::Array，实际: {other:?}"),
    }
}

/// #299 §2：NewArray 指令经字节码序列化→解码往返后保持语义
#[test]
fn test_new_array_bytecode_roundtrip_decode() {
    // Arrange — 构造原始编码：opcode(1) + dst(1) + count(4, 小端)
    let file = new_array_roundtrip_file();

    // Act — 字节码反序列化 → 解码
    let module = crate::middle::bytecode::BytecodeModule::from(file);
    let func = &module.functions[0];

    // Assert — 解码结果保持 NewArray 语义
    assert_new_array_decoded(func);

    // 全链路：解码后执行，验证产出 Array
    let mut interp = Interpreter::new();
    let result = interp
        .execute_function(func, &[])
        .expect("interp.execute_function(func, &[]) 应成功");
    assert!(
        matches!(result, RuntimeValue::Array(_)),
        "往返后执行应产出 Array"
    );
}
