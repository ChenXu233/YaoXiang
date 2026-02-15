//! 链式调用分析
//!
//! 分析方法链的所有权流动，支持：
//! - `p.rotate(90).scale(2.0).translate(1.0)`
//! - 追踪链中每个调用的消费模式
//! - 验证所有权在链中的正确流动

use super::ownership_flow::ConsumeMode;
use crate::middle::core::ir::{FunctionIR, Instruction, Operand};

/// 链式调用中的方法信息
#[derive(Debug, Clone)]
pub struct MethodInfo {
    /// 方法名
    pub name: String,
    /// 接收者操作数
    pub receiver: Operand,
    /// 其他参数
    pub args: Vec<Operand>,
    /// 消费模式
    pub consume_mode: ConsumeMode,
    /// 返回值操作数（如果有）
    pub result: Option<Operand>,
}

/// 链式调用分析结果
#[derive(Debug, Clone)]
pub struct ChainAnalysisResult {
    /// 链中的方法列表
    pub methods: Vec<MethodInfo>,
    /// 初始接收者
    pub initial_receiver: Operand,
    /// 最终返回值（用于赋值）
    pub final_result: Option<Operand>,
    /// 所有权是否正确闭合
    pub ownership_closed: bool,
}

/// 链式调用分析器
///
/// 分析方法链的所有权流动：
/// - 识别链中的每个方法调用
/// - 追踪接收者和参数的所有权
/// - 验证所有权闭合
///
/// # 示例
///
/// ```ignore
/// let p = Point(1.0, 2.0);
/// p.rotate(90)    // Method 1: rotate, Returns
///   .scale(2.0)   // Method 2: scale, Returns
///   .translate(1.0); // Method 3: translate, Consumes
/// ```
#[derive(Debug, Clone)]
pub struct ChainCallAnalyzer {
    /// 初始接收者
    initial_receiver: Option<Operand>,
    /// 方法链
    methods: Vec<MethodInfo>,
}

impl ChainCallAnalyzer {
    /// 创建新的链式调用分析器
    pub fn new() -> Self {
        Self {
            initial_receiver: None,
            methods: Vec::new(),
        }
    }

    /// 分析链式调用
    ///
    /// 从初始接收者开始，分析整个方法链的所有权流动
    pub fn analyze_chain(
        &mut self,
        receiver: Operand,
        calls: &[Instruction],
    ) -> ChainAnalysisResult {
        self.initial_receiver = Some(receiver.clone());
        self.methods.clear();

        // 使用可变接收者追踪链
        let mut current_receiver = receiver;

        // 分析每个方法调用
        for call in calls {
            current_receiver = self.analyze_call(call, &current_receiver);
        }

        // 构建结果
        self.build_result()
    }

    /// 分析单个方法调用
    fn analyze_call(
        &mut self,
        call: &Instruction,
        prev_result: &Operand,
    ) -> Operand {
        match call {
            Instruction::CallVirt {
                dst,
                obj,
                method_name,
                args,
            } => {
                // 检查是否是链式调用（obj 等于上一个结果）
                let is_chain_call = self.is_same_operand(obj, prev_result);

                if is_chain_call {
                    // 记录方法信息
                    let method_info = MethodInfo {
                        name: method_name.clone(),
                        receiver: obj.clone(),
                        args: args.clone(),
                        consume_mode: ConsumeMode::Undetermined, // 后续填充
                        result: dst.clone(),
                    };
                    self.methods.push(method_info);

                    // 返回值作为下一个方法的接收者
                    dst.clone().unwrap_or(prev_result.clone())
                } else {
                    // 不是链式调用，返回原始接收者
                    prev_result.clone()
                }
            }

            Instruction::Call { dst, args, .. } => {
                // 普通函数调用
                // 检查第一个参数是否是上一个结果（类似链式调用）
                if let Some(first_arg) = args.first() {
                    if self.is_same_operand(first_arg, prev_result) {
                        let method_info = MethodInfo {
                            name: "call".to_string(),
                            receiver: first_arg.clone(),
                            args: args.clone(),
                            consume_mode: ConsumeMode::Undetermined,
                            result: dst.clone(),
                        };
                        self.methods.push(method_info);
                        return dst.clone().unwrap_or(prev_result.clone());
                    }
                }
                prev_result.clone()
            }

            _ => prev_result.clone(),
        }
    }

    /// 检查两个操作数是否相同（简化版本）
    fn is_same_operand(
        &self,
        a: &Operand,
        b: &Operand,
    ) -> bool {
        match (a, b) {
            (Operand::Temp(i1), Operand::Temp(i2)) => i1 == i2,
            (Operand::Arg(i1), Operand::Arg(i2)) => i1 == i2,
            (Operand::Local(i1), Operand::Local(i2)) => i1 == i2,
            (Operand::Global(i1), Operand::Global(i2)) => i1 == i2,
            _ => false,
        }
    }

