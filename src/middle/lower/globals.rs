//! 全局定义族（`AstToIrGenerator` 的顶层绑定生成）
//!
//! 顶层值绑定槽位预分配、初始化指令与依赖拓扑序、匿名绑定、FFI 库 ID、
//! 常量初始化体与类型定义绑定判定。方法体逐字搬自原 `middle/ir/ir_gen.rs`
//! （WBS 7.0.2），语义未改。

use super::scope::NestedBodyGuard;
use super::AstToIrGenerator;
use crate::ast;
use crate::frontend::core::sema::MonoType;
use crate::frontend::module::symbol::SymbolTable;
use crate::middle::ir::ir::{
    BasicBlock, ConstValue, FunctionBody, FunctionIR, Instruction, LocalSlot, Operand,
};
use crate::middle::lower::expr::collect_free_vars_expr;
use crate::util::diagnostic::{Diagnostic, ErrorCodeDefinition};
use crate::util::span::Span;
use super::PendingInit;

impl AstToIrGenerator {
    /// T3：预扫描模块，为所有顶层值绑定分配槽位。
    ///
    /// 必须在生成任何初始化指令**之前**跑：前向引用（`b = a + 1` 中 `a` 在后）
    /// 要求 `a` 的槽位号在生成 `b` 的初始化时就已知。
    /// 按源码顺序分配，故槽位号 = 声明序（与旧行为一致）。
    pub(super) fn register_global_slots(
        &mut self,
        module: &ast::Module,
    ) {
        // 本单元内按名判重（4.10.3）：Script 顶层对 `mut` 的赋值语句与声明
        // 同形，逐语句计数会给同一变量重复配槽（初始化写后配的槽、读取用先
        // 配的槽，运行时类型错配）。布局键在多文件是限定名、本检查是裸名拦
        // 不到（见下行），故判重必须本单元局部做——与 extract_global_defs
        // 同名判据同源。
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        for stmt in &module.items {
            let ast::StmtKind::Assign {
                target,
                type_annotation,
                value,
                ..
            } = &stmt.kind
            else {
                continue;
            };
            let ast::Expr::Var(name, _) = target.as_ref() else {
                continue;
            };
            // 函数绑定进函数表，不占槽位；类型定义绑定无运行时代码
            if ast::Expr::block_binding_is_function(type_annotation.as_ref(), value.as_deref())
                || Self::is_type_def_binding(value.as_deref())
            {
                continue;
            }
            if !seen.insert(name.clone()) {
                continue;
            }
            // 已登记（如跨文件布局已有同名的本文件绑定）则跳过
            if self.global_slot_layout.contains_key(name) {
                continue;
            }
            let ty = type_annotation
                .as_ref()
                .map(|t| MonoType::from(t.clone()))
                .unwrap_or(MonoType::Int(64));
            // 名字已限定（多文件）时布局键用限定名，否则用裸名
            let key = match &self.module_key {
                Some(k) => SymbolTable::qualify(k, name),
                None => name.clone(),
            };
            let slot = self.global_slot_base + self.global_vars.len();
            self.global_slot_layout.insert(key, slot);
            self.global_vars.push((name.clone(), ty, None));
        }
    }

    /// 获取或创建 FFI 库绑定 ID
    pub(super) fn get_or_create_lib_id(
        &mut self,
        mechanism: &str,
        lib_name: &str,
    ) -> usize {
        if let Some(existing) = self
            .ffi_libs
            .iter()
            .find(|l| l.mechanism == mechanism && l.lib_name == lib_name)
        {
            return existing.id;
        }
        let id = self.next_lib_id;
        self.next_lib_id += 1;
        self.ffi_libs.push(crate::middle::ir::ir::FfiLibBinding {
            id,
            mechanism: mechanism.to_string(),
            lib_name: lib_name.to_string(),
        });
        id
    }

    /// 检查函数体（Stmt 列表）是否是对 ExternRef 的求值
    pub(super) fn try_eval_body_as_extern_ref(
        &self,
        body: &[ast::Stmt],
    ) -> Option<ConstValue> {
        if body.len() == 1 {
            match &body[0].kind {
                ast::StmtKind::Expr(expr) | ast::StmtKind::Return(Some(expr)) => {
                    // 直接 eval 表达式（内部会走 eval_const_expr 的 Call 分支检查类型）
                    return self.eval_const_expr(expr);
                }
                _ => {}
            }
        }
        None
    }

