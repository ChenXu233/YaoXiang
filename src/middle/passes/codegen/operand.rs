//! 操作数解析器
//!
//! 将 IR 操作数转换为寄存器编号。

use crate::middle::ir::ir::Operand;
use crate::util::diagnostic::{Diagnostic, ErrorCodeDefinition};

/// 操作数解析结果
pub type OperandResult = Result<u8, Diagnostic>;

/// 操作数解析器
///
/// 职责：
/// - 将 Operand 转换为寄存器编号
/// - 验证操作数的有效性
#[derive(Debug, Default)]
pub struct OperandResolver;

impl OperandResolver {
    /// 创建新的操作数解析器
    pub fn new() -> Self {
        OperandResolver
    }

    /// 将操作数转换为寄存器编号
    pub fn to_reg(
        &self,
        operand: &Operand,
    ) -> OperandResult {
        match operand {
            Operand::Local(id) => {
                if *id > 255 {
                    return Err(
                        ErrorCodeDefinition::register_overflow(&id.to_string(), "255").build(),
                    );
                }
                Ok(*id as u8)
            }
            // 占位（09a §P7 7b.1a 契约 §三 占位 2，7b.2 换槽位解析）：
            // `ValueId` 是 SSA 值编号，7b.2 起由 `next_temp_reg` 产出。本批生产
            // 构造点 0；若真出现，显式诊断优于静默映射到错误槽位。
            Operand::Value(id) => Err(ErrorCodeDefinition::codegen_invalid_operand(&format!(
                "Operand::Value({}) 尚不可解析为槽位（SSA 值槽位解析归 7b.2）",
                id.0
            ))
            .build()),
            Operand::Arg(id) => {
                if *id > 255 {
                    return Err(
                        ErrorCodeDefinition::register_overflow(&id.to_string(), "255").build(),
                    );
                }
                Ok(*id as u8)
            }
            _ => Err(ErrorCodeDefinition::codegen_invalid_operand("invalid operand type").build()),
        }
    }

    /// 验证操作数是否有效
    pub fn validate(
        &self,
        operand: &Operand,
    ) -> Result<(), Diagnostic> {
        self.to_reg(operand)?;
        Ok(())
    }

    /// 检查操作数是否为寄存器类型
    ///
    /// `Operand::Value` 要等 7b.2 才能解析为槽位（09a §P7 7b.1a 契约 §三
    /// 占位 2），本批不把它读成「已是寄存器」。
    pub fn is_register(
        &self,
        operand: &Operand,
    ) -> bool {
        matches!(operand, Operand::Local(_) | Operand::Arg(_))
    }

    /// 检查操作数是否为常量
    pub fn is_constant(
        &self,
        operand: &Operand,
    ) -> bool {
        matches!(operand, Operand::Const(_))
    }
}
