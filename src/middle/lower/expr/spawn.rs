//! spawn 表达式（spawn / spawn-for）
//!
//! 方法体逐字搬自原 `middle/ir/ir_gen.rs`（WBS 7.0.3），语义未改。

use crate::ast;
use crate::ast::Expr;
use crate::middle::ir::ir::{ConstValue, Instruction, Operand};
use crate::middle::lower::synth::{synth_closure, synth_expr_block, SynthEnv};
use crate::middle::lower::AstToIrGenerator;
use crate::util::diagnostic::Diagnostic;
use crate::util::span::Span;

impl AstToIrGenerator {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn generate_spawn_for_expr_ir(
        &mut self,
        var: &str,
        var_mut: &bool,
        iterable: &Expr,
        body: &ast::Block,
        span: &Span,
        result_reg: Operand,
        instructions: &mut Vec<Instruction>,
        constants: &mut Vec<ConstValue>,
    ) -> Result<(), Diagnostic> {
        self.generate_spawn_for_ir(
            var,
            *var_mut,
            iterable,
            body,
            result_reg,
            *span,
            instructions,
            constants,
        )?;
        Ok(())
    }

    pub(super) fn generate_spawn_expr_ir(
        &mut self,
        body: &ast::Block,
        span: &Span,
        _expr: &Expr,
        result_reg: Operand,
        instructions: &mut Vec<Instruction>,
        constants: &mut Vec<ConstValue>,
    ) -> Result<(), Diagnostic> {
        // Spawn block: spawn { ... }
        // RFC-024: DAG 分析识别直接子表达式，为每个生成独立闭包

        // 1. DAG 分析：识别直接子表达式，生成执行计划
        let (trait_table, local_var_types) = if let Some(ref type_result) = self.type_result {
            (&type_result.trait_table, &type_result.local_var_types)
        } else {
            // 无类型信息时使用空表（向后兼容）
            static EMPTY_TRAIT_TABLE: std::sync::LazyLock<
                crate::frontend::core::types::TraitTable,
            > = std::sync::LazyLock::new(crate::frontend::core::types::TraitTable::default);
            static EMPTY_VAR_TYPES: std::sync::LazyLock<
                std::collections::HashMap<String, crate::frontend::core::types::MonoType>,
            > = std::sync::LazyLock::new(std::collections::HashMap::new);
            (&*EMPTY_TRAIT_TABLE, &*EMPTY_VAR_TYPES)
        };
        let analysis = crate::frontend::core::spawn::analysis::analyze_spawn_body(
            body,
            trait_table,
            local_var_types,
        );

        // 2. 进入 spawn 作用域
        self.enter_scope();

        // 3. 为每个直接子表达式生成闭包
        let mut closure_regs = Vec::new();
        for task in &analysis.tasks {
            // 如果是赋值，注册目标变量到 spawn 作用域
            if let Some(target) = &task.target {
                if self.lookup_local(target).is_none() {
                    let reg = self.next_local_slot();
                    self.register_local(target, reg);
                }
            }

            // 将 RHS 包装为无参闭包：() => { rhs }
            // #254：捕获 task 读取的外层变量（RFC-024 §2.3 Move 值捕获）
            // 过滤：仅捕获能在当前作用域链解析的变量（spawn 内声明的由
            // 任务间依赖传递，不在此捕获）；task.reads 由 spawn analysis 提供。
            let mut env_ops = Vec::new();
            let mut env_names = Vec::new();
            for var in &task.reads {
                if let Some(operand) = self.capture_operand(var, instructions) {
                    env_ops.push(operand);
                    env_names.push(var.clone());
                }
            }
            let env = SynthEnv::new(env_ops, env_names);
            let closure_reg = self.next_temp_reg();
            let lambda = synth_closure(
                Vec::new(),
                synth_expr_block(task.expr.clone(), *span),
                *span,
            );
            self.generate_expr_ir_with_env(
                &lambda,
                env,
                closure_reg.clone(),
                instructions,
                constants,
            )?;
            closure_regs.push(closure_reg.clone());
        }

        // 4. 生成 spawn 块剩余语句（非直接子表达式，如 var 声明等）
        // 尾表达式跳过：它在第 6 步作为块值写出（#365）
        for (i, stmt) in body.stmts.iter().enumerate() {
            if !crate::frontend::core::spawn::analysis::is_direct_child(stmt)
                || crate::frontend::core::spawn::analysis::is_spawn_tail_expr(body, i)
            {
                self.generate_local_stmt_ir(stmt, instructions, constants)?;
                // NLL Release: 语句边界消费释放计划（RFC-039 D20）
                self.emit_release_drops(stmt.plan_id, instructions);
            }
        }

        // 5. 生成 Spawn 指令（多闭包 + 执行计划）
        // Spawn 指令会等待所有闭包完成，之后 t1/t2 等变量才可用
        instructions.push(Instruction::Spawn {
            closures: closure_regs,
            plan: analysis.plan,
            result: result_reg.clone(),
            span: self.cur_span,
        });

        // 6. 块的结果值（RFC-010a 规则①：块的值 = 尾表达式）
        // 必须在 Spawn 之后生成，因为 return / 尾表达式可能引用闭包的结果变量。
        //
        // 优先级：return 优先（显式退出函数体语义），否则取尾表达式。
        // #365：此前只扫 return，尾表达式被当并行任务包装进闭包而丢值，
        // 引用块内变量时还会报 E3006（变量由别的任务声明）。
        let mut has_value = false;
        for stmt in &body.stmts {
            if let ast::StmtKind::Expr(ref expr_stmt) = stmt.kind {
                if let ast::Expr::Return(Some(ret_expr), _) = expr_stmt.as_ref() {
                    let ret_reg = self.next_temp_reg();
                    self.generate_expr_ir(ret_expr, ret_reg.clone(), instructions, constants)?;
                    instructions.push(Instruction::Move {
                        dst: result_reg.clone(),
                        src: ret_reg.clone(),
                        span: self.cur_span,
                    });
                    has_value = true;
                    break;
                }
            }
        }
        if !has_value {
            // 尾表达式：值出口（空块 / 末位为赋值语句时为 Void，无需写出）
            if let Some(last_idx) = body.stmts.len().checked_sub(1) {
                if crate::frontend::core::spawn::analysis::is_spawn_tail_expr(body, last_idx) {
                    if let ast::StmtKind::Expr(ref expr) = body.stmts[last_idx].kind {
                        let tail_reg = self.next_temp_reg();
                        self.generate_expr_ir(expr, tail_reg.clone(), instructions, constants)?;
                        instructions.push(Instruction::Move {
                            dst: result_reg.clone(),
                            src: tail_reg.clone(),
                            span: self.cur_span,
                        });
                    }
                }
            }
        }

        // 7. spawn 体语句的释放边界结算（RFC-039 D20 / WBS 4.5.1）
        //
        // 直接子表达式语句在第 3 步被**闭包化**（每条包成无参闭包），
        // 尾表达式 / return 在第 6 步单独取值——这两类语句都不过通用语句循环。
        // 但 ownership 同样走它们（`walk_expr` 的 `Expr::Spawn` 臂遍历
        // `body.stmts`），条目必须有消费点：spawn 体的作用域退点就是此处
        //（`Spawn` 指令已把所有闭包 join 回来）。漏掉这一处，spawn 体语句的
        // 产出键就永远没人消费——义务结算报 W1090（实测红态残留 7 键即此类）。
        for (i, stmt) in body.stmts.iter().enumerate() {
            let generated_in_loop = !crate::frontend::core::spawn::analysis::is_direct_child(stmt)
                || crate::frontend::core::spawn::analysis::is_spawn_tail_expr(body, i);
            if !generated_in_loop {
                self.emit_release_drops(stmt.plan_id, instructions);
            }
        }

        // 8. 退出 spawn 作用域
        self.exit_scope();
        Ok(())
    }
}
