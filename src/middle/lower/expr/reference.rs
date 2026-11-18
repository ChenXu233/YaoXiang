//! 引用与 unsafe 表达式（Ref / Borrow / Unsafe）
//!
//! 方法体逐字搬自原 `middle/ir/ir_gen.rs`（WBS 7.0.3），语义未改。

use crate::ast;
use crate::ast::Expr;
use crate::middle::ir::ir::{ConstValue, Instruction, Operand};
use crate::middle::lower::AstToIrGenerator;
use crate::util::diagnostic::Diagnostic;

impl AstToIrGenerator {
    pub(super) fn generate_unsafe_expr_ir(
        &mut self,
        body: &ast::Block,
        result_reg: usize,
        instructions: &mut Vec<Instruction>,
        constants: &mut Vec<ConstValue>,
    ) -> Result<(), Diagnostic> {
        // unsafe 块：生成 UnsafeBlockStart/End 标记
        instructions.push(Instruction::UnsafeBlockStart {
            span: self.cur_span,
        });

        // RFC-010a 规则①：`unsafe {}` 是**有值块**，值出口为尾表达式。
        // 此前 result_reg 传 None 并硬写 Void，导致 `v = unsafe { 42 }`
        // 得 void（#347）。现按普通块处理，尾表达式写入 result_reg。
        self.generate_block_ir(body, Some(result_reg), instructions, constants)?;

        instructions.push(Instruction::UnsafeBlockEnd {
            span: self.cur_span,
        });
        Ok(())
    }

    pub(super) fn generate_ref_expr_ir(
        &mut self,
        expr: &Expr,
        result_reg: usize,
        instructions: &mut Vec<Instruction>,
        constants: &mut Vec<ConstValue>,
    ) -> Result<(), Diagnostic> {
        // 生成内部表达式的 IR
        let src_reg = self.next_temp_reg();
        self.generate_expr_ir(expr, src_reg, instructions, constants)?;

        // 逃逸分析：跨 spawn 使用 → Arc，否则 → Rc
        let var_name = match expr {
            ast::Expr::Var(name, _) => Some(name.clone()),
            _ => None,
        };
        let use_arc = var_name.as_ref().is_some_and(|n| {
            self.type_result
                .as_ref()
                .is_some_and(|tr| tr.escaped_refs.contains(n))
        });

        if use_arc {
            instructions.push(Instruction::ArcNew {
                dst: Operand::Local(result_reg),
                src: Operand::Local(src_reg),
                span: self.cur_span,
            });
        } else {
            instructions.push(Instruction::RcNew {
                dst: Operand::Local(result_reg),
                src: Operand::Local(src_reg),
                span: self.cur_span,
            });
        }
        Ok(())
    }

    pub(super) fn generate_borrow_expr_ir(
        &mut self,
        expr: &Expr,
        result_reg: usize,
        instructions: &mut Vec<Instruction>,
        constants: &mut Vec<ConstValue>,
    ) -> Result<(), Diagnostic> {
        // 1. 生成内部表达式的 IR
        let inner_reg = self.next_temp_reg();
        self.generate_expr_ir(expr, inner_reg, instructions, constants)?;

        // 借用令牌（& / &mut）是编译期品牌，运行时零大小：
        // Struct 值是堆句柄，传参/赋值复制的是句柄，共享同一对象——
        // 因此字段写（StoreField）经令牌自然写回底层，无需额外指令（#266）。
        // 所有权与借用合法性已在 typecheck 层验证（RFC-009a）。
        instructions.push(Instruction::Move {
            dst: Operand::Local(result_reg),
            src: Operand::Local(inner_reg),
            span: self.cur_span,
        });
        Ok(())
    }
}
