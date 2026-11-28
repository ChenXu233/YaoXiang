//! 字面量表达式（字面量 / f-string）
//!
//! 方法体逐字搬自原 `middle/ir/ir_gen.rs`（WBS 7.0.3），语义未改。

use crate::ast;
use crate::ast::Expr;
use crate::frontend::core::lexer::tokens::Literal;
use crate::frontend::core::sema::MonoType;
use crate::frontend::core::types::identity::Nominal;
use crate::middle::ir::ir::{ConstValue, Instruction, Operand};
use crate::middle::lower::AstToIrGenerator;
use crate::util::diagnostic::Diagnostic;
use crate::util::span::Span;

impl AstToIrGenerator {
    #[allow(clippy::only_used_in_recursion)]
    pub(super) fn generate_lit_expr_ir(
        &mut self,
        literal: &Literal,
        result_reg: Operand,
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
        // 字面量类型是可信来源：直接落值表（7b.2 契约 §二 第 2 条）。
        if let Some(ty) = const_val.mono_type() {
            self.note_value_type(&result_reg, ty);
        }
        // 添加到常量池
        constants.push(const_val.clone());
        instructions.push(Instruction::Load {
            dst: result_reg.clone(),
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
        result_reg: Operand,
        instructions: &mut Vec<Instruction>,
        constants: &mut Vec<ConstValue>,
    ) -> Result<(), Diagnostic> {
        // 1. 尝试常量求值
        if let Some(const_val) = self.eval_const_expr(expr) {
            constants.push(const_val.clone());
            instructions.push(Instruction::Load {
                dst: result_reg.clone(),
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
                    self.generate_expr_ir(interp_expr, arg_reg.clone(), instructions, constants)?;
                    arg_regs.push(arg_reg.clone());
                }
            }
        }

        // Load format string constant
        let fmt_reg = self.next_temp_reg();
        let fmt_const = ConstValue::String(format_str);
        constants.push(fmt_const.clone());
        instructions.push(Instruction::Load {
            dst: fmt_reg.clone(),
            src: Operand::Const(fmt_const),
            span: self.cur_span,
        });

        // Build args: [format_str, arg0, arg1, ...]
        let mut call_args = vec![fmt_reg.clone()];
        call_args.extend(arg_regs);

        // Generate Call to std.string.format
        instructions.push(Instruction::Call {
            dst: Some(result_reg.clone()),
            func: Operand::Const(ConstValue::String("std.string.format".to_string())),
            args: call_args,
            span: *span,
            def: None,
        });
        // f-string 是合成调用（AST 里没有调用节点），结果类型按声明签名恒为
        // String（src/std/string.rs 的 `format -> String`）。
        self.note_value_type(
            &result_reg,
            MonoType::Generic {
                name: Nominal::flat("String").expect("String 是裸类型名"),
                args: Vec::new(),
            },
        );
        Ok(())
    }
}
