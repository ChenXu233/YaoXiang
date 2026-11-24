//! 操作数解析器
//!
//! 将 IR 操作数转换为寄存器编号。

use crate::middle::ir::ir::Operand;
use crate::util::diagnostic::{Diagnostic, ErrorCodeDefinition};
use crate::util::span::Span;

/// 操作数解析结果
pub type OperandResult = Result<u8, Diagnostic>;

/// 操作数解析器
///
/// 职责：
/// - 将 Operand 转换为寄存器编号
/// - 验证操作数的有效性
///
/// `Value` 的槽位来自 7b.2 的过渡映射 `FunctionBody::Code.value_slots`
/// （index = ValueId；槽号由 lowering 既有分配器给出）。**本解析器不自行
/// 分配**——线性扫描分配器归 7b.4（09a §P7 7b.2 契约 §二 第 6 条）。
#[derive(Debug)]
pub struct OperandResolver {
    /// 当前函数的值 → 槽映射（按 ValueId 索引）
    value_slots: Vec<usize>,
    /// 当前指令的源码位置（E3017 的发射点必须带 span，见 #324）
    span: Span,
}

impl Default for OperandResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl OperandResolver {
    /// 创建新的操作数解析器
    pub fn new() -> Self {
        OperandResolver {
            value_slots: Vec::new(),
            span: Span::dummy(),
        }
    }

    /// 设置当前指令的源码位置（翻译每条指令前调用；诊断用）。
    pub fn set_span(
        &mut self,
        span: Span,
    ) {
        self.span = span;
    }

    /// 设置当前函数的值 → 槽映射（进入函数体时调用；init 序列清空）。
    pub fn set_value_slots(
        &mut self,
        slots: &[usize],
    ) {
        self.value_slots.clear();
        self.value_slots.extend_from_slice(slots);
    }

    /// 清空值映射（模块 init 序列没有值表）。
    pub fn clear_value_slots(&mut self) {
        self.value_slots.clear();
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
            // 7b.2 起 `Value` 由值表映射到既有临时槽；越界/无映射显式诊断。
            Operand::Value(id) => {
                let Some(&slot) = self.value_slots.get(id.0 as usize) else {
                    return Err(ErrorCodeDefinition::codegen_invalid_operand(&format!(
                        "Operand::Value({}) 在 value_slots 中无映射（值表 {} 项）：本函数的 IR 与值表不一致",
                        id.0,
                        self.value_slots.len()
                    ))
                    .at(self.span)
                    .build());
                };
                if slot > 255 {
                    return Err(
                        ErrorCodeDefinition::register_overflow(&slot.to_string(), "255").build(),
                    );
                }
                Ok(slot as u8)
            }
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
    /// 7b.2 起 `Value` 也是寄存器位（由值表映射到槽），故计入。
    pub fn is_register(
        &self,
        operand: &Operand,
    ) -> bool {
        matches!(
            operand,
            Operand::Local(_) | Operand::Arg(_) | Operand::Value(_)
        )
    }

    /// 检查操作数是否为常量
    pub fn is_constant(
        &self,
        operand: &Operand,
    ) -> bool {
        matches!(operand, Operand::Const(_))
    }
}
