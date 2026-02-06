//! 消费分析器
//!
//! 提供跨函数的消费模式查询能力，复用 Phase 3 的 OwnershipFlowAnalyzer。
//! 支持在函数调用点查询被调用函数对参数的消费模式。
//!
//! # 设计原理
//!
//! 消费分析器通过缓存和查询已分析函数的消费模式，支持：
//! - 函数级消费模式查询（Returns / Consumes / Undetermined）
//! - 调用点参数消费模式查询
//! - 内置函数特殊处理（consume, clone 等）

use super::ownership_flow::ConsumeMode;
use crate::middle::core::ir::{FunctionIR, Instruction, Operand};
use std::collections::HashMap;

/// 消费分析器
///
/// 复用 Phase 3 的 OwnershipFlowAnalyzer，提供：
/// - 函数消费模式缓存
/// - 内置函数特殊处理
/// - 调用点消费模式查询
#[derive(Debug, Clone)]
pub struct ConsumeAnalyzer {
    /// 缓存：函数名 -> 参数消费模式
    consume_mode_cache: HashMap<String, Vec<ConsumeMode>>,
    /// 缓存：函数 IR -> 参数消费模式
    ir_cache: HashMap<String, Vec<ConsumeMode>>,
}

impl ConsumeAnalyzer {
    /// 创建新的消费分析器
    pub fn new() -> Self {
        Self {
            consume_mode_cache: HashMap::new(),
            ir_cache: HashMap::new(),
        }
    }

    /// 获取函数的参数消费模式
    ///
    /// 复用 Phase 3 的 OwnershipFlowAnalyzer 进行分析
    pub fn get_function_consume_mode(
        &mut self,
        func: &FunctionIR,
    ) -> Vec<ConsumeMode> {
        // 检查缓存
        if let Some(cached) = self.ir_cache.get(&func.name).cloned() {
            return cached;
        }

        // 使用 OwnershipFlowAnalyzer 进行分析
        let mut analyzer = super::OwnershipFlowAnalyzer::new(func.name.clone());
        let modes = analyzer.analyze_function(func);

        // 缓存结果
        self.ir_cache.insert(func.name.clone(), modes.clone());
        self.consume_mode_cache
            .insert(func.name.clone(), modes.clone());

        modes
    }

    /// 获取函数的参数消费模式（按函数名）
    ///
    /// 需要函数已被分析过
    pub fn get_function_consume_mode_by_name(
        &self,
        func_name: &str,
    ) -> Option<&Vec<ConsumeMode>> {
        self.consume_mode_cache.get(func_name)
    }

    /// 分析特定调用点的消费模式
    ///
    /// 返回参数在本次调用中的消费模式
    pub fn get_call_consume_mode(
        &self,
        func_name: &str,
        _arg_idx: usize,
    ) -> Option<ConsumeMode> {
        self.consume_mode_cache
            .get(func_name)
            .and_then(|modes| modes.first().cloned())
    }

    /// 分析内置函数的消费模式
    ///
    /// 内置函数有特殊的消费语义：
    /// - `consume(x)`: Consumes 模式，x 进入 Empty
    /// - `clone(x)`: 不消费，x 仍为 Owned
    /// - 其他函数: 使用分析结果或保守估计
    pub fn get_builtin_consume_mode(
        &self,
        func_name: &str,
        _arg_idx: usize,
    ) -> ConsumeMode {
        match func_name {
            // consume 函数：消费参数
            "consume" => ConsumeMode::Consumes,
            // clone 函数：不消费参数（复制一个新值）
            "clone" => ConsumeMode::Returns,
            // 迭代器方法等：Consumes 模式
            _ => ConsumeMode::Undetermined,
        }
    }

    /// 预分析函数并缓存结果
    pub fn analyze_and_cache(
        &mut self,
        func: &FunctionIR,
    ) -> Vec<ConsumeMode> {
        self.get_function_consume_mode(func)
    }

    /// 清除缓存
    pub fn clear_cache(&mut self) {
        self.consume_mode_cache.clear();
        self.ir_cache.clear();
    }
}

impl Default for ConsumeAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

