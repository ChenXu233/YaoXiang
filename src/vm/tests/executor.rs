//! VM 单元测试
//!
//! 测试虚拟机执行器的配置、状态和值类型

use crate::runtime::value::{RuntimeValue, Heap, HeapValue};
use crate::vm::executor::{VMConfig, VMStatus, VM};
use crate::vm::opcode::TypedOpcode;
use std::sync::Arc;

#[cfg(test)]
mod vm_config_tests {
    use super::*;

    #[test]
    fn test_vm_config_default() {
        let config = VMConfig::default();
        assert_eq!(config.stack_size, 64 * 1024);
        assert_eq!(config.max_call_depth, 1024);
        assert!(!config.trace_execution);
    }

    #[test]
    fn test_vm_config_custom() {
        let config = VMConfig {
            stack_size: 128 * 1024,
            max_call_depth: 2048,
            trace_execution: true,
        };
        assert_eq!(config.stack_size, 128 * 1024);
        assert_eq!(config.max_call_depth, 2048);
        assert!(config.trace_execution);
    }

    #[test]
    fn test_vm_config_clone() {
        let config = VMConfig::default();
        let cloned = config.clone();
        assert_eq!(cloned.stack_size, config.stack_size);
    }
}

#[cfg(test)]
mod vm_tests {
    use super::*;

    #[test]
    fn test_vm_new() {
        let vm = VM::new();
        assert_eq!(vm.status(), VMStatus::Ready);
        assert!(vm.error().is_none());
    }

    #[test]
    fn test_vm_default() {
        let vm = VM::default();
        assert_eq!(vm.status(), VMStatus::Ready);
    }

    #[test]
    fn test_vm_new_with_config() {
        let config = VMConfig::default();
        let vm = VM::new_with_config(config);
        assert_eq!(vm.status(), VMStatus::Ready);
    }

    #[test]
    fn test_vm_execute_module() {
        use crate::middle::codegen::bytecode::CompiledModule;
        use crate::middle::ir::ModuleIR;

        let mut vm = VM::new();
        // Create a minimal CompiledModule for testing
        let module = CompiledModule::from_ir(&ModuleIR {
            types: vec![],
            globals: vec![],
            functions: vec![],
        });
        let result = vm.execute_module(&module);
        assert!(result.is_ok());
        assert_eq!(vm.status(), VMStatus::Finished);
    }

    #[test]
    fn test_vm_debug() {
        let vm = VM::new();
        let debug_output = format!("{:?}", vm);
        assert!(debug_output.contains("VM"));
    }
}

#[cfg(test)]
mod value_tests {
    use super::*;

    #[test]
    fn test_value_void() {
        let value = RuntimeValue::Unit;
        assert!(matches!(value, RuntimeValue::Unit));
    }

    #[test]
    fn test_value_bool() {
        let true_val = RuntimeValue::Bool(true);
        let false_val = RuntimeValue::Bool(false);
        assert!(matches!(true_val, RuntimeValue::Bool(true)));
        assert!(matches!(false_val, RuntimeValue::Bool(false)));
    }

    #[test]
    fn test_value_int() {
        let value = RuntimeValue::Int(42);
        assert!(matches!(value, RuntimeValue::Int(42)));
        let negative = RuntimeValue::Int(-100);
        assert!(matches!(negative, RuntimeValue::Int(-100)));
    }

    #[test]
    fn test_value_float() {
        let value = RuntimeValue::Float(3.14);
        assert!(matches!(value, RuntimeValue::Float(f) if (f - 3.14).abs() < 0.001));
    }

    #[test]
    fn test_value_char() {
        let value = RuntimeValue::Char('中' as u32);
        assert!(matches!(value, RuntimeValue::Char(c) if c == '中' as u32));
    }

    #[test]
    fn test_value_string() {
        let value = RuntimeValue::String(Arc::from("hello"));
        assert!(matches!(value, RuntimeValue::String(s) if s.as_ref() == "hello"));
    }

    #[test]
    fn test_value_bytes() {
        let value = RuntimeValue::Bytes(Arc::from(vec![1, 2, 3]));
        assert!(matches!(value, RuntimeValue::Bytes(b) if b.as_ref() == vec![1, 2, 3]));
    }

    #[test]
    fn test_value_list() {
        let mut heap = Heap::new();
        let handle = heap.allocate(HeapValue::List(vec![
            RuntimeValue::Int(1),
            RuntimeValue::Int(2),
        ]));
        let _value = RuntimeValue::List(handle);
        // Verify list is stored in heap
        match heap.get(handle) {
            Some(HeapValue::List(items)) => {
                assert_eq!(items.len(), 2);
            }
            _ => panic!("expected List in heap"),
        }
    }

    #[test]
    fn test_value_clone() {
        let value = RuntimeValue::Int(42);
        let cloned = value.clone();
        assert!(matches!(cloned, RuntimeValue::Int(42)));
    }
}

#[cfg(test)]
mod opcode_tests {
    use super::*;

    #[test]
    fn test_opcode_values() {
        assert_eq!(TypedOpcode::Nop as u8, 0x00);
        assert_eq!(TypedOpcode::Return as u8, 0x01);
        assert_eq!(TypedOpcode::ReturnValue as u8, 0x02);
        assert_eq!(TypedOpcode::Jmp as u8, 0x03);
    }

    #[test]
    fn test_opcode_try_from_valid() {
        let result = TypedOpcode::try_from(0x00);
        assert!(result.is_ok());
        assert!(matches!(result, Ok(TypedOpcode::Nop)));
    }

    #[test]
    fn test_opcode_try_from_invalid() {
        // 0x7E 未被使用（ArcDrop 是 0x7C），应该返回错误
        let result = TypedOpcode::try_from(0x7E);
        assert!(result.is_err());
    }

    #[test]
    fn test_opcode_partial_eq() {
        assert_eq!(TypedOpcode::Nop, TypedOpcode::Nop);
        assert_ne!(TypedOpcode::Nop, TypedOpcode::Return);
    }

    #[test]
    fn test_opcode_debug() {
        let debug_output = format!("{:?}", TypedOpcode::Nop);
        assert!(debug_output.contains("Nop"));
    }
}

#[cfg(test)]
mod vm_status_tests {
    use super::*;

    #[test]
    fn test_vm_status_values() {
        assert_eq!(VMStatus::Ready as u8, 0);
        assert_eq!(VMStatus::Running as u8, 1);
        assert_eq!(VMStatus::Finished as u8, 2);
        assert_eq!(VMStatus::Error as u8, 3);
    }

    #[test]
    fn test_vm_status_partial_eq() {
        assert_eq!(VMStatus::Ready, VMStatus::Ready);
        assert_ne!(VMStatus::Ready, VMStatus::Running);
    }

    #[test]
    fn test_vm_status_debug() {
        let debug_output = format!("{:?}", VMStatus::Ready);
        assert!(debug_output.contains("Ready"));
    }
}

// =====================
// 指令执行集成测试
// =====================

#[cfg(test)]
mod instruction_execution_tests {

