//! 所有权分析辅助类型
//!
//! 中间层状态追踪的公共接口和工具函数。

use crate::middle::core::ir::{FunctionIR, Operand};
use crate::util::diagnostic::Diagnostic;

/// 所有诊断检查器的最小公共接口
pub trait Checker {
    /// 检查函数的语义
    fn check_function(
        &mut self,
        func: &FunctionIR,
    ) -> &[Diagnostic];

    /// 获取收集的错误
    fn errors(&self) -> &[Diagnostic];

    /// 清除状态
    fn clear(&mut self);
}

/// 将 Operand 转换为字符串标识
pub fn operand_to_string(operand: &Operand) -> String {
    match operand {
        Operand::Local(idx) => format!("local_{}", idx),
        Operand::Arg(idx) => format!("arg_{}", idx),
        Operand::Temp(idx) => format!("temp_{}", idx),
        Operand::Global(idx) => format!("global_{}", idx),
        Operand::Const(c) => format!("const_{:?}", c),
        Operand::Label(idx) => format!("label_{}", idx),
        Operand::Register(idx) => format!("reg_{}", idx),
    }
}

/// 获取操作数的用户可见名称
///
/// 优先使用源码变量名（local_names），回退到内部名（operand_to_string）。
/// 确保错误信息显示 `'p' has been moved` 而不是 `'local_0' has been moved`。
pub fn operand_display_name(
    operand: &Operand,
    local_names: Option<&Vec<String>>,
) -> String {
    if let Operand::Local(idx) = operand {
        if let Some(names) = local_names {
            if *idx < names.len() && !names[*idx].is_empty() {
                return names[*idx].clone();
            }
        }
    }
    operand_to_string(operand)
}