    /// 判定 `name = <expr>` 是否为**类型定义绑定**（RFC-010）。
    ///
    /// 形态：`Db = unsafe { Db: Type = {...}; Db }` —— 块内定义类型并把类型名
    /// 作为尾表达式交回。这是编译期构造（类型已在 typecheck 注册并提升），
    /// 不应生成运行时全局变量，否则（T2 前）`unsafe` 无法常量折叠 → E3007。
    /// T2 后 E3007 已不再产生，但本判定仍然必要：类型定义本就不该有运行时代码，
    /// 生成多余初始化会把类型名当成值去求值。
    pub(super) fn is_type_def_binding(value: Option<&ast::Expr>) -> bool {
        ast::Expr::is_type_def_binding(value)
    }

    /// T3：把待定初始化按依赖顺序重排（DFS + 三色标记）。
    ///
    /// 返回按可执行顺序排列的初始化列表（被依赖者在前）。
    /// 检测到环时返回诊断，消息列出环上的名字——而非报一个无名的“循环依赖”。
    pub(super) fn topo_sort_pending_inits(&self) -> Result<Vec<&PendingInit>, Diagnostic> {
        #[derive(Clone, Copy, PartialEq)]
        enum Mark {
            White,
            Gray,
            Black,
        }
        use std::collections::HashMap;
        let mut marks: HashMap<&str, Mark> = self
            .pending_inits
            .iter()
            .map(|p| (p.name.as_str(), Mark::White))
            .collect();
        let by_name: HashMap<&str, &PendingInit> = self
            .pending_inits
            .iter()
            .map(|p| (p.name.as_str(), p))
            .collect();
        let mut order: Vec<&PendingInit> = Vec::with_capacity(self.pending_inits.len());
        // 显式栈的 DFS：避免深依赖链（上千绑定）退化为递归崩溃
        let mut stack: Vec<(&str, usize)> = Vec::new();
        let mut path: Vec<&str> = Vec::new();

        for root in &self.pending_inits {
            if marks[root.name.as_str()] != Mark::White {
                continue;
            }
            stack.push((root.name.as_str(), 0));
            path.push(root.name.as_str());
            marks.insert(root.name.as_str(), Mark::Gray);
            while let Some((name, dep_i)) = stack.pop() {
                let init = by_name[name];
                if dep_i < init.deps.len() {
                    let dep = init.deps[dep_i].as_str();
                    stack.push((name, dep_i + 1));
                    // 依赖不在本文件（跨文件布局、已由别的文件初始化或 std 常量）：跳过
                    if !by_name.contains_key(dep) {
                        continue;
                    }
                    match marks[dep] {
                        Mark::White => {
                            marks.insert(dep, Mark::Gray);
                            path.push(dep);
                            stack.push((dep, 0));
                        }
                        Mark::Gray => {
                            // 回边：环。从 path 里截出环上名字
                            let start = path.iter().position(|p| *p == dep).unwrap_or(0);
                            let cycle = path[start..].join(" → ");
                            return Err(ErrorCodeDefinition::global_init_cycle(&format!(
                                "{cycle} → {dep}"
                            ))
                            .at(init.span)
                            .build());
                        }
                        Mark::Black => {}
                    }
                } else {
                    marks.insert(name, Mark::Black);
                    path.pop();
                    order.push(init);
                }
            }
        }
        Ok(order)
    }

    /// T3：本文件内某个绑定名的槽位号（不含跨文件布局）。
    fn lookup_global_in_own_file(
        &self,
        name: &str,
    ) -> Option<usize> {
        let key = match &self.module_key {
            Some(k) => SymbolTable::qualify(k, name),
            None => name.to_string(),
        };
        self.global_slot_layout.get(&key).copied()
    }