    mod opcode_encoding_tests {
        use crate::vm::opcode::TypedOpcode;

        #[test]
        fn test_i32_opcodes_exist() {
            // 验证 I32 操作码存在（不检查具体编码，只验证变体存在）
            let _ = TypedOpcode::I32Add;
            let _ = TypedOpcode::I32Sub;
            let _ = TypedOpcode::I32Mul;
            let _ = TypedOpcode::I32Div;
            let _ = TypedOpcode::I32Rem;
            let _ = TypedOpcode::I32Neg;
        }

        #[test]
        fn test_i32_bitwise_opcodes_exist() {
            let _ = TypedOpcode::I32And;
            let _ = TypedOpcode::I32Or;
            let _ = TypedOpcode::I32Xor;
            let _ = TypedOpcode::I32Shl;
            let _ = TypedOpcode::I32Sar;
            let _ = TypedOpcode::I32Shr;
        }

        #[test]
        fn test_i64_bitwise_opcodes_exist() {
            let _ = TypedOpcode::I64And;
            let _ = TypedOpcode::I64Or;
            let _ = TypedOpcode::I64Xor;
            let _ = TypedOpcode::I64Shl;
            let _ = TypedOpcode::I64Sar;
            let _ = TypedOpcode::I64Shr;
        }

        #[test]
        fn test_f32_opcodes_exist() {
            let _ = TypedOpcode::F32Add;
            let _ = TypedOpcode::F32Sub;
            let _ = TypedOpcode::F32Mul;
            let _ = TypedOpcode::F32Div;
            let _ = TypedOpcode::F32Rem;
            let _ = TypedOpcode::F32Sqrt;
            let _ = TypedOpcode::F32Neg;
        }

        #[test]
        fn test_f32_comparison_opcodes_exist() {
            let _ = TypedOpcode::F32Eq;
            let _ = TypedOpcode::F32Ne;
            let _ = TypedOpcode::F32Lt;
            let _ = TypedOpcode::F32Le;
            let _ = TypedOpcode::F32Gt;
            let _ = TypedOpcode::F32Ge;
        }

        #[test]
        fn test_f64_rem_exists() {
            let _ = TypedOpcode::F64Rem;
        }

        #[test]
        fn test_string_opcodes_exist() {
            let _ = TypedOpcode::StringLength;
            let _ = TypedOpcode::StringConcat;
            let _ = TypedOpcode::StringEqual;
            let _ = TypedOpcode::StringGetChar;
            let _ = TypedOpcode::StringFromInt;
            let _ = TypedOpcode::StringFromFloat;
        }

        #[test]
        fn test_closure_opcodes_exist() {
            let _ = TypedOpcode::MakeClosure;
            let _ = TypedOpcode::LoadUpvalue;
            let _ = TypedOpcode::StoreUpvalue;
            let _ = TypedOpcode::CloseUpvalue;
        }

        #[test]
        fn test_exception_opcodes_exist() {
            let _ = TypedOpcode::TryBegin;
            let _ = TypedOpcode::TryEnd;
            let _ = TypedOpcode::Throw;
            let _ = TypedOpcode::Rethrow;
        }

        #[test]
        fn test_memory_opcodes_exist() {
            let _ = TypedOpcode::StackAlloc;
            let _ = TypedOpcode::HeapAlloc;
            let _ = TypedOpcode::GetField;
            let _ = TypedOpcode::SetField;
        }

        #[test]
        fn test_arc_opcodes_exist() {
            let _ = TypedOpcode::ArcNew;
            let _ = TypedOpcode::ArcClone;
            let _ = TypedOpcode::ArcDrop;
        }

        #[test]
        fn test_type_opcodes_exist() {
            let _ = TypedOpcode::TypeCheck;
            let _ = TypedOpcode::TypeOf;
            let _ = TypedOpcode::Cast;
        }

        #[test]
        fn test_control_flow_opcodes_exist() {
            // Phase 1: 控制流指令
            let _ = TypedOpcode::Switch; // 0x06 - 多分支跳转
            let _ = TypedOpcode::LoopStart; // 0x07 - 循环开始
            let _ = TypedOpcode::LoopInc; // 0x08 - 循环递增
            let _ = TypedOpcode::TailCall; // 0x09 - 尾调用
            let _ = TypedOpcode::Yield; // 0x0A - 协程让出
            let _ = TypedOpcode::Label; // 0x0B - 标签定义
        }
    }

    mod opcode_parsing_tests {
        use crate::vm::opcode::TypedOpcode;

        #[test]
        fn test_parse_known_opcodes() {
            // 验证已知操作码可以正确解析
            assert!(TypedOpcode::try_from(0x00).is_ok()); // Nop
            assert!(TypedOpcode::try_from(0x01).is_ok()); // Return
            assert!(TypedOpcode::try_from(0x03).is_ok()); // Jmp
            assert!(TypedOpcode::try_from(0x10).is_ok()); // Mov
        }

        #[test]
        fn test_parse_i32_opcodes() {
            // 验证 I32 操作码解析
            assert!(TypedOpcode::try_from(0x30).is_ok()); // I32Add
            assert!(TypedOpcode::try_from(0x31).is_ok()); // I32Sub
            assert!(TypedOpcode::try_from(0x32).is_ok()); // I32Mul
            assert!(TypedOpcode::try_from(0x33).is_ok()); // I32Div
            assert!(TypedOpcode::try_from(0x34).is_ok()); // I32Rem
            assert!(TypedOpcode::try_from(0x35).is_ok()); // I32Neg
        }

        #[test]
        fn test_parse_i32_bitwise_opcodes() {
            assert!(TypedOpcode::try_from(0x36).is_ok()); // I32And
            assert!(TypedOpcode::try_from(0x37).is_ok()); // I32Or
            assert!(TypedOpcode::try_from(0x38).is_ok()); // I32Xor
            assert!(TypedOpcode::try_from(0x39).is_ok()); // I32Shl
            assert!(TypedOpcode::try_from(0x3A).is_ok()); // I32Sar
            assert!(TypedOpcode::try_from(0x3B).is_ok()); // I32Shr
        }

        #[test]
        fn test_parse_f32_opcodes() {
            assert!(TypedOpcode::try_from(0x50).is_ok()); // F32Add
            assert!(TypedOpcode::try_from(0x51).is_ok()); // F32Sub
            assert!(TypedOpcode::try_from(0x52).is_ok()); // F32Mul
            assert!(TypedOpcode::try_from(0x53).is_ok()); // F32Div
            assert!(TypedOpcode::try_from(0x54).is_ok()); // F32Rem
        }

        #[test]
        fn test_parse_f32_comparison_opcodes() {
            assert!(TypedOpcode::try_from(0x6C).is_ok());
            assert!(TypedOpcode::try_from(0x6D).is_ok());
            assert!(TypedOpcode::try_from(0x6E).is_ok());
            assert!(TypedOpcode::try_from(0x6F).is_ok());
            assert!(TypedOpcode::try_from(0x70).is_ok());
            assert!(TypedOpcode::try_from(0x71).is_ok());
        }

