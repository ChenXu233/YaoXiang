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