    /// 构建分析结果
    fn build_result(&self) -> ChainAnalysisResult {
        let final_result = self.methods.last().and_then(|m| m.result.clone());
        let ownership_closed = self.methods.last().is_none_or(|last| {
            // 如果最后一个方法是 Consumes 模式，所有权正确闭合
            // 或者方法是 Returns 模式但返回值被使用
            matches!(last.consume_mode, ConsumeMode::Consumes)
                || matches!(last.consume_mode, ConsumeMode::Returns)
        });

        ChainAnalysisResult {
            methods: self.methods.clone(),
            initial_receiver: self.initial_receiver.clone().unwrap_or(Operand::Temp(0)),
            final_result,
            ownership_closed,
        }
    }

    /// 从函数 IR 中提取链式调用
    ///
    /// 遍历函数的指令序列，提取连续的虚方法调用
    pub fn extract_chain_calls(
        &self,
        func: &FunctionIR,
        start_idx: usize,
    ) -> Vec<Instruction> {
        let mut calls = Vec::new();
        let mut current_idx = start_idx;

        // 查找连续的 CallVirt 指令
        while current_idx < func.blocks.len() {
            let block = &func.blocks[current_idx];
            for instr in &block.instructions {
                if let Instruction::CallVirt { .. } = instr {
                    calls.push(instr.clone());
                } else {
                    // 非 CallVirt 指令，停止收集
                    return calls;
                }
            }
            current_idx += 1;
        }

        calls
    }

    /// 分析单个函数调用的消费模式
    ///
    /// 基于调用方式推断消费模式：
    /// - 结果被赋值 → Returns 模式
    /// - 结果不被使用 → Consumes 模式
    /// - 无法确定 → Undetermined
    pub fn infer_consume_mode(
        &self,
        call: &Instruction,
        result_used: bool,
    ) -> ConsumeMode {
        match call {
            Instruction::CallVirt { .. } => {
                if result_used {
                    // 返回值被使用，推断为 Returns
                    ConsumeMode::Returns
                } else {
                    // 返回值未被使用，推断为 Consumes
                    ConsumeMode::Consumes
                }
            }
            Instruction::Call { .. } => {
                if result_used {
                    ConsumeMode::Returns
                } else {
                    ConsumeMode::Consumes
                }
            }
            _ => ConsumeMode::Undetermined,
        }
    }

    /// 分析链中所有权是否正确闭合
    ///
    /// 检查链式调用中所有权是否正确流动：
    /// - 每个方法的所有权应该被正确消费或返回
    /// - 最后一个方法的返回值应该被正确处理
    pub fn check_ownership_closure(
        &self,
        chain: &ChainAnalysisResult,
    ) -> bool {
        if chain.methods.is_empty() {
            return true; // 空链，没有所有权问题
        }

        // 检查最后一个方法
        if let Some(last) = chain.methods.last() {
            // 如果最后一个方法是 Consumes 模式，所有权正确闭合
            // 如果是 Returns 模式，返回值应该被使用
            match last.consume_mode {
                ConsumeMode::Consumes => true,
                ConsumeMode::Returns => chain.final_result.is_some(),
                ConsumeMode::Undetermined => true, // 保守估计
            }
        } else {
            true
        }
    }
}