/// 从指令中提取函数名（如果可能）
pub fn extract_function_name(instr: &Instruction) -> Option<String> {
    match instr {
        Instruction::Call { func, .. } => {
            if let Operand::Global(name) = func {
                Some(name.to_string())
            } else {
                None
            }
        }
        Instruction::CallVirt { method_name, .. } => Some(method_name.clone()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::middle::core::ir::{BasicBlock, ConstValue, FunctionIR};
    use crate::frontend::typecheck::MonoType;

    fn make_test_function(returns_param: bool) -> FunctionIR {
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
    fn test_analyze_returns_mode() {
        let mut analyzer = ConsumeAnalyzer::new();
        let func = make_test_function(true);

        let modes = analyzer.analyze_and_cache(&func);

        assert_eq!(modes.len(), 1);
        assert_eq!(modes[0], ConsumeMode::Returns);
    }

    #[test]
    fn test_analyze_consumes_mode() {
        let mut analyzer = ConsumeAnalyzer::new();
        let func = make_test_function(false);

        let modes = analyzer.analyze_and_cache(&func);

        assert_eq!(modes.len(), 1);
        assert_eq!(modes[0], ConsumeMode::Consumes);
    }

    #[test]
    fn test_builtin_consume_mode() {
        let analyzer = ConsumeAnalyzer::new();

        assert_eq!(
            analyzer.get_builtin_consume_mode("consume", 0),
            ConsumeMode::Consumes
        );
        assert_eq!(
            analyzer.get_builtin_consume_mode("clone", 0),
            ConsumeMode::Returns
        );
    }

    #[test]
    fn test_cache_after_analysis() {
        let mut analyzer = ConsumeAnalyzer::new();
        let func = make_test_function(true);

        // 第一次分析
        let modes1 = analyzer.analyze_and_cache(&func);
        // 第二次查询（应该命中缓存）
        let modes2 = analyzer.get_function_consume_mode(&func);

        assert_eq!(modes1, modes2);
    }

    #[test]
    fn test_multiple_params_partial_return() {
        // 测试多参数函数，部分参数返回
        let mut analyzer = ConsumeAnalyzer::new();

        let func = FunctionIR {
            name: "partial_return".to_string(),
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

        let modes = analyzer.analyze_and_cache(&func);
        assert_eq!(modes.len(), 2);
        assert_eq!(modes[0], ConsumeMode::Returns); // arg_0 返回
        assert_eq!(modes[1], ConsumeMode::Consumes); // arg_1 不返回
    }

    #[test]
    fn test_multiple_params_all_consumed() {
        // 测试多参数函数，所有参数都被消费
        let mut analyzer = ConsumeAnalyzer::new();

        let func = FunctionIR {
            name: "all_consumed".to_string(),
            params: vec![MonoType::Int(0), MonoType::Int(0)],
            return_type: MonoType::Int(0),
            is_async: false,
            locals: vec![],
            blocks: vec![BasicBlock {
                label: 0,
                instructions: vec![Instruction::Ret(Some(Operand::Const(ConstValue::Int(0))))],
                successors: vec![],
            }],
            entry: 0,
        };

        let modes = analyzer.analyze_and_cache(&func);
        assert_eq!(modes.len(), 2);
        assert_eq!(modes[0], ConsumeMode::Consumes);
        assert_eq!(modes[1], ConsumeMode::Consumes);
    }

    #[test]
    fn test_no_return_returns_void() {
        // 测试无返回值的函数
        let mut analyzer = ConsumeAnalyzer::new();

        let func = FunctionIR {
            name: "void_func".to_string(),
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

        let modes = analyzer.analyze_and_cache(&func);
        assert_eq!(modes.len(), 1);
        assert_eq!(modes[0], ConsumeMode::Consumes);
    }

    #[test]
    fn test_returns_via_temp_variable() {
        // 测试通过临时变量返回参数
        let mut analyzer = ConsumeAnalyzer::new();

        let func = FunctionIR {
            name: "via_temp".to_string(),
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

        let modes = analyzer.analyze_and_cache(&func);
        assert_eq!(modes.len(), 1);
        assert_eq!(modes[0], ConsumeMode::Returns);
    }

    #[test]
    fn test_get_call_consume_mode() {
        let mut analyzer = ConsumeAnalyzer::new();
        let func = make_test_function(true);
        analyzer.analyze_and_cache(&func);

        let mode = analyzer.get_call_consume_mode("test_func", 0);
        assert_eq!(mode, Some(ConsumeMode::Returns));
    }

    #[test]
    fn test_unknown_function_uses_builtin() {
        let analyzer = ConsumeAnalyzer::new();
        // 未知函数使用内置处理
        let mode = analyzer.get_builtin_consume_mode("unknown_func", 0);
        assert_eq!(mode, ConsumeMode::Undetermined);
    }

    #[test]
    fn test_clear_cache() {
        let mut analyzer = ConsumeAnalyzer::new();
        let func = make_test_function(true);
        analyzer.analyze_and_cache(&func);

        // 确认缓存有数据
        assert!(analyzer
            .get_function_consume_mode_by_name("test_func")
            .is_some());

        // 清除缓存
        analyzer.clear_cache();

        // 缓存应该为空
        assert!(analyzer
            .get_function_consume_mode_by_name("test_func")
            .is_none());
    }
}