    /// 生成全局变量 IR（T2：槽位 + 初始化）。
    ///
    /// 顶层绑定不再编译成零参访问器函数，而是：
    /// 1. 在 `self.global_vars` 登记槽位（名字 + 类型）；
    /// 2. 生成初始化指令（求值 init 表达式 → `Store(Global(idx))`），
    ///    追加到 `self.module_init`（模块初始化序列）。
    ///
    /// 读取处（`Expr::Var`）走 `Load(Operand::Global(idx))`。
    /// 初始化表达式可以是任意表达式（含块值）——运行时求值，不再要求常量折叠。
    pub(super) fn generate_global_var_ir(
        &mut self,
        name: &str,
        type_annotation: Option<&ast::Type>,
        initializer: Option<&ast::Expr>,
        constants: &mut Vec<ConstValue>,
    ) -> Result<Option<FunctionIR>, Diagnostic> {
        let var_type = type_annotation
            .map(|t| (*t).clone().into())
            .unwrap_or(MonoType::Int(64));

        // T3：槽位已在 register_global_slots 预分配（前向引用需要），此处取回。
        // 未预分配的情形（非顶层语句路径）在此补登记，保持单一来源。
        let slot_idx = match self.lookup_global_in_own_file(name) {
            Some(idx) => idx,
            None => {
                let idx = self.global_slot_base + self.global_vars.len();
                self.global_vars
                    .push((name.to_string(), var_type.clone(), None));
                idx
            }
        };

        // WBS 4.9.8 / issue #465：顶层绑定同样记下**写出**的类型名（注解原文
        // 优先，否则构造器名）——Script 形态的顶层 `t = Temp(1.0)` 是全局槽位，
        // 与块内局部变量走同一条派发归因链。必须在生成初始化指令**之前**登记：
        // 初始化里的 `t.tag()` 就是消费点（同一语句内的自引用不在本判据面）。
        let written = type_annotation
            .and_then(Self::written_type_name_of)
            .map(str::to_string)
            .or_else(|| initializer.and_then(|e| self.constructor_written_type_name(e)));
        if let Some(written) = written {
            self.global_written_types.insert(name.to_string(), written);
        }

        // 生成初始化指令：求值 init 表达式，写入槽位。
        // 无 init 的绑定保留零初始化语义（C 风格默认值）。
        // 旧实现（E3007）要求 init 必须是编译期常量；T2 后改为运行时求值，
        // 「折叠不到」不再是错误，折叠仅作优化（能折就内联为常量）。
        let result_reg = self.next_temp_reg();
        // 先生成到局部缓冲（generate_expr_ir 需 &mut self，不能直接借 self.module_init）
        let mut init_instrs: Vec<Instruction> = Vec::new();
        if let Some(init) = initializer {
            // 常量折叠优化：能折就直接写常量，省掉运行时求值。
            // 折不出就走完整表达式生成（块值、调用、变量引用等）。
            if let Some(const_val) = self.eval_const_expr(init) {
                init_instrs.push(Instruction::Load {
                    dst: Operand::Local(result_reg),
                    src: Operand::Const(const_val),
                    span: Self::get_expr_span(init),
                });
            } else {
                self.generate_expr_ir(init, result_reg, &mut init_instrs, constants)?;
            }
        } else {
            init_instrs.push(Instruction::Load {
                dst: Operand::Local(result_reg),
                src: Operand::Const(ConstValue::Int(0)),
                span: Span::default(),
            });
        }
        init_instrs.push(Instruction::Store {
            dst: Operand::Global(slot_idx),
            src: Operand::Local(result_reg),
            span: initializer.map(Self::get_expr_span).unwrap_or_default(),
        });
        // T3：入待定队列（不直接追加到 module_init）——尾部按依赖拓扑排序后写入。
        let key = match &self.module_key {
            Some(k) => SymbolTable::qualify(k, name),
            None => name.to_string(),
        };
        let deps = initializer
            .map(|init| self.collect_global_deps(init))
            .unwrap_or_default();
        self.pending_inits.push(PendingInit {
            name: key,
            instructions: init_instrs,
            deps,
            span: initializer.map(Self::get_expr_span).unwrap_or_default(),
        });

        // 顶层绑定不再需要独立 FunctionIR
        Ok(None)
    }

