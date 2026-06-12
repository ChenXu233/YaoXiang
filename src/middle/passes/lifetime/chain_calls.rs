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
                ..
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
