//! 字面量表达式（字面量 / f-string）
//!
//! 方法体逐字搬自原 `middle/ir/ir_gen.rs`（WBS 7.0.3），语义未改。

use crate::ast::Expr;
use crate::ast;
use crate::frontend::core::lexer::tokens::Literal;
use crate::middle::ir::ir::{ConstValue, Instruction, Operand};
use crate::middle::lower::AstToIrGenerator;
use crate::util::diagnostic::Diagnostic;
use crate::util::span::Span;

impl AstToIrGenerator {
    #[allow(clippy::only_used_in_recursion)]
    pub(super) fn generate_lit_expr_ir(
        &mut self,
        literal: &Literal,
        result_reg: usize,
        instructions: &mut Vec<Instruction>,
        constants: &mut Vec<ConstValue>,
    ) -> Result<(), Diagnostic> {
        // 常量加载
        let const_val = match literal {
            Literal::Int(n) => ConstValue::Int(*n),
            Literal::Float(f) => ConstValue::Float(*f),
            Literal::Bool(b) => ConstValue::Bool(*b),
            Literal::String(s) => ConstValue::String(s.clone()),
            Literal::Char(c) => ConstValue::Char(*c),
            Literal::Void => ConstValue::Void,
        };
        // 添加到常量池
        constants.push(const_val.clone());
        instructions.push(Instruction::Load {
            dst: Operand::Local(result_reg),
            src: Operand::Const(const_val),
            span: self.cur_span,
        });
        Ok(())
    }

    pub(super) fn generate_f_string_expr_ir(
        &mut self,
        segments: &Vec<ast::FStringSegment>,
        span: &Span,
        expr: &Expr,
        result_reg: usize,
        instructions: &mut Vec<Instruction>,
        constants: &mut Vec<ConstValue>,
    ) -> Result<(), Diagnostic> {
        // 1. 尝试常量求值
        if let Some(const_val) = self.eval_const_expr(expr) {
            constants.push(const_val.clone());
            instructions.push(Instruction::Load {
                dst: Operand::Local(result_reg),
                src: Operand::Const(const_val),
                span: self.cur_span,
            });
            return Ok(());
        }

        // 2. 转换为 format() 调用
        // 构建 format_str: "Hello {} is {} years old"
        // 构建 args: [name, age]
        let mut format_str = String::new();
        let mut arg_regs = Vec::new();
        let mut arg_index = 0usize;

        for segment in segments {
            match segment {
                ast::FStringSegment::Text(text) => {
                    // Text 段是解码后的字面内容（#402），可能含 `{`/`}`；
                    // std.string.format 以 `{{`/`}}` 为字面花括号转义，
                    // 这里翻倍以免被当成占位符
                    for c in text.chars() {
                        if c == '{' || c == '}' {
                            format_str.push(c);
                        }
                        format_str.push(c);
                    }
                }
                ast::FStringSegment::Interpolation {
                    expr: interp_expr,
                    format_spec,
                } => {
                    // Build format placeholder: {0}, {1}, or {0:.2f}
                    if let Some(spec) = format_spec {
                        format_str.push_str(&format!("{{{0}:{1}}}", arg_index, spec));
                    } else {
                        format_str.push_str(&format!("{{{}}}", arg_index));
                    }
                    arg_index += 1;

                    // Generate IR for the interpolation expression
                    let arg_reg = self.next_temp_reg();
                    self.generate_expr_ir(interp_expr, arg_reg, instructions, constants)?;
                    arg_regs.push(Operand::Local(arg_reg));
                }
            }
        }

        // Load format string constant
        let fmt_reg = self.next_temp_reg();
        let fmt_const = ConstValue::String(format_str);
        constants.push(fmt_const.clone());
        instructions.push(Instruction::Load {
            dst: Operand::Local(fmt_reg),
            src: Operand::Const(fmt_const),
            span: self.cur_span,
        });

        // Build args: [format_str, arg0, arg1, ...]
        let mut call_args = vec![Operand::Local(fmt_reg)];
        call_args.extend(arg_regs);

        // Generate Call to std.string.format
        instructions.push(Instruction::Call {
            dst: Some(Operand::Local(result_reg)),
            func: Operand::Const(ConstValue::String("std.string.format".to_string())),
            args: call_args,
            span: *span,
            def: None,
        });
        Ok(())
    }
}
