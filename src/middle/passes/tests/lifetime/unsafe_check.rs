//! unsafe 语义检查器测试

#[cfg(test)]
mod tests {
    use crate::middle::core::ir::{BasicBlock, FunctionIR, Instruction, Operand};
    use crate::middle::passes::lifetime::unsafe_check::UnsafeChecker;
    use crate::middle::passes::lifetime::OwnershipError;

    /// 创建简单的 FunctionIR
    fn make_func() -> FunctionIR {
        FunctionIR {
            name: "test_func".to_string(),
            params: vec![],
            return_type: crate::frontend::typecheck::MonoType::Void,
            is_async: false,
            locals: vec![],
            blocks: vec![BasicBlock {
                label: 0,
                instructions: vec![],
                successors: vec![],
            }],
            entry: 0,
        }
    }

    // ========== unsafe 块范围收集测试 ==========

    #[test]
    fn test_unsafe_block_ranges() {
        let mut checker = UnsafeChecker::new();
        let mut func = make_func();

        // 添加 unsafe 块
        func.blocks[0].instructions = vec![
            Instruction::UnsafeBlockStart,
            Instruction::PtrDeref {
                dst: Operand::Local(0),
                src: Operand::Local(1),
            },
            Instruction::UnsafeBlockEnd,
        ];

        let errors = checker.check_function(&func);
        assert!(errors.is_empty(), "unsafe 块内的操作不应报错");
    }

    #[test]
    fn test_no_unsafe_block_deref() {
        let mut checker = UnsafeChecker::new();
        let mut func = make_func();

        // 没有 unsafe 块的解引用
        func.blocks[0].instructions = vec![Instruction::PtrDeref {
            dst: Operand::Local(0),
            src: Operand::Local(1),
        }];

        let errors = checker.check_function(&func);
        assert!(!errors.is_empty(), "unsafe 块外的解引用应该报错");
        assert!(matches!(errors[0], OwnershipError::UnsafeDeref { .. }));
    }

    #[test]
    fn test_unsafe_block_start_end_matching() {
        let mut checker = UnsafeChecker::new();
        let mut func = make_func();

        // 嵌套的 unsafe 块
        func.blocks[0].instructions = vec![
            Instruction::UnsafeBlockStart,
            Instruction::Load {
                dst: Operand::Local(0),
                src: Operand::Const(crate::middle::core::ir::ConstValue::Int(1)),
            },
            Instruction::UnsafeBlockStart,
            Instruction::PtrDeref {
                dst: Operand::Local(1),
                src: Operand::Local(2),
            },
            Instruction::UnsafeBlockEnd,
            Instruction::UnsafeBlockEnd,
        ];

        let errors = checker.check_function(&func);
        assert!(errors.is_empty(), "嵌套 unsafe 块内的操作不应报错");
    }

    #[test]
    fn test_ptr_from_ref_requires_unsafe() {
        let mut checker = UnsafeChecker::new();
        let mut func = make_func();

        // 裸指针创建在 unsafe 块外
        func.blocks[0].instructions = vec![Instruction::PtrFromRef {
            dst: Operand::Local(0),
            src: Operand::Local(1),
        }];

        let errors = checker.check_function(&func);
        assert!(!errors.is_empty(), "unsafe 块外的 PtrFromRef 应该报错");
    }

    #[test]
    fn test_ptr_store_requires_unsafe() {
        let mut checker = UnsafeChecker::new();
        let mut func = make_func();

        // 指针存储在 unsafe 块外
        func.blocks[0].instructions = vec![Instruction::PtrStore {
            dst: Operand::Local(0),
            src: Operand::Local(1),
        }];

        let errors = checker.check_function(&func);
        assert!(!errors.is_empty(), "unsafe 块外的 PtrStore 应该报错");
    }

    #[test]
    fn test_ptr_load_requires_unsafe() {
        let mut checker = UnsafeChecker::new();
        let mut func = make_func();

        // 指针加载在 unsafe 块外
        func.blocks[0].instructions = vec![Instruction::PtrLoad {
            dst: Operand::Local(0),
            src: Operand::Local(1),
        }];

        let errors = checker.check_function(&func);
        assert!(!errors.is_empty(), "unsafe 块外的 PtrLoad 应该报错");
    }

    #[test]
    fn test_mixed_unsafe_safe_operations() {
        let mut checker = UnsafeChecker::new();
        let mut func = make_func();

        // 混合安全和不安全操作
        func.blocks[0].instructions = vec![
            // 安全操作
            Instruction::Load {
                dst: Operand::Local(0),
                src: Operand::Const(crate::middle::core::ir::ConstValue::Int(1)),
            },
            Instruction::Add {
                dst: Operand::Local(1),
                lhs: Operand::Local(0),
                rhs: Operand::Const(crate::middle::core::ir::ConstValue::Int(2)),
            },
            // 进入 unsafe 块
            Instruction::UnsafeBlockStart,
            Instruction::PtrDeref {
                dst: Operand::Local(2),
                src: Operand::Local(3),
            },
            Instruction::UnsafeBlockEnd,
            // 退出 unsafe 块后的安全操作
            Instruction::Load {
                dst: Operand::Local(4),
                src: Operand::Const(crate::middle::core::ir::ConstValue::Int(3)),
            },
        ];

        let errors = checker.check_function(&func);
        assert!(errors.is_empty(), "混合操作不应有错误");
    }

    #[test]
    fn test_unsafe_checker_state_reset() {
        let mut checker = UnsafeChecker::new();
        let mut func = make_func();

        // 第一次检查
        func.blocks[0].instructions = vec![Instruction::UnsafeBlockStart];
        let _ = checker.check_function(&func);

        // 重置后检查不同函数
        let mut func2 = make_func();
        func2.blocks[0].instructions = vec![
            Instruction::UnsafeBlockStart,
            Instruction::PtrDeref {
                dst: Operand::Local(0),
                src: Operand::Local(1),
            },
            Instruction::UnsafeBlockEnd,
        ];

        let errors = checker.check_function(&func2);
        assert!(errors.is_empty(), "检查器状态应该正确重置");
    }

    #[test]
    fn test_error_message_format() {
        let mut checker = UnsafeChecker::new();
        let mut func = make_func();

        func.blocks[0].instructions = vec![Instruction::PtrDeref {
            dst: Operand::Local(0),
            src: Operand::Local(1),
        }];

        let errors = checker.check_function(&func);
        assert!(!errors.is_empty());

        let error_str = format!("{}", errors[0]);
        assert!(
            error_str.contains("UnsafeDeref"),
            "错误消息应包含 UnsafeDeref: {}",
            error_str
        );
    }
}