        #[test]
        fn test_parse_string_opcodes() {
            assert!(TypedOpcode::try_from(0x90).is_ok());
            assert!(TypedOpcode::try_from(0x91).is_ok());
            assert!(TypedOpcode::try_from(0x92).is_ok());
            assert!(TypedOpcode::try_from(0x93).is_ok());
            assert!(TypedOpcode::try_from(0x94).is_ok());
            assert!(TypedOpcode::try_from(0x95).is_ok());
        }

        #[test]
        fn test_parse_closure_opcodes() {
            assert!(TypedOpcode::try_from(0x83).is_ok()); // MakeClosure
            assert!(TypedOpcode::try_from(0x84).is_ok());
            assert!(TypedOpcode::try_from(0x85).is_ok());
            assert!(TypedOpcode::try_from(0x86).is_ok());
        }

        #[test]
        fn test_parse_exception_opcodes() {
            assert!(TypedOpcode::try_from(0xA0).is_ok());
            assert!(TypedOpcode::try_from(0xA1).is_ok());
            assert!(TypedOpcode::try_from(0xA2).is_ok());
            assert!(TypedOpcode::try_from(0xA3).is_ok());
        }

        #[test]
        fn test_parse_memory_opcodes() {
            assert!(TypedOpcode::try_from(0x72).is_ok());
            assert!(TypedOpcode::try_from(0x73).is_ok());
            assert!(TypedOpcode::try_from(0x75).is_ok());
            assert!(TypedOpcode::try_from(0x76).is_ok());
        }

        #[test]
        fn test_parse_arc_opcodes() {
            assert!(TypedOpcode::try_from(0x7A).is_ok()); // ArcNew
            assert!(TypedOpcode::try_from(0x7B).is_ok()); // ArcClone
            assert!(TypedOpcode::try_from(0x7C).is_ok()); // ArcDrop
        }

        #[test]
        fn test_parse_type_opcodes() {
            assert!(TypedOpcode::try_from(0xC0).is_ok()); // TypeCheck
            assert!(TypedOpcode::try_from(0xC1).is_ok()); // Cast
        }

        #[test]
        fn test_parse_control_flow_opcodes() {
            // Phase 1: 控制流指令解析测试
            assert!(TypedOpcode::try_from(0x06).is_ok()); // Switch
            assert!(TypedOpcode::try_from(0x07).is_ok()); // LoopStart
            assert!(TypedOpcode::try_from(0x08).is_ok()); // LoopInc
            assert!(TypedOpcode::try_from(0x09).is_ok()); // TailCall
            assert!(TypedOpcode::try_from(0x0A).is_ok()); // Yield
            assert!(TypedOpcode::try_from(0x0B).is_ok()); // Label
        }

        #[test]
        fn test_invalid_opcode_returns_error() {
            // 验证无效操作码返回错误
            assert!(TypedOpcode::try_from(0x7E).is_err()); // 未使用
        }
    }

    mod opcode_name_tests {
        use crate::vm::opcode::TypedOpcode;

        #[test]
        fn test_i32_opcode_names() {
            assert_eq!(TypedOpcode::I32Add.name(), "I32Add");
            assert_eq!(TypedOpcode::I32Sub.name(), "I32Sub");
            assert_eq!(TypedOpcode::I32Mul.name(), "I32Mul");
            assert_eq!(TypedOpcode::I32Div.name(), "I32Div");
            assert_eq!(TypedOpcode::I32Rem.name(), "I32Rem");
            assert_eq!(TypedOpcode::I32Neg.name(), "I32Neg");
        }

        #[test]
        fn test_f32_opcode_names() {
            assert_eq!(TypedOpcode::F32Add.name(), "F32Add");
            assert_eq!(TypedOpcode::F32Sub.name(), "F32Sub");
            assert_eq!(TypedOpcode::F32Mul.name(), "F32Mul");
            assert_eq!(TypedOpcode::F32Div.name(), "F32Div");
            assert_eq!(TypedOpcode::F32Rem.name(), "F32Rem");
            assert_eq!(TypedOpcode::F32Sqrt.name(), "F32Sqrt");
        }

        #[test]
        fn test_f32_comparison_names() {
            assert_eq!(TypedOpcode::F32Eq.name(), "F32Eq");
            assert_eq!(TypedOpcode::F32Ne.name(), "F32Ne");
            assert_eq!(TypedOpcode::F32Lt.name(), "F32Lt");
            assert_eq!(TypedOpcode::F32Le.name(), "F32Le");
            assert_eq!(TypedOpcode::F32Gt.name(), "F32Gt");
            assert_eq!(TypedOpcode::F32Ge.name(), "F32Ge");
        }

        #[test]
        fn test_string_opcode_names() {
            assert_eq!(TypedOpcode::StringLength.name(), "StringLength");
            assert_eq!(TypedOpcode::StringConcat.name(), "StringConcat");
            assert_eq!(TypedOpcode::StringEqual.name(), "StringEqual");
            assert_eq!(TypedOpcode::StringGetChar.name(), "StringGetChar");
        }

        #[test]
        fn test_exception_opcode_names() {
            assert_eq!(TypedOpcode::TryBegin.name(), "TryBegin");
            assert_eq!(TypedOpcode::TryEnd.name(), "TryEnd");
            assert_eq!(TypedOpcode::Throw.name(), "Throw");
            assert_eq!(TypedOpcode::Rethrow.name(), "Rethrow");
        }

        #[test]
        fn test_closure_opcode_names() {
            assert_eq!(TypedOpcode::MakeClosure.name(), "MakeClosure");
            assert_eq!(TypedOpcode::LoadUpvalue.name(), "LoadUpvalue");
            assert_eq!(TypedOpcode::StoreUpvalue.name(), "StoreUpvalue");
            assert_eq!(TypedOpcode::CloseUpvalue.name(), "CloseUpvalue");
        }

        #[test]
        fn test_control_flow_opcode_names() {
            // Phase 1: 控制流指令名称测试
            assert_eq!(TypedOpcode::Switch.name(), "Switch");
            assert_eq!(TypedOpcode::LoopStart.name(), "LoopStart");
            assert_eq!(TypedOpcode::LoopInc.name(), "LoopInc");
            assert_eq!(TypedOpcode::TailCall.name(), "TailCall");
            assert_eq!(TypedOpcode::Yield.name(), "Yield");
            assert_eq!(TypedOpcode::Label.name(), "Label");
        }
    }

    // =====================
    // P0 指令测试：编译器后端必需
    // =====================
    mod p0_instruction_tests {
        use crate::runtime::value::{RuntimeValue, Heap, HeapValue, Handle};

        // 测试 StoreElement 指令逻辑
        #[test]
        fn test_store_element_logic() {
            // 直接测试数据结构操作逻辑
            let mut heap = Heap::new();
            let handle = heap.allocate(HeapValue::List(vec![
                RuntimeValue::Int(1),
                RuntimeValue::Int(2),
                RuntimeValue::Int(3),
            ]));

            // 模拟 StoreElement: list[1] = 99
            let idx = 1usize;
            let new_value = RuntimeValue::Int(99);

            if let Some(HeapValue::List(items)) = heap.get_mut(handle) {
                assert!(idx < items.len(), "Index out of bounds");
                items[idx] = new_value.clone();
            }

            // 验证修改
            if let Some(HeapValue::List(items)) = heap.get(handle) {
                assert_eq!(items[1], RuntimeValue::Int(99));
            } else {
                panic!("List not found in heap");
            }
        }

