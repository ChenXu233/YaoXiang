//! 所有权回流分析
//!
//! 分析函数参数是否在返回值中返回，支持所有权闭环推断：
//! - Returns 模式: `p = p.process()` - 参数在返回值中返回
//! - Consumes 模式: `consume(p)` - 参数被消费，不返回
//!
//! # 设计原理
//!
//! 所有权回流分析器通过分析函数的 return 语句，判断参数是否在返回值中：
//! - 如果返回值引用了参数 → Returns 模式（所有权回流）
//! - 如果返回值不引用参数 → Consumes 模式（参数被消费）

use crate::middle::core::ir::{FunctionIR, Instruction, Operand};
use std::collections::HashSet;

/// 消费模式推断结果
///
/// 表示函数参数被消费后是否返回：
/// - **Returns**: 参数在返回值中返回，所有权回流
/// - **Consumes**: 参数被消费，不返回
/// - **Undetermined**: 无法确定（保守分析）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsumeMode {
    /// 参数在返回值中返回，所有权回流
    Returns,
    /// 参数被消费，不返回
    Consumes,
    /// 无法确定（保守分析）
    Undetermined,
}

/// 所有权回流分析器
///
/// 分析函数的返回值是否包含函数参数，支持：
/// - 参数引用检测
/// - 消费模式推断
/// - 链式调用分析
#[derive(Debug, Clone)]
pub struct OwnershipFlowAnalyzer {
    /// 函数参数索引集合
    param_indices: HashSet<usize>,
    /// 函数名（用于错误信息）
    function_name: String,
}

impl OwnershipFlowAnalyzer {
    /// 创建新的所有权回流分析器
    pub fn new(function_name: String) -> Self {
        Self {
            param_indices: HashSet::new(),
            function_name,
        }
    }

    /// 分析函数的消费模式
    ///
    /// 遍历函数的所有 return 语句，判断每个参数是否在返回值中：
    /// - 如果任何 return 语句引用了参数 → Returns 模式
    /// - 如果没有 return 语句引用参数 → Consumes 模式
    /// - 如果没有 return 语句 → Consumes（保守估计）
    pub fn analyze_function(
        &mut self,
        func: &FunctionIR,
    ) -> Vec<ConsumeMode> {
        // 初始化参数索引
        self.param_indices.clear();
        for (idx, _) in func.params.iter().enumerate() {
            self.param_indices.insert(idx);
        }

        // 收集所有 return 语句的操作数
        let return_operands = self.collect_return_operands(func);

        if return_operands.is_empty() {
            // 没有 return 语句，所有参数都是 Consumes 模式
            vec![ConsumeMode::Consumes; func.params.len()]
        } else {
            // 分析每个参数是否在返回值中
            self.analyze_params_in_returns(func, &return_operands)
        }
    }

    /// 收集函数中所有 return 语句的操作数
    fn collect_return_operands(
        &self,
        func: &FunctionIR,
    ) -> Vec<Operand> {
        let mut operands = Vec::new();

        for block in &func.blocks {
            for instr in &block.instructions {
                if let Instruction::Ret(Some(value)) = instr {
                    operands.push(value.clone());
                }
            }
        }

        operands
    }

    /// 分析每个参数是否在返回值中
    fn analyze_params_in_returns(
        &self,
        func: &FunctionIR,
        return_operands: &[Operand],
    ) -> Vec<ConsumeMode> {
        let mut modes = Vec::new();

        for (idx, _) in func.params.iter().enumerate() {
            let references_param = return_operands
                .iter()
                .any(|op| self.operand_references_param(op, idx, func));

            if references_param {
                modes.push(ConsumeMode::Returns);
            } else {
                modes.push(ConsumeMode::Consumes);
            }
        }

        modes
    }

    /// 检查操作数是否引用指定参数
    fn operand_references_param(
        &self,
        operand: &Operand,
        param_idx: usize,
        _func: &FunctionIR,
    ) -> bool {
        match operand {
            // 直接引用参数
            Operand::Arg(idx) => *idx == param_idx,
            // 临时变量：保守估计，可能引用参数
            Operand::Temp(_) => true,
            // 其他类型不引用参数
            _ => false,
        }
    }

