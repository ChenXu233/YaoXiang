//! 跨 spawn 循环引用检测测试

#[cfg(test)]
mod tests {
    use crate::middle::ir::{BasicBlock, FunctionIR, Instruction, Operand};
    use crate::middle::lifetime::cycle_check::CycleChecker;
    use crate::middle::lifetime::OwnershipError;

    /// 创建一个简单的 spawn 用于测试
    fn make_spawn_instr(
        func: Operand,
        args: Vec<Operand>,
        result: Operand,
    ) -> Instruction {
        Instruction::Spawn {
            func,
            args,
            result,
        }
    }

    /// 创建一个 Move 指令用于追踪
    fn make_move_instr(dst: Operand, src: Operand) -> Instruction {
        Instruction::Move { dst, src }
    }

    /// 创建一个 ArcNew 指令（ref）
    fn make_arc_new_instr(dst: Operand, src: Operand) -> Instruction {
        Instruction::ArcNew { dst, src }
    }

    /// 创建空的 FunctionIR 用于测试
    fn make_empty_func() -> FunctionIR {
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

    #[test]
    fn test_no_cycle_simple() {
        // 没有循环的情况
        let mut checker = CycleChecker::new();
        let mut func = make_empty_func();

        // a = spawn(task_a())
        // b = spawn(task_b(ref a))
        // a 不持有 b，b 持有 a（单向，不是环）
        func.blocks[0].instructions = vec![
            make_spawn_instr(Operand::Local(0), vec![], Operand::Local(2)), // a = spawn(task_a)
            make_spawn_instr(Operand::Local(1), vec![Operand::Local(2)], Operand::Local(3)), // b = spawn(task_b(ref a))
        ];

        let errors = checker.check_function(&func);
        assert!(errors.is_empty(), "应该没有错误：单向引用不是环");
    }

    #[test]
    fn test_no_cycle_pool() {
        // 工作池模式：多个任务共享同一个 ref，不是环
        let mut checker = CycleChecker::new();
        let mut func = make_empty_func();

        // shared = ref config
        // workers = spawn for i in 0..10 { process(shared) }
        // 多个 spawn 都 ref shared，但 shared 不 ref 它们（扇出，不是环）
        func.blocks[0].instructions = vec![
            make_arc_new_instr(Operand::Local(0), Operand::Local(10)), // shared = ref config
            make_spawn_instr(Operand::Local(1), vec![Operand::Local(0)], Operand::Local(2)),
            make_spawn_instr(Operand::Local(3), vec![Operand::Local(0)], Operand::Local(4)),
            make_spawn_instr(Operand::Local(5), vec![Operand::Local(0)], Operand::Local(6)),
        ];

        let errors = checker.check_function(&func);
        assert!(errors.is_empty(), "应该没有错误：工作池是扇出，不是环");
    }

    #[test]
    fn test_cycle_two_spawns() {
        // 两个 spawn 互相 ref，形成环
        let mut checker = CycleChecker::new();
        let mut func = make_empty_func();

        // a = spawn(task_a())      // 返回值持有 b
        // b = spawn(task_b(ref a)) // 参数是 a，返回值持有 a
        // a → b 且 b → a，形成环
        func.blocks[0].instructions = vec![
            // a = spawn(task_a())
            make_spawn_instr(Operand::Local(0), vec![], Operand::Local(2)),
            // b = spawn(task_b(ref a))
            make_spawn_instr(Operand::Local(1), vec![Operand::Local(2)], Operand::Local(3)),
            // a 持有 b 的 ref（通过 Move）
            make_move_instr(Operand::Local(2), Operand::Local(3)),
        ];

        let errors = checker.check_function(&func);
        assert!(!errors.is_empty(), "应该检测到循环：a 和 b 互相 ref");

        let has_cycle_error = errors.iter().any(|e| {
            matches!(e, OwnershipError::CrossSpawnCycle { .. })
        });
        assert!(has_cycle_error, "应该有 CrossSpawnCycle 错误");
    }

    #[test]
    fn test_cycle_three_spawns() {
        // 三个 spawn 形成环：A → B → C → A
        let mut checker = CycleChecker::new();
        let mut func = make_empty_func();

        // a = spawn(task_a())        // 返回值持有 c
        // b = spawn(task_b(ref a))   // 参数是 a
        // c = spawn(task_c(ref b))   // 参数是 b，返回值持有 b
        // c → b, b → a, a → c，形成环
        func.blocks[0].instructions = vec![
            make_spawn_instr(Operand::Local(0), vec![], Operand::Local(3)), // a = spawn(task_a)
            make_spawn_instr(Operand::Local(1), vec![Operand::Local(3)], Operand::Local(4)), // b = spawn(task_b(ref a))
            make_spawn_instr(Operand::Local(2), vec![Operand::Local(4)], Operand::Local(5)), // c = spawn(task_c(ref b))
            // c 持有 b 的 ref
            make_move_instr(Operand::Local(5), Operand::Local(4)),
            // a 持有 c 的 ref
            make_move_instr(Operand::Local(3), Operand::Local(5)),
        ];

        let errors = checker.check_function(&func);
        let has_cycle_error = errors.iter().any(|e| {
            matches!(e, OwnershipError::CrossSpawnCycle { .. })
        });
        assert!(has_cycle_error, "应该检测到三节点环");
    }

    #[test]
    fn test_no_cycle_chain() {
        // 链式引用，不是环
        let mut checker = CycleChecker::new();
        let mut func = make_empty_func();

        // a = spawn(task_a())        // 返回值持有 a
        // b = spawn(task_b(ref a))   // 参数是 a，返回值持有 b
        // c = spawn(task_c(ref b))   // 参数是 b，返回值持有 c
        // a → b → c，单向链，不是环
        func.blocks[0].instructions = vec![
            make_spawn_instr(Operand::Local(0), vec![], Operand::Local(3)),
            make_spawn_instr(Operand::Local(1), vec![Operand::Local(3)], Operand::Local(4)),
            make_spawn_instr(Operand::Local(2), vec![Operand::Local(4)], Operand::Local(5)),
        ];

        let errors = checker.check_function(&func);
        assert!(errors.is_empty(), "应该没有错误：链式引用不是环");
    }

    #[test]
    fn test_empty_function() {
        // 空函数，没有 spawn
        let mut checker = CycleChecker::new();
        let func = make_empty_func();

        let errors = checker.check_function(&func);
        assert!(errors.is_empty(), "空函数应该没有错误");
    }

    #[test]
    fn test_single_spawn() {
        // 只有一个 spawn，不可能形成环
        let mut checker = CycleChecker::new();
        let mut func = make_empty_func();

        func.blocks[0].instructions = vec![
            make_spawn_instr(Operand::Local(0), vec![], Operand::Local(1)),
        ];

        let errors = checker.check_function(&func);
        assert!(errors.is_empty(), "单个 spawn 不可能形成环");
    }

    #[test]
    fn test_complex_move_chain() {
        // 复杂 Move 链：a -> tmp -> b，b 应该能追溯到 a
        let mut checker = CycleChecker::new();
        let mut func = make_empty_func();

        // a = spawn(task_a())
        // tmp = Move a
        // b = spawn(task_b(ref tmp))
        // tmp2 = Move a
        // c = spawn(task_c(ref tmp2))
        // c 持有 a（不是环，因为 a 不持有 c）
        func.blocks[0].instructions = vec![
            make_spawn_instr(Operand::Local(0), vec![], Operand::Local(3)),
            make_move_instr(Operand::Local(1), Operand::Local(3)), // tmp = Move a
            make_spawn_instr(Operand::Local(2), vec![Operand::Local(1)], Operand::Local(4)), // b = spawn(task_b(ref tmp))
            make_move_instr(Operand::Local(5), Operand::Local(3)), // tmp2 = Move a
            make_spawn_instr(Operand::Local(6), vec![Operand::Local(5)], Operand::Local(7)), // c = spawn(task_c(ref tmp2))
        ];

        let errors = checker.check_function(&func);
        assert!(errors.is_empty(), "应该没有错误：复杂 Move 链不是环");
    }

    #[test]
    fn test_fan_in_pattern() {
        // 扇入模式：多个 spawn 持有同一个 spawn 的 ref，不是环
        let mut checker = CycleChecker::new();
        let mut func = make_empty_func();

        // a = spawn(task_a())  // a 持有 x
        // b = spawn(task_b(ref a))
        // c = spawn(task_c(ref a))
        // d = spawn(task_d(ref a))
        // a 不持有 b、c、d（扇入，不是环）
        func.blocks[0].instructions = vec![
            make_spawn_instr(Operand::Local(0), vec![], Operand::Local(5)), // a
            make_spawn_instr(Operand::Local(1), vec![Operand::Local(5)], Operand::Local(6)), // b
            make_spawn_instr(Operand::Local(2), vec![Operand::Local(5)], Operand::Local(7)), // c
            make_spawn_instr(Operand::Local(3), vec![Operand::Local(5)], Operand::Local(8)), // d
        ];

        let errors = checker.check_function(&func);
        assert!(errors.is_empty(), "应该没有错误：扇入模式不是环");
    }

    #[test]
    fn test_self_reference() {
        // 自引用：spawn 的返回值持有自己的 ref，不是环（但可能有其他问题）
        let mut checker = CycleChecker::new();
        let mut func = make_empty_func();

        // a = spawn(task_a())
        // a = Move a  // 自己持有自己
        func.blocks[0].instructions = vec![
            make_spawn_instr(Operand::Local(0), vec![], Operand::Local(2)),
            make_move_instr(Operand::Local(2), Operand::Local(2)), // 自引用
        ];

        let errors = checker.check_function(&func);
        // 自引用不构成跨 spawn 环
        assert!(errors.is_empty(), "应该没有错误：自引用不是跨 spawn 环");
    }

    #[test]
    fn test_broken_cycle() {
        // 中断的环：看似有环但被中断
        let mut checker = CycleChecker::new();
        let mut func = make_empty_func();

        // a = spawn(task_a())  // 持有 c
        // b = spawn(task_b(ref a)) // 参数是 a
        // c = spawn(task_c(ref b)) // 参数是 b，但返回值不持有任何 spawn
        // a → c，但 c 不持有 a（环被中断）
        func.blocks[0].instructions = vec![
            make_spawn_instr(Operand::Local(0), vec![], Operand::Local(4)), // a
            make_spawn_instr(Operand::Local(1), vec![Operand::Local(4)], Operand::Local(5)), // b
            make_spawn_instr(Operand::Local(2), vec![Operand::Local(5)], Operand::Local(6)), // c
            // c 的返回值是 Local(6)，但 Move 指向 Local(10) 不是 spawn 结果
            make_move_instr(Operand::Local(4), Operand::Local(10)), // a 持有非 spawn 值
        ];

        let errors = checker.check_function(&func);
        assert!(errors.is_empty(), "应该没有错误：环被中断");
    }

    #[test]
    fn test_four_node_cycle() {
        // 四节点环：A → B → C → D → A
        let mut checker = CycleChecker::new();
        let mut func = make_empty_func();

        // a = spawn(task_a())      // 持有 d
        // b = spawn(task_b(ref a)) // 参数是 a
        // c = spawn(task_c(ref b)) // 参数是 b，持有 d
        // d = spawn(task_d(ref c)) // 参数是 c
        // a → d, d → c, c → b, b → a，形成环
        func.blocks[0].instructions = vec![
            make_spawn_instr(Operand::Local(0), vec![], Operand::Local(5)), // a
            make_spawn_instr(Operand::Local(1), vec![Operand::Local(5)], Operand::Local(6)), // b
            make_spawn_instr(Operand::Local(2), vec![Operand::Local(6)], Operand::Local(7)), // c
            make_spawn_instr(Operand::Local(3), vec![Operand::Local(7)], Operand::Local(8)), // d
            make_move_instr(Operand::Local(5), Operand::Local(8)), // a 持有 d
            make_move_instr(Operand::Local(7), Operand::Local(8)), // c 持有 d
        ];

        let errors = checker.check_function(&func);
        let has_cycle_error = errors.iter().any(|e| {
            matches!(e, OwnershipError::CrossSpawnCycle { .. })
        });
        assert!(has_cycle_error, "应该检测到四节点环");
    }

    #[test]
    fn test_multiple_refs_to_same_spawn() {
        // 同一个 spawn 被多次持有，但不形成环
        let mut checker = CycleChecker::new();
        let mut func = make_empty_func();

        // a = spawn(task_a())      // 持有 c
        // b = spawn(task_b(ref a)) // 参数是 a
        // c = spawn(task_c(ref a)) // 参数是 a，持有 a
        // a ← c（c 持有 a），a 不持有 c（只有单向）
        func.blocks[0].instructions = vec![
            make_spawn_instr(Operand::Local(0), vec![], Operand::Local(4)), // a
            make_spawn_instr(Operand::Local(1), vec![Operand::Local(4)], Operand::Local(5)), // b
            make_spawn_instr(Operand::Local(2), vec![Operand::Local(4)], Operand::Local(6)), // c
            make_move_instr(Operand::Local(6), Operand::Local(4)), // c 持有 a
        ];

        let errors = checker.check_function(&func);
        assert!(errors.is_empty(), "应该没有错误：多次持有但不形成环");
    }

    #[test]
    fn test_cycle_with_common_ancestor() {
        // 共同祖先模式：a 被 b 和 c 同时持有，但 b 和 c 不互相持有
        let mut checker = CycleChecker::new();
        let mut func = make_empty_func();

        // a = spawn(task_a())      // 不持有任何人
        // b = spawn(task_b(ref a)) // 参数是 a
        // c = spawn(task_c(ref a)) // 参数是 a
        // b 和 c 都 ref a，但彼此不形成环
        func.blocks[0].instructions = vec![
            make_spawn_instr(Operand::Local(0), vec![], Operand::Local(4)),
            make_spawn_instr(Operand::Local(1), vec![Operand::Local(4)], Operand::Local(5)),
            make_spawn_instr(Operand::Local(2), vec![Operand::Local(4)], Operand::Local(6)),
        ];

        let errors = checker.check_function(&func);
        assert!(errors.is_empty(), "应该没有错误：共同祖先模式不是环");
    }
}