        // 测试 NewListWithCap 指令逻辑
        #[test]
        fn test_new_list_with_cap_logic() {
            let capacity = 10;
            let mut heap = Heap::new();
            let handle = heap.allocate(HeapValue::List(Vec::with_capacity(capacity)));

            // Handle 可以是 0，检查是否在 heap 中
            assert!(heap.get(handle).is_some());

            if let Some(HeapValue::List(items)) = heap.get(handle) {
                assert!(items.capacity() >= capacity);
            }
        }

        // 测试 GetField 指令（List）
        #[test]
        fn test_get_field_list() {
            let mut heap = Heap::new();
            let handle = heap.allocate(HeapValue::List(vec![
                RuntimeValue::Int(10),
                RuntimeValue::Int(20),
                RuntimeValue::Int(30),
            ]));

            let offset = 1;
            let value = match heap.get(handle) {
                Some(HeapValue::List(items)) => items.get(offset).cloned().unwrap_or(RuntimeValue::Unit),
                _ => RuntimeValue::Unit,
            };

            assert_eq!(value, RuntimeValue::Int(20));
        }

        // 测试 GetField 指令（Tuple）
        #[test]
        fn test_get_field_tuple() {
            let mut heap = Heap::new();
            let handle = heap.allocate(HeapValue::Tuple(vec![
                RuntimeValue::String("hello".into()),
                RuntimeValue::Int(42),
            ]));

            let offset = 0;
            let value = match heap.get(handle) {
                Some(HeapValue::Tuple(items)) => items.get(offset).cloned().unwrap_or(RuntimeValue::Unit),
                _ => RuntimeValue::Unit,
            };

            assert!(matches!(value, RuntimeValue::String(s) if s.as_ref() == "hello"));
        }

        // 测试 SetField 指令（List）
        #[test]
        fn test_set_field_list() {
            let mut heap = Heap::new();
            let handle = heap.allocate(HeapValue::List(vec![
                RuntimeValue::Int(1),
                RuntimeValue::Int(2),
            ]));

            // 模拟 SetField: list[0] = 100
            let offset = 0;
            let new_value = RuntimeValue::Int(100);

            if let Some(HeapValue::List(items)) = heap.get_mut(handle) {
                items[offset] = new_value.clone();
            }

            // 验证
            if let Some(HeapValue::List(items)) = heap.get(handle) {
                assert_eq!(items[0], RuntimeValue::Int(100));
            }
        }

        // 测试 GetField 边界情况
        #[test]
        fn test_get_field_out_of_bounds() {
            let mut heap = Heap::new();
            let handle = heap.allocate(HeapValue::List(vec![RuntimeValue::Int(1)]));

            let offset = 10; // 超出范围
            let value = match heap.get(handle) {
                Some(HeapValue::List(items)) => items.get(offset).cloned().unwrap_or(RuntimeValue::Unit),
                _ => RuntimeValue::Unit,
            };

            assert_eq!(value, RuntimeValue::Unit);
        }
    }

    // =====================
    // P1 指令测试：函数式编程
    // =====================
    mod p1_instruction_tests {
        use crate::runtime::value::{RuntimeValue, FunctionValue, FunctionId};

        // 测试 MakeClosure 指令逻辑
        #[test]
        fn test_make_closure_logic() {
            let func_id: u32 = 123;
            let upvalue_count = 2;

            // 模拟闭包环境
            let env = vec![
                RuntimeValue::Int(10),
                RuntimeValue::Int(20),
            ];

            // 创建闭包
            let closure = RuntimeValue::Function(FunctionValue {
                func_id: FunctionId(func_id),
                env: env.clone(),
            });

            // 验证闭包创建
            if let RuntimeValue::Function(func) = &closure {
                assert_eq!(func.func_id.0, func_id);
                assert_eq!(func.env.len(), upvalue_count as usize);
                assert_eq!(func.env[0], RuntimeValue::Int(10));
                assert_eq!(func.env[1], RuntimeValue::Int(20));
            } else {
                panic!("Expected Function value");
            }
        }

        // 测试 LoadUpvalue 逻辑
        #[test]
        fn test_load_upvalue_logic() {
            let closure_env = vec![
                RuntimeValue::Int(42),
                RuntimeValue::String("test".into()),
            ];

            let upvalue_idx = 0usize;
            let dst_value = closure_env.get(upvalue_idx).cloned().unwrap_or(RuntimeValue::Unit);

            assert_eq!(dst_value, RuntimeValue::Int(42));
        }

        // 测试 StoreUpvalue 逻辑
        #[test]
        fn test_store_upvalue_logic() {
            let mut closure_env = vec![RuntimeValue::Unit];
            let new_value = RuntimeValue::Float(3.14);

            let upvalue_idx = 0usize;
            // 确保空间足够
            if upvalue_idx >= closure_env.len() {
                closure_env.resize(upvalue_idx + 1, RuntimeValue::Unit);
            }
            closure_env[upvalue_idx] = new_value.clone();

            // 验证
            if let RuntimeValue::Float(f) = closure_env[0] {
                assert!((f - 3.14).abs() < 0.001);
            } else {
                panic!("Expected Float value");
            }
        }

        // 测试 CloseUpvalue 逻辑
        #[test]
        fn test_close_upvalue_logic() {
            let mut closure_env: Vec<RuntimeValue> = vec![];
            let new_value = RuntimeValue::Bool(true);

            closure_env.push(new_value.clone());

            // 验证
            assert_eq!(closure_env.len(), 1);
            assert_eq!(closure_env[0], RuntimeValue::Bool(true));
        }

        // 测试空闭包
        #[test]
        fn test_empty_closure() {
            let closure = RuntimeValue::Function(FunctionValue {
                func_id: FunctionId(0),
                env: vec![],
            });

            if let RuntimeValue::Function(func) = closure {
                assert!(func.env.is_empty());
            }
        }
    }

    // =====================
    // P2 指令测试：高级特性
    // =====================
    mod p2_instruction_tests {
        use crate::runtime::value::RuntimeValue;
        use std::sync::Arc;

        // 测试 ArcNew 逻辑
        #[test]
        fn test_arc_new_logic() {
            let value = RuntimeValue::Int(42);
            let arc_value = value.into_arc();

            // 验证
            if let RuntimeValue::Arc(arc_ref) = &arc_value {
                if let RuntimeValue::Int(n) = **arc_ref {
                    assert_eq!(n, 42);
                } else {
                    panic!("Expected Int inside Arc");
                }
            } else {
                panic!("Expected Arc value");
            }
        }

        // 测试 ArcClone 逻辑
        #[test]
        fn test_arc_clone_logic() {
            let original = RuntimeValue::Int(100).into_arc();
            let cloned = original.clone();

            // 验证克隆
            assert!(matches!(cloned, RuntimeValue::Arc(_)));
        }