    /// T3：收集一个初始化表达式引用的其它顶层绑定名（依赖边）。
    ///
    /// 只统计“解析为本文件或跨文件全局槽位”的引用；局部变量、函数名不算。
    /// 保守起见走完整 AST 遍历——多收一条依赖只会把顺序摆得更保守，
    /// 漏收才会导致初始化顺序错误。
    fn collect_global_deps(
        &self,
        expr: &ast::Expr,
    ) -> Vec<String> {
        let mut found = std::collections::HashSet::new();
        let mut out = Vec::new();
        Self::walk_expr_vars(expr, &mut |name: &str| {
            let key = match &self.module_key {
                Some(k) => SymbolTable::qualify(k, name),
                None => name.to_string(),
            };
            // 自引用不算依赖（由循环检测单独报）
            if self.global_slot_layout.contains_key(&key) && found.insert(key.clone()) {
                out.push(key);
            }
        });
        out
    }

    /// 遍历表达式中的所有 `Var` 名字（含嵌套块、二元、调用等）。
    ///
    /// 复用既有的 `collect_free_vars_expr`（AST 全形态遍历，已覆盖闭包捕获
    /// 场景并经受测试）——`bound` 传空集即「所有 Var 都是自由变量」。
    fn walk_expr_vars(
        expr: &ast::Expr,
        visit: &mut impl FnMut(&str),
    ) {
        let bound: std::collections::HashSet<&str> = std::collections::HashSet::new();
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut names: Vec<String> = Vec::new();
        collect_free_vars_expr(expr, &bound, &mut seen, &mut names);
        for n in &names {
            visit(n.as_str());
        }
    }

    /// RFC-004: 为匿名函数绑定生成独立的 FunctionIR
    ///
    /// 匿名函数绑定在类型定义体内以 lambda 形式定义，需要生成独立的函数 IR。
    /// 函数命名格式为 `TypeName.__anon_method_name`。
    pub(super) fn generate_anon_binding_ir(
        &mut self,
        name: &str,
        params: &[ast::Param],
        return_type: &ast::Type,
        body: &ast::Expr,
        constants: &mut Vec<ConstValue>,
    ) -> Result<Option<FunctionIR>, Diagnostic> {
        // WBS 7a.2：匿名绑定体是独立指令流——父帧（槽位表 / 循环栈 / 当前 span /
        // 三项计数）整体摘进 guard，Drop 时原样返回。
        let mut frame = NestedBodyGuard::enter(self);

        let mut instructions = Vec::new();

        // 进入匿名函数作用域
        frame.enter_scope();

        // 为每个参数生成 LoadArg 指令并注册
        for (i, param) in params.iter().enumerate() {
            instructions.push(Instruction::Load {
                dst: Operand::Local(i),
                src: Operand::Arg(i),
                span: frame.cur_span,
            });

            frame.register_local(&param.name, i);
        }
        // 匿名绑定 / 闭包的形参就是声明（绑定注解 / 返回位内层 Fn）本身，无需补位。
        frame.note_param_written_types(params, &[]);

        // 记录局部变量起始位置
        let local_var_start = params.len();
        frame.next_temp = local_var_start;
        // #393：独立寄存器空间——高水位与循环态地板随之清零
        frame.temp_high_water = local_var_start;
        frame.temp_floor = 0;

        // 生成表达式体的 IR，并返回结果
        let result_reg = frame.next_temp_reg();
        frame.generate_expr_ir(body, result_reg, &mut instructions, constants)?;
        instructions.push(Instruction::Ret {
            value: Some(Operand::Local(result_reg)),
            span: frame.cur_span,
        });

        // 退出作用域
        frame.exit_scope();

        // 槽位表 + 寄存器预算（E3014）单点
        let locals_types: Vec<LocalSlot> = frame.take_body_locals()?;

        // 解析返回类型
        let ret_type: MonoType = return_type.clone().into();

        let func_ir = FunctionIR {
            def: None, // 由 generate_module_ir 尾部 assign_defs 填充
            name: name.to_string(),
            params: params
                .iter()
                .filter_map(|p| p.ty.clone())
                .map(|t| t.into())
                .collect(),
            return_type: ret_type,
            generic_params: None,
            body: FunctionBody::Code {
                blocks: vec![BasicBlock {
                    label: 0,
                    instructions,
                    successors: Vec::new(),
                }],
                entry: 0,
                locals: locals_types,
            },
        };

        Ok(Some(func_ir))
    }
}
