//! 闭包与 lambda 体（lambda body / lambda 表达式）
//!
//! `LambdaBodyIR` 是闭包体生成的中间结果载体。方法体逐字搬自原 `middle/ir/ir_gen.rs`
//! （WBS 7.0.3），语义未改。

use crate::ast;
use crate::frontend::core::sema::MonoType;
use crate::middle::ir::ir::{
    ConstValue, FunctionBody, FunctionIR, Instruction, LocalSlot, Operand,
};
use crate::middle::lower::scope::NestedBodyGuard;
use crate::middle::lower::AstToIrGenerator;
use crate::util::diagnostic::Diagnostic;

/// Lambda 函数体 IR 结果（7b.4 起块由收尾单点切好，不再回传展平指令）
struct LambdaBodyIR {
    blocks: Vec<crate::middle::ir::ir::BasicBlock>,
    locals: Vec<LocalSlot>,
    values: Vec<crate::middle::ir::ir::ValueInfo>,
    value_slots: Vec<usize>,
}

impl AstToIrGenerator {
    /// 生成 Lambda 函数体 IR
    ///
    /// 返回闭包函数体的指令列表和局部变量信息
    fn generate_lambda_body_ir(
        &mut self,
        params: &[ast::Param],
        body: &ast::Block,
        constants: &mut Vec<ConstValue>,
        env_count: usize,
    ) -> Result<LambdaBodyIR, Diagnostic> {
        // WBS 7a.2：lambda 体是独立指令流——父帧（槽位表 / 循环栈 / 当前 span /
        // 三项计数）整体摘进 guard，Drop 时原样返回。
        let mut frame = NestedBodyGuard::enter(self);

        let mut instructions = Vec::new();

        // 进入闭包函数体作用域
        frame.enter_scope();

        // 注册参数槽位。
        // env_count：闭包 env 在帧参数中占前段（运行时 `final_args = env + args`
        // 且 `Frame::with_args` 已把它们铺进 slots[0..]）。因此本层参数直接位于
        // `env_count + i`，**无需** LoadArg——那会与已铺好的槽位错位。
        // 捕获变量同理：`closure_captures` 让 Var 解析走 LoadUpvalue。
        for (i, param) in params.iter().enumerate() {
            frame.register_local(&param.name, env_count + i);
        }
        // 匿名绑定 / 闭包的形参就是声明（绑定注解 / 返回位内层 Fn）本身，无需补位。
        frame.note_param_written_types(params, &[]);

        // 记录局部变量起始位置（env 段之后）。口径与改动前逐字一致：lambda 只
        // 重置 next_temp，不动高水位与循环态地板（既有行为的 C1 约束）。
        let local_var_start = env_count + params.len();
        frame.next_temp = local_var_start;
        // 7b.4：具名基址（分配器的值槽起点）= 本帧的具名高水位。前 5 个函数体
        // 形态早就成对初始化这两项，闭包体此前只装 next_temp——值槽会落进
        // env / 形参槽区间（实测：list_ops 的 reduce 断言在分配器落地后翻红）。
        frame.temp_high_water = local_var_start;

        // 处理函数体语句（RFC-010a 规则①：尾表达式给出 lambda 的值）。
        // 此前无条件逐语句生成、末尾补 `Ret(None)`，导致块体内的
        // `() => 7` 返回值被丢弃（`mk: () -> (() -> Int) = { () => 7 }` 得 void）。
        let last_idx = body.stmts.len().checked_sub(1);
        let mut tail_handled = false;
        for (i, stmt) in body.stmts.iter().enumerate() {
            // 语句边界设定当前 span，供本条语句的所有指令构造就地捕获
            frame.set_cur_span(stmt.span);
            if Some(i) == last_idx {
                let result_reg = frame.next_temp_reg();
                if frame.generate_tail_stmt_ir(
                    stmt,
                    result_reg.clone(),
                    &mut instructions,
                    constants,
                )? {
                    // 尾语句路径（RFC-039 D20 逐案核实项）：本条语句经
                    // `generate_tail_stmt_ir` 提前返回，**必须**在同款语句边界上
                    // 消费它的释放条目——漏这一处，尾语句的 Drop 就静默丢失
                    //（实测红态里正是这一类：尾位置语句的计划键永不命中）。
                    // 取值已完成（值在 result_reg），故 Drop 排在 Ret 之前。
                    frame.emit_release_drops(stmt.plan_id, &mut instructions);
                    instructions.push(Instruction::Ret {
                        value: Some(result_reg.clone()),
                        span: frame.cur_span,
                    });
                    frame.set_cur_span(stmt.span);
                    tail_handled = true;
                    break;
                }
            }
            frame.generate_local_stmt_ir(stmt, &mut instructions, constants)?;
            // NLL Release: 语句边界消费释放计划（RFC-039 D20）
            frame.emit_release_drops(stmt.plan_id, &mut instructions);
            // #393：语句级临时寄存器回收（同 generate_block_ir）
            let named_watermark = frame
                .cur_locals
                .iter()
                .rposition(|slot| slot.name.is_some())
                .map_or(0, |i| i + 1);
            frame.next_temp = named_watermark.max(frame.temp_floor);
            frame.set_cur_span(stmt.span);
        }

        // 尾位置无值（末位为语句 / 空体）时补 Ret(None)
        if !tail_handled {
            let has_ret = instructions
                .iter()
                .any(|inst| matches!(inst, Instruction::Ret { value: _, span: _ }));
            if !has_ret {
                instructions.push(Instruction::Ret {
                    value: None,
                    span: frame.cur_span,
                });
            }
        }

        // 退出作用域
        frame.exit_scope();

        // 收尾单点：值类型 → 块切分 → 槽分配 → 预算闸（E3014，带 span）→ 取表
        let param_types: Vec<MonoType> = params
            .iter()
            .map(|p| {
                p.ty.clone()
                    .map(MonoType::from)
                    .unwrap_or(MonoType::Int(64))
            })
            .collect();
        let body = frame.finalize_code_body(instructions, &param_types)?;

        Ok(LambdaBodyIR {
            blocks: body.blocks,
            locals: body.locals,
            values: body.values,
            value_slots: body.value_slots,
        })
    }