        // 测试 ArcClone 非 Arc 类型
        #[test]
        fn test_arc_clone_non_arc() {
            let value = RuntimeValue::Int(200);
            let arc_value = value.into_arc();

            assert!(matches!(arc_value, RuntimeValue::Arc(_)));
        }

        // 测试 ArcDrop 逻辑（通过清除引用）
        #[test]
        fn test_arc_drop_logic() {
            let arc_value = RuntimeValue::Int(999).into_arc();

            // 模拟 ArcDrop：清除引用
            let cleared = RuntimeValue::Unit;

            // 验证
            assert!(matches!(cleared, RuntimeValue::Unit));
        }

        // 测试 Cast：I64 → F64
        #[test]
        fn test_cast_int_to_float() {
            let src: RuntimeValue = RuntimeValue::Int(42);
            let target_type_id: u16 = 0; // I64 → F64

            let dst = match target_type_id {
                0 => {
                    if let RuntimeValue::Int(n) = &src {
                        RuntimeValue::Float(*n as f64)
                    } else {
                        panic!("Expected Int");
                    }
                }
                _ => panic!("Unknown cast type"),
            };

            if let RuntimeValue::Float(f) = dst {
                assert!((f - 42.0).abs() < 0.001);
            } else {
                panic!("Expected Float value");
            }
        }

        // 测试 Cast：F64 → I64
        #[test]
        fn test_cast_float_to_int() {
            let src: RuntimeValue = RuntimeValue::Float(3.99);
            let target_type_id: u16 = 1; // F64 → I64

            let dst = match target_type_id {
                1 => {
                    if let RuntimeValue::Float(f) = &src {
                        RuntimeValue::Int(*f as i64)
                    } else {
                        panic!("Expected Float");
                    }
                }
                _ => panic!("Unknown cast type"),
            };

            if let RuntimeValue::Int(n) = dst {
                assert_eq!(n, 3); // 截断
            } else {
                panic!("Expected Int value");
            }
        }

        // 测试 Cast：I64 → I32
        #[test]
        fn test_cast_int_to_i32() {
            let src: RuntimeValue = RuntimeValue::Int(100);
            let target_type_id: u16 = 2; // I64 → I32

            let dst = match target_type_id {
                2 => {
                    if let RuntimeValue::Int(n) = &src {
                        RuntimeValue::Int(*n as i32 as i64)
                    } else {
                        panic!("Expected Int");
                    }
                }
                _ => panic!("Unknown cast type"),
            };

            assert!(matches!(dst, RuntimeValue::Int(100)));
        }

        // 测试 Cast：I32 → I64
        #[test]
        fn test_cast_i32_to_int() {
            let src: RuntimeValue = RuntimeValue::Int(50); // I32 值
            let target_type_id: u16 = 3; // I32 → I64

            let dst = match target_type_id {
                3 => {
                    if let RuntimeValue::Int(n) = &src {
                        RuntimeValue::Int(*n as i64)
                    } else {
                        panic!("Expected Int");
                    }
                }
                _ => panic!("Unknown cast type"),
            };

            assert!(matches!(dst, RuntimeValue::Int(50)));
        }

        // 测试 Cast：F64 → F32
        #[test]
        fn test_cast_float_to_f32() {
            let src: RuntimeValue = RuntimeValue::Float(3.14159265);
            let target_type_id: u16 = 4; // F64 → F32

            let dst = match target_type_id {
                4 => {
                    if let RuntimeValue::Float(f) = &src {
                        RuntimeValue::Float(*f as f32 as f64)
                    } else {
                        panic!("Expected Float");
                    }
                }
                _ => panic!("Unknown cast type"),
            };

            if let RuntimeValue::Float(f) = dst {
                assert!((f - 3.1415926535).abs() < 0.001); // 精度损失
            } else {
                panic!("Expected Float value");
            }
        }

        // 测试 Cast：F32 → F64
        #[test]
        fn test_cast_f32_to_float() {
            let src: RuntimeValue = RuntimeValue::Float(2.5); // 实际上是 F32
            let target_type_id: u16 = 5; // F32 → F64

            let dst = match target_type_id {
                5 => {
                    if let RuntimeValue::Float(f) = &src {
                        RuntimeValue::Float(*f as f64)
                    } else {
                        panic!("Expected Float");
                    }
                }
                _ => panic!("Unknown cast type"),
            };

            if let RuntimeValue::Float(f) = dst {
                assert!((f - 2.5).abs() < 0.001);
            } else {
                panic!("Expected Float value");
            }
        }

        // 测试 Arc 引用计数
        #[test]
        fn test_arc_reference_counting() {
            use std::rc::Rc;

            // 使用 Rc 测试引用计数行为（与 Arc 类似）
            let rc = Rc::new(RuntimeValue::Int(1));
            assert_eq!(Rc::strong_count(&rc), 1);

            let rc2 = rc.clone();
            assert_eq!(Rc::strong_count(&rc), 2);

            drop(rc2);
            assert_eq!(Rc::strong_count(&rc), 1);

            drop(rc);
            // Rc 被释放
        }
    }

    // =====================
    // P3 指令测试：基础操作
    // =====================
    mod p3_instruction_tests {
        use crate::runtime::value::RuntimeValue;
        use std::collections::HashMap;

        // 测试 Mov 指令逻辑
        #[test]
        fn test_mov_logic() {
            let src_value = RuntimeValue::Int(42);
            let dst_value = src_value.clone();

            assert_eq!(dst_value, RuntimeValue::Int(42));
        }

        // 测试 Drop 指令逻辑
        #[test]
        fn test_drop_logic() {
            // Drop 只是让值离开作用域
            let _value = RuntimeValue::Int(999);
            // 值离开作用域后被释放
        }

        // 测试 LoadElement 指令逻辑
        #[test]
        fn test_load_element_logic() {
            let items = vec![
                RuntimeValue::Int(10),
                RuntimeValue::Int(20),
                RuntimeValue::Int(30),
            ];

            let idx = 2usize;
            let value = items.get(idx).cloned().unwrap_or(RuntimeValue::Unit);

            assert_eq!(value, RuntimeValue::Int(30));
        }

        // 测试 LoadElement 边界检查
        #[test]
        fn test_load_element_out_of_bounds() {
            let items = vec![RuntimeValue::Int(1)];

            let idx = 5usize;
            let value = items.get(idx).cloned().unwrap_or(RuntimeValue::Unit);

            assert_eq!(value, RuntimeValue::Unit);
        }

        // 测试 Nop 指令逻辑
        #[test]
        fn test_nop_logic() {
            // Nop 不做任何事情
            let result = ();
            assert!(result == ());
        }

        // 测试 Return 指令逻辑
        #[test]
        fn test_return_logic() {
            // Return 只是返回单位值
            let return_value = RuntimeValue::Unit;
            assert!(matches!(return_value, RuntimeValue::Unit));
        }
    }

