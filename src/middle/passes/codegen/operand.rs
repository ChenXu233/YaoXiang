//! 操作数解析器
//!
//! 将 IR 操作数转换为寄存器编号。

use crate::middle::core::ir::Operand;

/// 操作数解析结果
pub type OperandResult = Result<u8, super::CodegenError>;

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
    pub fn to_reg(&self, operand: &Operand) -> OperandResult {
        match operand {
            Operand::Local(id) => {
                if *id > 255 {
                    return Err(super::CodegenError::RegisterOverflow {
                        id: *id,
                        limit: 255,
                    });
                }
                Ok(*id as u8)
            }
            Operand::Temp(id) => {
                if *id > 255 {
                    return Err(super::CodegenError::RegisterOverflow {
                        id: *id,
                        limit: 255,
                    });
                }
                Ok(*id as u8)
            }
            Operand::Arg(id) => {
                if *id > 255 {
                    return Err(super::CodegenError::RegisterOverflow {
                        id: *id,
                        limit: 255,
                    });
                }
                Ok(*id as u8)
            }
            _ => Err(super::CodegenError::InvalidOperand),
        }
    }

    /// 验证操作数是否有效
    pub fn validate(&self, operand: &Operand) -> Result<(), super::CodegenError> {
        self.to_reg(operand)?;
        Ok(())
    }

    /// 检查操作数是否为寄存器类型
    pub fn is_register(&self, operand: &Operand) -> bool {
        matches!(operand, Operand::Local(_) | Operand::Temp(_) | Operand::Arg(_))
    }

    /// 检查操作数是否为常量
    pub fn is_constant(&self, operand: &Operand) -> bool {
        matches!(operand, Operand::Const(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::middle::core::ir::Operand;

    #[test]
    fn test_local_reg() {
        let resolver = OperandResolver::new();
        assert_eq!(resolver.to_reg(&Operand::Local(0)).unwrap(), 0);
        assert_eq!(resolver.to_reg(&Operand::Local(100)).unwrap(), 100);
    }

    #[test]
    fn test_register_overflow() {
        let resolver = OperandResolver::new();
        assert!(resolver.to_reg(&Operand::Local(256)).is_err());
    }
}