    pub(super) fn generate_lambda_expr_ir(
        &mut self,
        params: &[ast::Param],
        body: &ast::Block,
        result_reg: Operand,
        instructions: &mut Vec<Instruction>,
        constants: &mut Vec<ConstValue>,
    ) -> Result<(), Diagnostic> {
        // Lambda 表达式 IR 生成
        // 例如: (x, y) => x + y

        // 1. 生成唯一的闭包函数名
        let closure_name = format!("closure_{}", self.closure_counter);
        self.closure_counter += 1;

        // 2. 获取闭包的返回类型（简化处理：使用 Void）
        // TODO: 可以通过类型检查结果获取更精确的返回类型
        let return_type = MonoType::Void;

        // 3. 为闭包参数分配寄存器索引
        let _param_regs: Vec<usize> = (0..params.len()).collect();

        // RFC-009a / #254：闭包需捕获自由变量。
        // `pending_env_vars` 只由 spawn/for 路径填充；普通 lambda（如
        // `adder: (n: Int) -> ((x: Int) -> Int) = (x) => x + n` 里的内层）
        // 从未填充，导致 `MakeClosure` 的 env 为空，`n` 在闭包体内取不到
        // （实测 `adder(10)(5)` 得 10 而非 15）。
        // 此处按“body 中引用但非自身参数”收集自由变量，用外层寄存器作为 env。
        let mut env_vars = std::mem::take(&mut self.pending_env_vars);
        let mut env_names = std::mem::take(&mut self.pending_env_names);
        if env_names.is_empty() {
            let bound: std::collections::HashSet<&str> =
                params.iter().map(|p| p.name.as_str()).collect();
            let mut free = Vec::new();
            let mut seen = std::collections::HashSet::new();
            for stmt in &body.stmts {
                super::collect_free_vars_stmt(stmt, &bound, &mut seen, &mut free);
            }
            for name in free {
                if let Some(idx) = self.lookup_local(&name) {
                    env_vars.push(Operand::Local(idx));
                    env_names.push(name);
                }
            }
        }
        // #254：捕获表（变量名 → env 槽位），供闭包体内 Var 解析 → LoadUpvalue
        self.closure_captures = env_names
            .iter()
            .enumerate()
            .map(|(i, n)| (n.clone(), i))
            .collect();

        // 5. 生成闭包函数体 IR
        // 类似于 generate_function_ir 的逻辑，但针对 Lambda
        let closure_body =
            self.generate_lambda_body_ir(params, body, constants, env_names.len())?;
        // #254：闭包体生成完毕，清除捕获表
        self.closure_captures.clear();

        // 6. 创建闭包函数 IR
        // WBS 2.1.2 实测缺陷修复：无注解参数此前被 filter_map 丢弃，
        // params 长度与帧布局错位（参数槽仍在 env_count+i 占位，元数据却空）。
        // 对齐 generate_method_ir:1749 惯例：无注解参数填 Int(64) 占位。
        // 形参类型：外层声明若在返回位给了完整 Fn（括号返回 / 闭包包装形态），
        // 按它渲染（与 typecheck 的声明驱动同一来源）；否则按作者标注；再否则 Int64
        // 占位（WBS 2.1.2：长度必须与帧布局一致）。
        let declared = &self.closure_decl_params;
        let param_types: Vec<MonoType> = params
            .iter()
            .enumerate()
            .map(|(i, p)| {
                if declared.len() == params.len() {
                    return declared[i].1.clone();
                }
                p.ty.clone().map(|t| t.into()).unwrap_or(MonoType::Int(64))
            })
            .collect();

        let closure_func = FunctionIR {
            def: None, // 由 generate_module_ir 尾部 assign_defs 填充
            name: closure_name.clone(),
            params: param_types,
            return_type,
            generic_params: None,
            body: FunctionBody::Code {
                blocks: closure_body.blocks,
                entry: 0,
                locals: closure_body.locals.clone(),
                values: closure_body.values,
                value_slots: closure_body.value_slots,
            },
        };

        // 7. 将闭包函数添加到嵌套函数列表
        self.nested_functions.push(closure_func);

        // 9. 创建 MakeClosure 指令
        // env 包含被捕获的外部变量的 Operand
        instructions.push(Instruction::MakeClosure {
            dst: result_reg.clone(),
            func: closure_name,
            env: env_vars,
            def: None,
            span: self.cur_span,
        });
        Ok(())
    }
}