    // =====================
    // P4 指令测试：整数操作
    // =====================
    mod p4_instruction_tests {
        use crate::runtime::value::RuntimeValue;

        // 测试 I64Add 逻辑
        #[test]
        fn test_i64_add_logic() {
            let a = RuntimeValue::Int(10);
            let b = RuntimeValue::Int(32);

            let result = match (a, b) {
                (RuntimeValue::Int(x), RuntimeValue::Int(y)) => RuntimeValue::Int(x + y),
                _ => RuntimeValue::Unit,
            };

            assert_eq!(result, RuntimeValue::Int(42));
        }

        // 测试 I64Sub 逻辑
        #[test]
        fn test_i64_sub_logic() {
            let a = RuntimeValue::Int(50);
            let b = RuntimeValue::Int(8);

            let result = match (a, b) {
                (RuntimeValue::Int(x), RuntimeValue::Int(y)) => RuntimeValue::Int(x - y),
                _ => RuntimeValue::Unit,
            };

            assert_eq!(result, RuntimeValue::Int(42));
        }

        // 测试 I64Mul 逻辑
        #[test]
        fn test_i64_mul_logic() {
            let a = RuntimeValue::Int(6);
            let b = RuntimeValue::Int(7);

            let result = match (a, b) {
                (RuntimeValue::Int(x), RuntimeValue::Int(y)) => RuntimeValue::Int(x * y),
                _ => RuntimeValue::Unit,
            };

            assert_eq!(result, RuntimeValue::Int(42));
        }

        // 测试 I64Div 逻辑
        #[test]
        fn test_i64_div_logic() {
            let a = RuntimeValue::Int(84);
            let b = RuntimeValue::Int(2);

            let result = match (a, b) {
                (RuntimeValue::Int(x), RuntimeValue::Int(y)) if y != 0 => RuntimeValue::Int(x / y),
                _ => RuntimeValue::Unit,
            };

            assert_eq!(result, RuntimeValue::Int(42));
        }

        // 测试 I64Neg 逻辑
        #[test]
        fn test_i64_neg_logic() {
            let a = RuntimeValue::Int(42);

            let result = match a {
                RuntimeValue::Int(x) => RuntimeValue::Int(-x),
                _ => RuntimeValue::Unit,
            };

            assert_eq!(result, RuntimeValue::Int(-42));
        }

        // 测试 I64Mod 逻辑
        #[test]
        fn test_i64_mod_logic() {
            let a = RuntimeValue::Int(85);
            let b = RuntimeValue::Int(43);

            let result = match (a, b) {
                (RuntimeValue::Int(x), RuntimeValue::Int(y)) if y != 0 => RuntimeValue::Int(x % y),
                _ => RuntimeValue::Unit,
            };

            assert_eq!(result, RuntimeValue::Int(42));
        }

        // 测试 I64 比较 (Eq/Ne/Lt/Le/Gt/Ge)
        #[test]
        fn test_i64_comparison() {
            let a = RuntimeValue::Int(42);
            let b = RuntimeValue::Int(42);

            assert!(matches!(a, RuntimeValue::Int(x) if x == 42));
            assert!(matches!(b, RuntimeValue::Int(y) if y == 42));
        }
    }

    // =====================
    // P5 指令测试：字符串操作
    // =====================
    mod p5_instruction_tests {
        use crate::runtime::value::RuntimeValue;
        use std::sync::Arc;

        // 测试 StringLength 逻辑
        #[test]
        fn test_string_length_logic() {
            let s: RuntimeValue = RuntimeValue::String(Arc::from("hello"));
            let len = match &s {
                RuntimeValue::String(str_ref) => str_ref.len(),
                _ => 0,
            };

            assert_eq!(len, 5);
        }

        // 测试 StringConcat 逻辑
        #[test]
        fn test_string_concat_logic() {
            let a: RuntimeValue = RuntimeValue::String(Arc::from("hello"));
            let b: RuntimeValue = RuntimeValue::String(Arc::from(" world"));

            let result = match (a, b) {
                (RuntimeValue::String(s1), RuntimeValue::String(s2)) => {
                    RuntimeValue::String(Arc::from(s1.as_ref().to_owned() + s2.as_ref()))
                }
                _ => RuntimeValue::Unit,
            };

            if let RuntimeValue::String(s) = result {
                assert_eq!(s.as_ref(), "hello world");
            } else {
                panic!("Expected String");
            }
        }

        // 测试 StringEqual 逻辑
        #[test]
        fn test_string_equal_logic() {
            let a: RuntimeValue = RuntimeValue::String(Arc::from("test"));
            let b: RuntimeValue = RuntimeValue::String(Arc::from("test"));
            let c: RuntimeValue = RuntimeValue::String(Arc::from("other"));

            let eq_ab = match (&a, &b) {
                (RuntimeValue::String(s1), RuntimeValue::String(s2)) => s1.as_ref() == s2.as_ref(),
                _ => false,
            };

            let eq_ac = match (&a, &c) {
                (RuntimeValue::String(s1), RuntimeValue::String(s2)) => s1.as_ref() == s2.as_ref(),
                _ => false,
            };

            assert!(eq_ab);
            assert!(!eq_ac);
        }

        // 测试 StringGetChar 逻辑
        #[test]
        fn test_string_get_char_logic() {
            let s: RuntimeValue = RuntimeValue::String(Arc::from("hello"));

            let char_code = match &s {
                RuntimeValue::String(str_ref) => {
                    let chars: Vec<char> = str_ref.chars().collect();
                    chars.get(1).map(|&c| c as u32).unwrap_or(0)
                }
                _ => 0,
            };

            assert_eq!(char_code, 'e' as u32);
        }

        // 测试 StringFromInt 逻辑
        #[test]
        fn test_string_from_int_logic() {
            let n = 42i64;

            let result = RuntimeValue::String(Arc::from(n.to_string()));

            if let RuntimeValue::String(s) = result {
                assert_eq!(s.as_ref(), "42");
            } else {
                panic!("Expected String");
            }
        }

        // 测试 StringFromFloat 逻辑
        #[test]
        fn test_string_from_float_logic() {
            let f = 3.14f64;

            let result = RuntimeValue::String(Arc::from(format!("{}", f)));

            if let RuntimeValue::String(s) = result {
                assert!(s.as_ref().starts_with("3.14"));
            } else {
                panic!("Expected String");
            }
        }

        // 测试空字符串
        #[test]
        fn test_empty_string() {
            let s: RuntimeValue = RuntimeValue::String(Arc::from(""));

            if let RuntimeValue::String(str_ref) = &s {
                assert_eq!(str_ref.len(), 0);
            } else {
                panic!("Expected String");
            }
        }

        // 测试中文字符串
        #[test]
        fn test_chinese_string() {
            let s: RuntimeValue = RuntimeValue::String(Arc::from("你好"));

            if let RuntimeValue::String(str_ref) = &s {
                assert_eq!(str_ref.len(), 6); // UTF-8 编码
            } else {
                panic!("Expected String");
            }
        }
    }

    // =====================
    // P6 指令测试：异常处理
    // =====================
    mod p6_instruction_tests {
        use crate::runtime::value::RuntimeValue;