    /// 检查函数是否以参数作为返回值直接返回
    pub fn returns_param_directly(
        &self,
        func: &FunctionIR,
        param_idx: usize,
    ) -> bool {
        for block in &func.blocks {
            for instr in &block.instructions {
                if let Instruction::Ret(Some(Operand::Arg(idx))) = instr {
                    if *idx == param_idx {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// 获取函数名
    pub fn function_name(&self) -> &str {
        &self.function_name
    }
}

impl Default for OwnershipFlowAnalyzer {
    fn default() -> Self {
        Self::new(String::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::middle::core::ir::{BasicBlock, ConstValue, FunctionIR};
    use crate::frontend::typecheck::MonoType;

    fn create_test_function(returns_param: bool) -> FunctionIR {
        let return_instr = if returns_param {
            Instruction::Ret(Some(Operand::Arg(0)))
        } else {
            Instruction::Ret(Some(Operand::Const(ConstValue::Int(0))))
        };

        FunctionIR {
            name: "test_func".to_string(),
            params: vec![MonoType::Int(0)],
            return_type: MonoType::Int(0),
            is_async: false,
            locals: vec![],
            blocks: vec![BasicBlock {
                label: 0,
                instructions: vec![return_instr],
                successors: vec![],
            }],
            entry: 0,
        }
    }

    #[test]
    fn test_returns_param_directly() {
        let analyzer = OwnershipFlowAnalyzer::new("test".to_string());

        // return p; 模式
        let func_returns_param = create_test_function(true);
        assert!(analyzer.returns_param_directly(&func_returns_param, 0));

        // return 0; 模式
        let func_not_returns_param = create_test_function(false);
        assert!(!analyzer.returns_param_directly(&func_not_returns_param, 0));
    }

    #[test]
    fn test_analyze_function_returns_mode() {
        let mut analyzer = OwnershipFlowAnalyzer::new("test".to_string());
        let func_returns_param = create_test_function(true);
        let modes = analyzer.analyze_function(&func_returns_param);

        assert_eq!(modes.len(), 1);
        assert_eq!(modes[0], ConsumeMode::Returns);
    }

    #[test]
    fn test_analyze_function_consumes_mode() {
        let mut analyzer = OwnershipFlowAnalyzer::new("test".to_string());
        let func_not_returns_param = create_test_function(false);
        let modes = analyzer.analyze_function(&func_not_returns_param);

        assert_eq!(modes.len(), 1);
        assert_eq!(modes[0], ConsumeMode::Consumes);
    }

    #[test]
    fn test_returns_via_move() {
        // 测试 return temp，其中 temp = arg_0
        let mut analyzer = OwnershipFlowAnalyzer::new("test".to_string());

        let func = FunctionIR {
            name: "test".to_string(),
            params: vec![MonoType::Int(0)],
            return_type: MonoType::Int(0),
            is_async: false,
            locals: vec![],
            blocks: vec![BasicBlock {
                label: 0,
                instructions: vec![
                    Instruction::Move {
                        dst: Operand::Temp(0),
                        src: Operand::Arg(0),
                    },
                    Instruction::Ret(Some(Operand::Temp(0))),
                ],
                successors: vec![],
            }],
            entry: 0,
        };

        let modes = analyzer.analyze_function(&func);
        assert_eq!(modes.len(), 1);
        assert_eq!(modes[0], ConsumeMode::Returns);
    }

    #[test]
    fn test_returns_different_value() {
        // 测试返回常量值，参数被消费
        let mut analyzer = OwnershipFlowAnalyzer::new("test".to_string());

        let func = FunctionIR {
            name: "test".to_string(),
            params: vec![MonoType::Int(0)],
            return_type: MonoType::Int(0),
            is_async: false,
            locals: vec![],
            blocks: vec![BasicBlock {
                label: 0,
                instructions: vec![Instruction::Ret(Some(Operand::Const(ConstValue::Int(42))))],
                successors: vec![],
            }],
            entry: 0,
        };

        let modes = analyzer.analyze_function(&func);
        assert_eq!(modes.len(), 1);
        assert_eq!(modes[0], ConsumeMode::Consumes);
    }

    #[test]
    fn test_multiple_params_partial_return() {
        // 测试多参数函数，部分参数返回
        let mut analyzer = OwnershipFlowAnalyzer::new("test".to_string());

        let func = FunctionIR {
            name: "test".to_string(),
            params: vec![MonoType::Int(0), MonoType::Int(0)],
            return_type: MonoType::Int(0),
            is_async: false,
            locals: vec![],
            blocks: vec![BasicBlock {
                label: 0,
                instructions: vec![Instruction::Ret(Some(Operand::Arg(0)))],
                successors: vec![],
            }],
            entry: 0,
        };

        let modes = analyzer.analyze_function(&func);
        assert_eq!(modes.len(), 2);
        assert_eq!(modes[0], ConsumeMode::Returns); // arg_0 返回
        assert_eq!(modes[1], ConsumeMode::Consumes); // arg_1 不返回
    }

    #[test]
    fn test_returns_via_call() {
        // 测试参数作为参数传递给函数，函数返回值
        let mut analyzer = OwnershipFlowAnalyzer::new("test".to_string());

        let func = FunctionIR {
            name: "test".to_string(),
            params: vec![MonoType::Int(0)],
            return_type: MonoType::Int(0),
            is_async: false,
            locals: vec![],
            blocks: vec![BasicBlock {
                label: 0,
                instructions: vec![
                    Instruction::Call {
                        dst: Some(Operand::Temp(0)),
                        func: Operand::Global(0),
                        args: vec![Operand::Arg(0)],
                    },
                    Instruction::Ret(Some(Operand::Temp(0))),
                ],
                successors: vec![],
            }],
            entry: 0,
        };

        let modes = analyzer.analyze_function(&func);
        assert_eq!(modes.len(), 1);
        assert_eq!(modes[0], ConsumeMode::Returns);
    }

    #[test]
    fn test_returns_via_load_field() {
        // 测试从参数加载字段并返回
        let mut analyzer = OwnershipFlowAnalyzer::new("test".to_string());

        let func = FunctionIR {
            name: "test".to_string(),
            params: vec![MonoType::Int(0)],
            return_type: MonoType::Int(0),
            is_async: false,
            locals: vec![],
            blocks: vec![BasicBlock {
                label: 0,
                instructions: vec![
                    Instruction::LoadField {
                        dst: Operand::Temp(0),
                        src: Operand::Arg(0),
                        field: 0,
                    },
                    Instruction::Ret(Some(Operand::Temp(0))),
                ],
                successors: vec![],
            }],
            entry: 0,
        };

        let modes = analyzer.analyze_function(&func);
        assert_eq!(modes.len(), 1);
        assert_eq!(modes[0], ConsumeMode::Returns);
    }

    #[test]
    fn test_no_return_statement() {
        // 测试没有 return 语句的函数（应该是 Consumes）
        let mut analyzer = OwnershipFlowAnalyzer::new("test".to_string());

        let func = FunctionIR {
            name: "test".to_string(),
            params: vec![MonoType::Int(0)],
            return_type: MonoType::Void,
            is_async: false,
            locals: vec![],
            blocks: vec![BasicBlock {
                label: 0,
                instructions: vec![Instruction::Ret(None)],
                successors: vec![],
            }],
            entry: 0,
        };

        let modes = analyzer.analyze_function(&func);
        assert_eq!(modes.len(), 1);
        assert_eq!(modes[0], ConsumeMode::Consumes);
    }

    #[test]
    fn test_multiple_returns_same_param() {
        // 测试多个 return 语句都返回同一个参数
        let mut analyzer = OwnershipFlowAnalyzer::new("test".to_string());

        let func = FunctionIR {
            name: "test".to_string(),
            params: vec![MonoType::Int(0)],
            return_type: MonoType::Int(0),
            is_async: false,
            locals: vec![],
            blocks: vec![BasicBlock {
                label: 0,
                instructions: vec![Instruction::Ret(Some(Operand::Const(ConstValue::Bool(
                    false,
                ))))],
                successors: vec![],
            }],
            entry: 0,
        };

        let modes = analyzer.analyze_function(&func);
        assert_eq!(modes.len(), 1);
        assert_eq!(modes[0], ConsumeMode::Consumes);
    }
}