impl Default for ChainCallAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::middle::core::ir::{BasicBlock, ConstValue, FunctionIR};
    use crate::frontend::typecheck::MonoType;

    fn create_test_func_with_calls(calls: Vec<Instruction>) -> FunctionIR {
        FunctionIR {
            name: "test_chain".to_string(),
            params: vec![],
            return_type: MonoType::Void,
            is_async: false,
            locals: vec![],
            blocks: vec![BasicBlock {
                label: 0,
                instructions: calls,
                successors: vec![],
            }],
            entry: 0,
        }
    }

    #[test]
    fn test_extract_chain_calls() {
        let analyzer = ChainCallAnalyzer::new();

        let calls = vec![
            Instruction::CallVirt {
                dst: Some(Operand::Temp(1)),
                obj: Operand::Temp(0),
                method_name: "rotate".to_string(),
                args: vec![Operand::Const(ConstValue::Int(90))],
            },
            Instruction::CallVirt {
                dst: Some(Operand::Temp(2)),
                obj: Operand::Temp(1),
                method_name: "scale".to_string(),
                args: vec![Operand::Const(ConstValue::Int(2))],
            },
        ];

        let func = create_test_func_with_calls(calls);
        let extracted = analyzer.extract_chain_calls(&func, 0);

        assert_eq!(extracted.len(), 2);
    }

    #[test]
    fn test_infer_consume_mode() {
        let analyzer = ChainCallAnalyzer::new();

        let call = Instruction::CallVirt {
            dst: Some(Operand::Temp(1)),
            obj: Operand::Temp(0),
            method_name: "rotate".to_string(),
            args: vec![],
        };

        // 返回值被使用
        assert_eq!(
            analyzer.infer_consume_mode(&call, true),
            ConsumeMode::Returns
        );

        // 返回值未被使用
        assert_eq!(
            analyzer.infer_consume_mode(&call, false),
            ConsumeMode::Consumes
        );
    }

    #[test]
    fn test_check_ownership_closure() {
        let analyzer = ChainCallAnalyzer::new();

        // Consumes 模式，所有权闭合
        let chain_consumes = ChainAnalysisResult {
            methods: vec![MethodInfo {
                name: "consume".to_string(),
                receiver: Operand::Temp(0),
                args: vec![],
                consume_mode: ConsumeMode::Consumes,
                result: None,
            }],
            initial_receiver: Operand::Temp(0),
            final_result: None,
            ownership_closed: false,
        };
        assert!(analyzer.check_ownership_closure(&chain_consumes));

        // Returns 模式，返回值被使用
        let chain_returns = ChainAnalysisResult {
            methods: vec![MethodInfo {
                name: "transform".to_string(),
                receiver: Operand::Temp(0),
                args: vec![],
                consume_mode: ConsumeMode::Returns,
                result: Some(Operand::Temp(1)),
            }],
            initial_receiver: Operand::Temp(0),
            final_result: Some(Operand::Temp(1)),
            ownership_closed: false,
        };
        assert!(analyzer.check_ownership_closure(&chain_returns));
    }

    #[test]
    fn test_long_chain_calls() {
        // 测试长链式调用: p.rotate(90).scale(2.0).translate(1.0)
        let mut analyzer = ChainCallAnalyzer::new();

        let calls = vec![
            Instruction::CallVirt {
                dst: Some(Operand::Temp(1)),
                obj: Operand::Temp(0),
                method_name: "rotate".to_string(),
                args: vec![Operand::Const(ConstValue::Int(90))],
            },
            Instruction::CallVirt {
                dst: Some(Operand::Temp(2)),
                obj: Operand::Temp(1),
                method_name: "scale".to_string(),
                args: vec![Operand::Const(ConstValue::Int(2))],
            },
            Instruction::CallVirt {
                dst: Some(Operand::Temp(3)),
                obj: Operand::Temp(2),
                method_name: "translate".to_string(),
                args: vec![Operand::Const(ConstValue::Int(1))],
            },
        ];

        let result = analyzer.analyze_chain(Operand::Temp(0), &calls);

        assert_eq!(result.methods.len(), 3);
        assert_eq!(result.methods[0].name, "rotate");
        assert_eq!(result.methods[1].name, "scale");
        assert_eq!(result.methods[2].name, "translate");
    }

    #[test]
    fn test_chain_with_different_temp_indices() {
        // 测试链中临时变量索引的正确追踪
        let mut analyzer = ChainCallAnalyzer::new();

        let calls = vec![
            Instruction::CallVirt {
                dst: Some(Operand::Temp(100)),
                obj: Operand::Temp(0),
                method_name: "method1".to_string(),
                args: vec![],
            },
            Instruction::CallVirt {
                dst: Some(Operand::Temp(200)),
                obj: Operand::Temp(100),
                method_name: "method2".to_string(),
                args: vec![],
            },
        ];

        let result = analyzer.analyze_chain(Operand::Temp(0), &calls);

        assert_eq!(result.methods.len(), 2);
        assert_eq!(result.methods[0].name, "method1");
        assert_eq!(result.methods[1].name, "method2");
    }

    #[test]
    fn test_chain_with_args() {
        // 测试带参数的方法链
        let mut analyzer = ChainCallAnalyzer::new();

        let calls = vec![Instruction::CallVirt {
            dst: Some(Operand::Temp(1)),
            obj: Operand::Temp(0),
            method_name: "rotate".to_string(),
            args: vec![
                Operand::Const(ConstValue::Int(90)),
                Operand::Const(ConstValue::Float(3.14)),
            ],
        }];

        let result = analyzer.analyze_chain(Operand::Temp(0), &calls);

        assert_eq!(result.methods.len(), 1);
        assert_eq!(result.methods[0].name, "rotate");
        assert_eq!(result.methods[0].args.len(), 2);
    }

    #[test]
    fn test_empty_chain() {
        // 测试空链
        let mut analyzer = ChainCallAnalyzer::new();

        let result = analyzer.analyze_chain(Operand::Temp(0), &[]);

        assert!(result.methods.is_empty());
        assert_eq!(result.initial_receiver, Operand::Temp(0));
    }

    #[test]
    fn test_non_chain_call() {
        // 测试非链式调用（接收者不匹配）
        let mut analyzer = ChainCallAnalyzer::new();

        let calls = vec![Instruction::CallVirt {
            dst: Some(Operand::Temp(1)),
            obj: Operand::Temp(5), // 不同的索引，不匹配
            method_name: "method".to_string(),
            args: vec![],
        }];

        let result = analyzer.analyze_chain(Operand::Temp(0), &calls);

        assert!(result.methods.is_empty());
    }

    #[test]
    fn test_call_instruction_chain() {
        // 测试普通函数调用链（第一个参数是接收者）
        let mut analyzer = ChainCallAnalyzer::new();

        let calls = vec![
            Instruction::Call {
                dst: Some(Operand::Temp(1)),
                func: Operand::Global(0),
                args: vec![Operand::Temp(0), Operand::Const(ConstValue::Int(1))],
            },
            Instruction::Call {
                dst: Some(Operand::Temp(2)),
                func: Operand::Global(0),
                args: vec![Operand::Temp(1), Operand::Const(ConstValue::Int(2))],
            },
        ];

        let result = analyzer.analyze_chain(Operand::Temp(0), &calls);

        assert_eq!(result.methods.len(), 2);
        assert_eq!(result.methods[0].name, "call");
        assert_eq!(result.methods[1].name, "call");
    }

    #[test]
    fn test_mixed_call_chain() {
        // 测试混合调用链（Call 和 CallVirt）
        let mut analyzer = ChainCallAnalyzer::new();

        let calls = vec![
            Instruction::CallVirt {
                dst: Some(Operand::Temp(1)),
                obj: Operand::Temp(0),
                method_name: "method1".to_string(),
                args: vec![],
            },
            Instruction::Call {
                dst: Some(Operand::Temp(2)),
                func: Operand::Global(0),
                args: vec![Operand::Temp(1)],
            },
            Instruction::CallVirt {
                dst: Some(Operand::Temp(3)),
                obj: Operand::Temp(2),
                method_name: "method2".to_string(),
                args: vec![],
            },
        ];

        let result = analyzer.analyze_chain(Operand::Temp(0), &calls);

        assert_eq!(result.methods.len(), 3);
    }

    #[test]
    fn test_extract_stops_at_non_call() {
        // 测试提取链时遇到非 Call 指令停止
        let analyzer = ChainCallAnalyzer::new();

        let calls = vec![
            Instruction::CallVirt {
                dst: Some(Operand::Temp(1)),
                obj: Operand::Temp(0),
                method_name: "method1".to_string(),
                args: vec![],
            },
            Instruction::Add {
                dst: Operand::Temp(2),
                lhs: Operand::Temp(0),
                rhs: Operand::Const(ConstValue::Int(1)),
            }, // 非调用指令
            Instruction::CallVirt {
                dst: Some(Operand::Temp(3)),
                obj: Operand::Temp(1),
                method_name: "method2".to_string(),
                args: vec![],
            },
        ];

        let func = create_test_func_with_calls(calls);
        let extracted = analyzer.extract_chain_calls(&func, 0);

        // 应该只提取第一个 CallVirt
        assert_eq!(extracted.len(), 1);
    }

    #[test]
    fn test_ownership_closure_with_undetermined() {
        // 测试 Undetermined 模式的保守闭合检查
        let analyzer = ChainCallAnalyzer::new();

        let chain = ChainAnalysisResult {
            methods: vec![MethodInfo {
                name: "unknown".to_string(),
                receiver: Operand::Temp(0),
                args: vec![],
                consume_mode: ConsumeMode::Undetermined,
                result: None,
            }],
            initial_receiver: Operand::Temp(0),
            final_result: None,
            ownership_closed: false,
        };

        // Undetermined 应该保守返回 true
        assert!(analyzer.check_ownership_closure(&chain));
    }

    #[test]
    fn test_chain_final_result_tracking() {
        // 测试链中最终返回值的追踪
        let mut analyzer = ChainCallAnalyzer::new();

        let calls = vec![
            Instruction::CallVirt {
                dst: Some(Operand::Temp(1)),
                obj: Operand::Temp(0),
                method_name: "method1".to_string(),
                args: vec![],
            },
            Instruction::CallVirt {
                dst: Some(Operand::Temp(2)),
                obj: Operand::Temp(1),
                method_name: "method2".to_string(),
                args: vec![],
            },
        ];

        let result = analyzer.analyze_chain(Operand::Temp(0), &calls);

        // 最终返回值应该是最后一个调用的返回值
        assert_eq!(result.final_result, Some(Operand::Temp(2)));
    }
}