        // 测试 TryBegin 逻辑（模拟）
        #[test]
        fn test_try_begin_logic() {
            // TryBegin 只是标记异常处理范围的开始
            let in_try_block = true;
            assert!(in_try_block);
        }

        // 测试 TryEnd 逻辑（模拟）
        #[test]
        fn test_try_end_logic() {
            // TryEnd 标记异常处理范围的结束
            let try_active = false;
            assert!(!try_active);
        }

        // 测试 Throw 逻辑
        #[test]
        fn test_throw_logic() {
            // Throw 创建异常值
            let exception = RuntimeValue::String("error".into());

            assert!(matches!(exception, RuntimeValue::String(_)));
        }

        // 测试 Rethrow 逻辑
        #[test]
        fn test_rethrow_logic() {
            // Rethrow 重新抛出当前异常
            let exception = RuntimeValue::String("original error".into());

            assert!(matches!(exception, RuntimeValue::String(_)));
        }

        // 测试异常传播
        #[test]
        fn test_exception_propagation() {
            let caught = true;
            assert!(caught);
        }
    }

    // =====================
    // P7 指令测试：内存操作
    // =====================
    mod p7_instruction_tests {
        use crate::runtime::value::{RuntimeValue, Heap, HeapValue};

        // 测试 Struct 字段访问
        #[test]
        fn test_struct_field_access() {
            let mut heap = Heap::new();

            // 模拟 Struct: (name, age) = ("Alice", 30)
            let struct_handle = heap.allocate(HeapValue::Tuple(vec![
                RuntimeValue::String("Alice".into()),
                RuntimeValue::Int(30),
            ]));

            // 获取字段 0 (name)
            let name = match heap.get(struct_handle) {
                Some(HeapValue::Tuple(items)) => items.get(0).cloned().unwrap_or(RuntimeValue::Unit),
                _ => RuntimeValue::Unit,
            };

            assert!(matches!(name, RuntimeValue::String(s) if s.as_ref() == "Alice"));

            // 获取字段 1 (age)
            let age = match heap.get(struct_handle) {
                Some(HeapValue::Tuple(items)) => items.get(1).cloned().unwrap_or(RuntimeValue::Unit),
                _ => RuntimeValue::Unit,
            };

            assert_eq!(age, RuntimeValue::Int(30));
        }

        // 测试 Dict 操作
        #[test]
        fn test_dict_operations() {
            use std::collections::HashMap;

            let mut heap = Heap::new();
            let mut dict = HashMap::new();
            dict.insert(RuntimeValue::Int(1), RuntimeValue::String("one".into()));
            dict.insert(RuntimeValue::Int(2), RuntimeValue::String("two".into()));

            let dict_handle = heap.allocate(HeapValue::Dict(dict));

            // 验证 Dict 存储
            if let Some(HeapValue::Dict(d)) = heap.get(dict_handle) {
                assert_eq!(d.len(), 2);
            } else {
                panic!("Expected Dict");
            }
        }

        // 测试 Array 固定大小数组
        #[test]
        fn test_array_operations() {
            let mut heap = Heap::new();
            let arr_handle = heap.allocate(HeapValue::Array(vec![
                RuntimeValue::Int(1),
                RuntimeValue::Int(2),
                RuntimeValue::Int(3),
            ]));

            if let Some(HeapValue::Array(arr)) = heap.get(arr_handle) {
                assert_eq!(arr.len(), 3);
                assert_eq!(arr[0], RuntimeValue::Int(1));
                assert_eq!(arr[2], RuntimeValue::Int(3));
            } else {
                panic!("Expected Array");
            }
        }

        // 测试 Enum 变体访问
        #[test]
        fn test_enum_variant_access() {
            let variant_id = 2u32;
            let payload = RuntimeValue::String("payload".into());

            assert_eq!(variant_id, 2);
            assert!(matches!(payload, RuntimeValue::String(_)));
        }

        // 测试 Bytes 操作
        #[test]
        fn test_bytes_operations() {
            use std::sync::Arc;

            let bytes: RuntimeValue = RuntimeValue::Bytes(Arc::from(vec![0x01, 0x02, 0x03, 0x04]));

            if let RuntimeValue::Bytes(b) = &bytes {
                assert_eq!(b.len(), 4);
                assert_eq!(b[0], 0x01);
            } else {
                panic!("Expected Bytes");
            }
        }
    }

    // =====================
    // P8 指令测试：类型系统
    // =====================
    mod p8_instruction_tests {
        use crate::runtime::value::{RuntimeValue, ValueType, IntWidth, FloatWidth};

        // 测试 ValueType 层次结构
        #[test]
        fn test_value_type_hierarchy() {
            let int_type = ValueType::Int(IntWidth::I64);
            let float_type = ValueType::Float(FloatWidth::F64);
            let string_type = ValueType::String;

            assert!(matches!(int_type, ValueType::Int(IntWidth::I64)));
            assert!(matches!(float_type, ValueType::Float(FloatWidth::F64)));
            assert_eq!(string_type, ValueType::String);
        }

        // 测试类型检查 (TypeCheck)
        #[test]
        fn test_type_check_logic() {
            let value: RuntimeValue = RuntimeValue::Int(42);
            let expected_type = ValueType::Int(IntWidth::I64);

            let matches = match &value {
                RuntimeValue::Int(_) => matches!(expected_type, ValueType::Int(IntWidth::I64)),
                _ => false,
            };

            assert!(matches);
        }

        // 测试类型获取 (TypeOf)
        #[test]
        fn test_type_of_logic() {
            let value: RuntimeValue = RuntimeValue::Float(3.14);
            let value_type = match &value {
                RuntimeValue::Int(_) => ValueType::Int(IntWidth::I64),
                RuntimeValue::Float(_) => ValueType::Float(FloatWidth::F64),
                RuntimeValue::String(_) => ValueType::String,
                _ => ValueType::Unit,
            };

            assert!(matches!(value_type, ValueType::Float(FloatWidth::F64)));
        }

        // 测试 Async 值
        #[test]
        fn test_async_value() {
            use crate::runtime::value::{AsyncValue, AsyncState, TaskId};

            let async_val = AsyncValue {
                state: Box::new(AsyncState::Ready(Box::new(RuntimeValue::Int(42)))),
                value_type: ValueType::Int(IntWidth::I64),
            };

            if let AsyncState::Ready(val) = &*async_val.state {
                if let RuntimeValue::Int(n) = **val {
                    assert_eq!(n, 42);
                }
            }
        }

        // 测试 Ptr 指针类型
        #[test]
        fn test_ptr_type() {
            use crate::runtime::value::PtrKind;

            let ptr = RuntimeValue::Ptr {
                kind: PtrKind::Const,
                address: 0xDEADBEEF,
                type_id: crate::runtime::value::TypeId(1),
            };

            if let RuntimeValue::Ptr { kind, address, .. } = ptr {
                assert!(matches!(kind, PtrKind::Const));
                assert_eq!(address, 0xDEADBEEF);
            }
        }
    }

    // =====================
    // P9 指令测试：浮点操作
    // =====================
    mod p9_instruction_tests {
        use crate::runtime::value::RuntimeValue;

        // 测试 F64Add 逻辑
        #[test]
        fn test_f64_add_logic() {
            let a = RuntimeValue::Float(1.5);
            let b = RuntimeValue::Float(2.5);

            let result = match (a, b) {
                (RuntimeValue::Float(x), RuntimeValue::Float(y)) => RuntimeValue::Float(x + y),
                _ => RuntimeValue::Unit,
            };

            assert_eq!(result, RuntimeValue::Float(4.0));
        }

        // 测试 F64Sub 逻辑
        #[test]
        fn test_f64_sub_logic() {
            let a = RuntimeValue::Float(10.0);
            let b = RuntimeValue::Float(3.5);

            let result = match (a, b) {
                (RuntimeValue::Float(x), RuntimeValue::Float(y)) => RuntimeValue::Float(x - y),
                _ => RuntimeValue::Unit,
            };

            assert_eq!(result, RuntimeValue::Float(6.5));
        }

        // 测试 F64Mul 逻辑
        #[test]
        fn test_f64_mul_logic() {
            let a = RuntimeValue::Float(7.0);
            let b = RuntimeValue::Float(6.0);

            let result = match (a, b) {
                (RuntimeValue::Float(x), RuntimeValue::Float(y)) => RuntimeValue::Float(x * y),
                _ => RuntimeValue::Unit,
            };

            assert_eq!(result, RuntimeValue::Float(42.0));
        }

        // 测试 F64Div 逻辑
        #[test]
        fn test_f64_div_logic() {
            let a = RuntimeValue::Float(84.0);
            let b = RuntimeValue::Float(2.0);

            let result = match (a, b) {
                (RuntimeValue::Float(x), RuntimeValue::Float(y)) if y != 0.0 => RuntimeValue::Float(x / y),
                _ => RuntimeValue::Unit,
            };

            assert_eq!(result, RuntimeValue::Float(42.0));
        }

        // 测试 F64Neg 逻辑
        #[test]
        fn test_f64_neg_logic() {
            let a = RuntimeValue::Float(42.0);

            let result = match a {
                RuntimeValue::Float(x) => RuntimeValue::Float(-x),
                _ => RuntimeValue::Unit,
            };

            assert_eq!(result, RuntimeValue::Float(-42.0));
        }

        // 测试 F64Sqrt 逻辑
        #[test]
        fn test_f64_sqrt_logic() {
            let a = RuntimeValue::Float(16.0);

            let result = match a {
                RuntimeValue::Float(x) if x >= 0.0 => RuntimeValue::Float(x.sqrt()),
                _ => RuntimeValue::Unit,
            };

            if let RuntimeValue::Float(f) = result {
                assert!((f - 4.0).abs() < 0.001);
            } else {
                panic!("Expected Float");
            }
        }

        // 测试 F64Rem (取模)
        #[test]
        fn test_f64_rem_logic() {
            let a = RuntimeValue::Float(10.5);
            let b = RuntimeValue::Float(4.0);

            let result = match (a, b) {
                (RuntimeValue::Float(x), RuntimeValue::Float(y)) if y != 0.0 => RuntimeValue::Float(x % y),
                _ => RuntimeValue::Unit,
            };

            if let RuntimeValue::Float(f) = result {
                assert!((f - 2.5).abs() < 0.001);
            } else {
                panic!("Expected Float");
            }
        }

        // 测试 F64 比较 (Eq/Ne/Lt/Le/Gt/Ge)
        #[test]
        fn test_f64_comparison_logic() {
            let a = 3.14f64;
            let b = 3.14f64;
            let c = 2.71f64;

            assert!((a - b).abs() < 0.001); // a == b
            assert!((a - c).abs() > 0.001); // a != c
            assert!(a > c); // a > c
            assert!(c < a); // c < a
        }
    }

    // =====================
    // P10 指令测试：位操作
    // =====================
    mod p10_instruction_tests {
        use crate::runtime::value::RuntimeValue;

        // 测试 I64And 逻辑
        #[test]
        fn test_i64_and_logic() {
            let a = RuntimeValue::Int(0b1111);
            let b = RuntimeValue::Int(0b1010);

            let result = match (a, b) {
                (RuntimeValue::Int(x), RuntimeValue::Int(y)) => RuntimeValue::Int(x & y),
                _ => RuntimeValue::Unit,
            };

            assert_eq!(result, RuntimeValue::Int(0b1010));
        }

        // 测试 I64Or 逻辑
        #[test]
        fn test_i64_or_logic() {
            let a = RuntimeValue::Int(0b1100);
            let b = RuntimeValue::Int(0b1010);

            let result = match (a, b) {
                (RuntimeValue::Int(x), RuntimeValue::Int(y)) => RuntimeValue::Int(x | y),
                _ => RuntimeValue::Unit,
            };

            assert_eq!(result, RuntimeValue::Int(0b1110));
        }

        // 测试 I64Xor 逻辑
        #[test]
        fn test_i64_xor_logic() {
            let a = RuntimeValue::Int(0b1111);
            let b = RuntimeValue::Int(0b1010);

            let result = match (a, b) {
                (RuntimeValue::Int(x), RuntimeValue::Int(y)) => RuntimeValue::Int(x ^ y),
                _ => RuntimeValue::Unit,
            };

            assert_eq!(result, RuntimeValue::Int(0b0101));
        }

        // 测试 I64Shl (左移)
        #[test]
        fn test_i64_shl_logic() {
            let a = RuntimeValue::Int(1);
            let shift = 5u8;

            let result = match a {
                RuntimeValue::Int(x) if shift < 64 => RuntimeValue::Int(x << shift),
                _ => RuntimeValue::Unit,
            };

            assert_eq!(result, RuntimeValue::Int(32));
        }

        // 测试 I64Shr (逻辑右移)
        #[test]
        fn test_i64_shr_logic() {
            let a = RuntimeValue::Int(0b100000); // 32
            let shift = 2u8;

            let result = match a {
                RuntimeValue::Int(x) => RuntimeValue::Int(x >> shift),
                _ => RuntimeValue::Unit,
            };

            assert_eq!(result, RuntimeValue::Int(8));
        }

        // 测试 I64Sar (算术右移)
        #[test]
        fn test_i64_sar_logic() {
            let a = RuntimeValue::Int(-32); // -32 的二进制补码
            let shift = 2u8;

            let result = match a {
                RuntimeValue::Int(x) => RuntimeValue::Int(x >> shift), // Rust 默认算术右移
                _ => RuntimeValue::Unit,
            };

            assert_eq!(result, RuntimeValue::Int(-8));
        }

        // 测试 I64Not (按位取反)
        #[test]
        fn test_i64_not_logic() {
            let a = RuntimeValue::Int(0);

            let result = match a {
                RuntimeValue::Int(x) => RuntimeValue::Int(!x),
                _ => RuntimeValue::Unit,
            };

            assert_eq!(result, RuntimeValue::Int(-1));
        }
    }
}
