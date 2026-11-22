//! 语句族（`AstToIrGenerator` 的语句与函数体生成）
//!
//! 覆盖语句入口、函数与方法体装配、语句级 span/释放计划边界、块与 if 语句。
//! 方法体逐字搬自原 `middle/ir/ir_gen.rs`（WBS 7.0.2），语义未改。

use super::scope::NestedBodyGuard;
use super::AstToIrGenerator;
use crate::ast;
use crate::frontend::core::sema::MonoType;
use crate::frontend::module::symbol::SymbolTable;
use crate::middle::ir::ir::{
    ConstValue, FunctionBody, FunctionIR, Instruction, Operand, TypeDeclBody, TypeDeclField,
};
use crate::tlog;
use crate::util::diagnostic::{Diagnostic, ErrorCodeDefinition};
use crate::util::i18n::MSG;
use crate::util::plan_id::PlanId;
use crate::util::span::Span;

impl AstToIrGenerator {
    /// 生成语句的 IR
    pub(super) fn generate_stmt_ir(
        &mut self,
        stmt: &ast::Stmt,
        constants: &mut Vec<ConstValue>,
    ) -> Result<Option<FunctionIR>, Diagnostic> {
        match &stmt.kind {
            ast::StmtKind::TypeDefinition {
                name,
                type_annotation,
                definition,
            } => {
                self.declared_type_names.insert(name.clone());
                use crate::ast::extract_generic_param_names;
                use crate::frontend::core::types::mono::UniverseLevel;

                // 泛型形参的唯一来源：声明注解的 Type::Fn 链（6.5.2）
                let signature = type_annotation
                    .as_ref()
                    .map(crate::ast::Type::fn_params_flat)
                    .unwrap_or_default();

                // 提取泛型参数名。探针传 L3 语义实现：约束名 / 内置名定种必须以
                // 类型环境的权威表为准，否则 `(T: Add)` 不登记为泛型参数、
                // 单态化请求的类型实参与 IR 的 generic_params 对不上（E3018）。
                // 扫描面 = 整棵声明注解（形参标注 ∪ 返回位）。构造器面里 const 名的
                // 另一处引用面是**定义体**——那是单态化请求侧只携带类型实参的既有面
                // （请求 arity 与本表逐字对齐），不属本函数的扫描面。
                let type_positions: Vec<&ast::Type> = type_annotation.iter().collect();
                let generic_params =
                    extract_generic_param_names(&signature, &type_positions, &self.type_level);
                let generic_param_names = if generic_params.is_empty() {
                    None
                } else {
                    Some(generic_params.iter().map(|p| p.name.clone()).collect())
                };

                // 签名：(T: Type) -> Type → params = [MetaType], return_type = MetaType
                let params: Vec<MonoType> = signature
                    .iter()
                    .map(|p| {
                        p.ty.as_ref()
                            .map(|t| t.clone().into())
                            .unwrap_or(MonoType::MetaType {
                                universe_level: UniverseLevel::type1(),
                                type_params: Vec::new(),
                            })
                    })
                    .collect();
                let return_type = MonoType::MetaType {
                    universe_level: UniverseLevel::type1(),
                    type_params: Vec::new(),
                };

                // 记录 struct_definitions（字段索引解析仍需要）
                // 同时处理类型绑定和匿名函数 IR 生成
                match definition {
                    ast::Type::NamedStruct {
                        name: struct_name,
                        fields,
                        ..
                    } => {
                        self.struct_definitions
                            .insert(struct_name.clone(), fields.clone());
                    }
                    ast::Type::Struct { body } => {
                        let fields: Vec<ast::StructField> = body
                            .iter()
                            .filter_map(|it| {
                                if let ast::TypeBodyItem::Field(f) = it {
                                    Some(f.clone())
                                } else {
                                    None
                                }
                            })
                            .collect();
                        let bindings: Vec<ast::TypeBodyBinding> = body
                            .iter()
                            .filter_map(|it| {
                                if let ast::TypeBodyItem::Binding(b) = it {
                                    Some(b.clone())
                                } else {
                                    None
                                }
                            })
                            .collect();
                        let fields = self.classified_fields(name, &fields);
                        self.struct_definitions.insert(name.clone(), fields);
                        // 记录绑定信息（用于方法调用时的参数重排，RFC-004）
                        self.register_type_bindings(name, &bindings);
                        // RFC-004: 为匿名函数绑定生成独立的 FunctionIR
                        for binding in &bindings {
                            if let ast::BindingKind::Anonymous {
                                params: anon_params,
                                return_type: anon_ret,
                                positions: _,
                                body,
                            } = &binding.kind
                            {
                                let anon_func_name = format!("{}.__anon_{}", name, binding.name);
                                match self.generate_anon_binding_ir(
                                    &anon_func_name,
                                    anon_params,
                                    anon_ret,
                                    body,
                                    constants,
                                ) {
                                    Ok(Some(func_ir)) => {
                                        self.anon_function_irs.push(func_ir);
                                    }
                                    Ok(None) => {}
                                    Err(e) => return Err(e),
                                }
                            }
                        }
                    }
                    _ => {}
                }

                Ok(Some(FunctionIR {
                    def: None, // 由 generate_module_ir 尾部 assign_defs 填充
                    name: name.clone(),
                    params,
                    return_type,
                    generic_params: generic_param_names,
                    // 类型声明的 IR 原生收窄载体（WBS 6.6.12）：只带字段表与类型表项。
                    // 形参名 / 默认值表达式 / const 界 / 匿名绑定 / span 都不进 IR，
                    // 故 6.5 时代「入 IR 前剥形参名」的 `without_fn_param_names` 随之删除。
                    body: FunctionBody::TypeDecl {
                        body: TypeDeclBody {
                            fields: {
                                let declared: Vec<ast::StructField> =
                                    definition.struct_fields().into_iter().cloned().collect();
                                self.classified_fields(name, &declared)
                                    .into_iter()
                                    .map(|f| TypeDeclField {
                                        name: f.name.clone(),
                                        ty: f.ty.clone().into(),
                                        quantity: f.quantity,
                                        is_mut: f.is_mut,
                                        has_default: f.default.is_some(),
                                        is_method: Self::field_is_method(&f.ty),
                                    })
                                    .collect()
                            },
                            self_type: Self::type_decl_self_type(definition),
                        },
                    },
                }))
            }
            ast::StmtKind::Assign {
                target,
                type_annotation,
                value,
                ..
            } => {
                use crate::ast::Expr;
                // 签名形参的唯一来源：注解的 Type::Fn 链（6.5.2）
                let signature = type_annotation
                    .as_ref()
                    .map(crate::ast::Type::fn_params_flat)
                    .unwrap_or_default();
                let (name, type_name) = match target.as_ref() {
                    Expr::Var(n, _) => (n.clone(), None),
                    Expr::FieldAccess { expr, field, .. } => {
                        if let Expr::Var(tn, _) = expr.as_ref() {
                            (field.clone(), Some(tn.clone()))
                        } else {
                            (field.clone(), None)
                        }
                    }
                    _ => return Ok(None),
                };
                // RFC-010a 附录D：`name = { ... }` 无注解时按内容推断（块值是尾表达式）；
                // 非 Fn 注解→块值。后者不得当函数体处理，否则变量拿不到值。
                let is_fn_binding = ast::Expr::block_binding_is_function(
                    type_annotation.as_ref(),
                    value.as_deref(),
                );
                let (params, body): (Vec<_>, Vec<_>) = if is_fn_binding {
                    match value {
                        Some(v) => v.callable_parts(),
                        None => (Vec::new(), Vec::new()),
                    }
                } else {
                    (Vec::new(), Vec::new())
                };
                // 扫描面 = 整棵声明注解（形参标注 ∪ 返回位，SPEC §7.1「全部 Type 子树」）：
                // 只在返回位被引用的编译期值参数同样要进 generic_params（#528 §4-4）。
                let type_positions: Vec<&ast::Type> = type_annotation.iter().collect();
                let generic_params = crate::ast::extract_generic_param_names(
                    &signature,
                    &type_positions,
                    &self.type_level,
                );
                // 登记形参名，供命名参数调用（`add(b = 5, a = 3)`）重排实参。
                //
                // 名字取**声明形态**（WBS 6.8.2a / D54 ③）：函数收几个值、每一位叫什么
                // 由标注决定，实现头的名字只是体内的局部名。旧口径取 Lambda 头
                //（`params`）的名字表，于是实现头一改名，**调用侧跟着改名**：实测
                // `f: (a: Int) -> Int = (b) => b` 的 `f(a = 7)` 报 E1014
                //（available: b），而声明里明明写着 a。声明名 = 签名里不占编译期位的
                // 形参——剔除口径与解析/检查层同源（extract_generic_param_names 判类型位、
                // const 位、约束位）。
                //
                // 与 typecheck 的按位对齐**同一条口径**：运行期值位排在声明序末尾
                //（类型泛型位在首、柯里值层是最内层组），故取末尾 N 位（N = 本函数的
                // 运行期形参数）——与 `check_fn_stmt` 的 `aligned_value_params` 同源。
                // 两处必须用同一条切分规则，否则「调用侧可用名」与「体内形参类型」
                // 会按两套规则切同一个签名（D54 要消灭的正是这种按形态分叉）。
                if is_fn_binding && !params.is_empty() {
                    // 调用侧可用名与形参**类型**同一来源（同一个 helper、同一个顺序）：
                    // 旧口径类型取前 N、名字取末尾 N，在多层柯里声明上会错位
                    // （实测 `f(a = 1, b = "s")` 报 E1014 available: b，而第 0 槽是 a 的槽）。
                    let names: Vec<String> = self
                        .declaration_runtime_param_slots(type_annotation.as_ref(), params.len())
                        .into_iter()
                        .map(|p| p.name)
                        .collect();
                    self.fn_param_names.insert(name.clone(), names);
                }
                if let Some(type_name) = type_name {
                    // RFC-004 外部重绑定：`Type.method = fn` / `Type.method = fn[pos]`
                    // 只登记 type_bindings（方法调用时参数重排），不生成新函数 IR。
                    // #563：提取器对超 i64 位置字面量返回 Err（checker 已报
                    // E1064、管线在 check 出错后中止，本臂实际不可达）——保底
                    // 不登记，防止错误位置进 IR。
                    let rebind = match value.as_deref() {
                        Some(Expr::Var(fn_name, _)) => Some((fn_name.clone(), vec![0i64])),
                        Some(Expr::Index { expr, index, .. }) => match expr.as_ref() {
                            Expr::Var(fn_name, _) => {
                                match ast::Expr::extract_binding_positions(index) {
                                    Ok(positions) => Some((fn_name.clone(), positions)),
                                    Err(_) => None,
                                }
                            }
                            _ => None,
                        },
                        _ => None,
                    };
                    if let Some((function, positions)) = rebind {
                        self.register_type_bindings(
                            &type_name,
                            &[ast::TypeBodyBinding {
                                name: name.clone(),
                                kind: ast::BindingKind::External {
                                    function,
                                    positions,
                                },
                            }],
                        );
                        return Ok(None);
                    }
                    // MethodBind：签名标注是绑定语义的唯一来源（接收者位、形参名、
                    // 返回类型都取自它）。无标注时**不得**对 None 执行 unwrap——
                    // 那会把用户写法错误变成编译器进程 panic（#451，D59 A 类：
                    // 实测 `exit=101`）；此处编译期拒绝并指明补全写法。
                    let Some(method_type) = type_annotation.as_ref() else {
                        // help 不在此硬编（issue #469 处 1）：E1110 的参数化模板
                        // 已在 locales 六语言就位（{type}/{method} 由码表 param 供值）
                        return Err(ErrorCodeDefinition::method_binding_without_signature(
                            &type_name, &name,
                        )
                        .at(stmt.span)
                        .build());
                    };
                    self.generate_method_ir(
                        &type_name,
                        &name,
                        method_type,
                        &params,
                        &body,
                        constants,
                    )
                } else if !params.is_empty() || !body.is_empty() || is_fn_binding {
                    // Fn: 普通函数（含**空块体**的注解形态 `f: () -> Void = { }`——
                    // 注解为 Fn 即函数；不能因 `body.is_empty()` 就落到
                    // 值绑定分支，否则 `f()` 解析不到函数。
                    //
                    // 注：无注解的空块 `f = { }` 到不了这里——它在 parser 层就被
                    // 认作**空 Dict 字面量**（`{}` 二义，见 #359），属值绑定。
                    let generic_param_names = if generic_params.is_empty() {
                        None
                    } else {
                        Some(generic_params.iter().map(|p| p.name.clone()).collect())
                    };
                    // curry 分流：如果 return_type 是 Type::Fn，走 curry 生成路径
                    // 注意：必须传签名全层形参（signature），而非 params（仅 Lambda 参数）
                    if let Some(ast::Type::Fn { return_type, .. }) = type_annotation.as_ref() {
                        if matches!(return_type.as_ref(), ast::Type::Fn { .. }) {
                            self.generate_curry_function_ir(
                                &name,
                                type_annotation.as_ref().unwrap(),
                                &signature,
                                &body,
                                constants,
                                generic_param_names,
                            )
                        } else {
                            self.generate_function_ir(
                                &name,
                                type_annotation.as_ref(),
                                &params,
                                &body,
                                constants,
                                generic_param_names,
                            )
                        }
                    } else {
                        self.generate_function_ir(
                            &name,
                            type_annotation.as_ref(),
                            &params,
                            &body,
                            constants,
                            generic_param_names,
                        )
                    }
                } else if Self::is_type_def_binding(value.as_deref()) {
                    // RFC-010：`Db = unsafe { Db: Type = {...}; Db }` 是**类型定义绑定**，
                    // 不是运行时全局变量。类型已在 typecheck 阶段注册（并提升到本作用域），
                    // 此处不生成运行时代码。
                    // 此前（T2 前）落入 `generate_global_var_ir`，因 `unsafe` 无法常量折叠而报
                    // E3007（当时要求顶层初始化必须是编译期常量；T2 后该门槛已取消）。
                    Ok(None)
                } else {
                    // spec §4.3：首次出现是**声明**（槽位 + 初始化），
                    // 后续同名是**重赋值**（StoreGlobal，进模块语句段）。
                    //
                    // 此前无差别当声明，导致 `mut i = 0; i = i + 1` 被当成
                    // 两份初始化 → 自引用 → 误报 E3019。
                    let key = match &self.module_key {
                        Some(k) => SymbolTable::qualify(k, &name),
                        None => name.clone(),
                    };
                    if self.declared_globals.contains(&key) {
                        // 重赋值：求值后写回已有槽位，进程序主体（顶层语句段）
                        if let Some(slot) = self.lookup_global(&name) {
                            let mut instrs = Vec::new();
                            if let Some(v) = value.as_ref() {
                                let reg = self.next_temp_reg();
                                self.generate_expr_ir(v, reg.clone(), &mut instrs, constants)?;
                                instrs.push(Instruction::Store {
                                    dst: Operand::Global(slot),
                                    src: reg.clone(),
                                    span: Self::get_expr_span(v),
                                });
                            }
                            self.module_stmts.extend(instrs);
                        }
                        Ok(None)
                    } else {
                        self.declared_globals.insert(key);
                        // 全局变量（T2：槽位 + 初始化指令）
                        self.generate_global_var_ir(
                            &name,
                            type_annotation.as_ref(),
                            value.as_ref().map(|v| v.as_ref()),
                            constants,
                        )
                    }
                }
            }
            // 导入语句：解析在独立 pass 完成（build_import_aliases），不生成运行时代码
            ast::StmtKind::Use { .. } => Ok(None),
            // T4：Script 模式（无 yaoxiang.toml）下顶层就是程序主体——
            // 任意可执行语句按书写顺序编进模块初始化序列。
            //
            // Bin 模式（有 manifest）仍拒绝：那里程序入口是 `main`，
            // 顶层散语句会让「先建什么后建什么」含糊。
            // 注：初始化序列本身也是代码——所以顶层可执行语句不报错，
            // 而是像 Python 一样按序执行。这是「默认情况零门槛」的代价与红利。
            _ if self.script_mode => {
                // T4：顶层可执行语句（Script 模式）——先收进 module_stmts，
                // 待所有绑定初始化（拓扑序）就绪后再拼到后面。
                //
                // 跳转重定位：语句在空 Vec 上生成，内部回跳（如 while）
                // 的 target 从 0 起算；拼进 module_stmts 后必须加基线偏移。
                let base = self.module_stmts.len();
                let mut instrs = Vec::new();
                self.generate_local_stmt_ir(stmt, &mut instrs, constants)?;
                Self::rebase_jump_targets(&mut instrs, base);
                self.module_stmts.extend(instrs);
                Ok(None)
            }
            _ => {
                // 多文件/有 manifest：顶层只允许定义（绑定/类型/导入）；
                // 可执行语句必须在 main 体内（规范 §3.11）。
                //
                // 这是**用户写法错误**，不是编译器缺陷——此前挨 E3005「IR 内部
                // 错误，请报告此问题」并把 `Discriminant(N)` 泄给用户，既指错方向
                // 又误导提单（同族：#360 索引赋值、#311 break）。
                Err(ErrorCodeDefinition::bin_top_level_statement()
                    .at(stmt.span)
                    .build())
            }
        }
    }

    /// 生成方法 IR
    #[allow(clippy::too_many_arguments)]
    fn generate_method_ir(
        &mut self,
        type_name: &str,
        method_name: &str,
        method_type: &ast::Type,
        params: &[ast::Param],
        body: &[ast::Stmt],
        constants: &mut Vec<ConstValue>,
    ) -> Result<Option<FunctionIR>, Diagnostic> {
        // 命名空间机制：方法函数名 = Type.method
        // 例如：Point.get_x 生成函数名 "Point.get_x"
        // 调用时：p.get_x() -> Point.get_x(p)
        let base_name = format!("{}.{}", type_name, method_name);
        // RFC-011a §3：同名重载的定义序混编（首定义裸名，后续 #序）——
        // 与 typecheck 注册序（AST 序）对齐，调用点决议按名派发
        let ordinal = self
            .method_def_ordinals
            .entry(base_name.clone())
            .or_insert(0);
        let counted = *ordinal;
        *ordinal += 1;
        let func_name = if counted == 0 {
            base_name
        } else {
            format!("{base_name}#{counted}")
        };

        // #466：重载候选的形参名按**混编名**精确登记——叶子名键（Assign 臂
        // 的登记）在同名重载下末次覆盖，决议命中 `Temp.pick` 的调用会按末候选
        // `Temp.pick#1` 的形参名绑定（E1014 available: 错候选）。混编名与候选
        // 一一对应，调用点按决议出的名字查（`lookup_param_info` 先全名后叶名）。
        // 名字口径与普通函数绑定**同一规则**（D54 ③：调用侧可用名取声明，实现头的
        // 名字只是体内局部名）。方法声明的首位是接收者位，suffix-N 仍按位置对齐。
        if !params.is_empty() {
            // 与普通函数绑定同一来源（同一个 helper）：方法声明的首位是接收者位，
            // 位置口径一致 ⇒ 调用侧可用名与槽位不会错位。
            let names: Vec<String> = self
                .declaration_runtime_param_slots(Some(method_type), params.len())
                .into_iter()
                .map(|p| p.name)
                .collect();
            self.fn_param_names.insert(func_name.clone(), names);
        }

        // 注册方法到 type_bindings，使方法调用脱糖能找到绑定
        let binding_entry = ast::TypeBodyBinding {
            name: method_name.to_string(),
            kind: ast::BindingKind::DefaultExternal {
                function: func_name.clone(),
            },
        };
        self.register_type_bindings(type_name, &[binding_entry]);

        // 解析返回类型
        let return_type = if let ast::Type::Fn { return_type, .. } = method_type {
            (**return_type).clone().into()
        } else {
            // 非函数类型，报错
            return Err(ErrorCodeDefinition::ir_internal_error(&format!(
                "Method {} is not a function type",
                method_name
            ))
            .build());
        };

        // 进入新作用域
        self.enter_scope();

        // 注册参数；类型按**方法自己的声明签名**渲染（与普通函数同一 helper：
        // 作者标注优先、缺标注取声明同类位；IR 只渲染不判定——parser 已不再
        // 往实现头填声明类型）。
        let declared = self.declaration_runtime_param_slots(Some(method_type), params.len());
        let param_types = self.impl_param_types(params, Some(method_type));
        // 槽位取**调用点实参序列**里的下标：绑定第 i 位 ≠ 运行期第 i 位
        //（见 `impl_head_runtime_indices`）。
        let runtime_indices = self.impl_head_runtime_indices(Some(method_type));
        for (i, param) in params.iter().enumerate() {
            // 注册参数到符号表
            self.register_local(&param.name, runtime_indices.get(i).copied().unwrap_or(i));
        }
        self.note_param_written_types(params, &declared);
        // 量词事实（q=0 槽位）随声明一起落到形参槽位，供擦除前识别。
        self.note_param_quantities(params, &declared);

        // 记录局部变量起始位置（在参数之后）
        // 帧宽只看**实际用到的**槽位：`take(params.len())` 保证「声明比实现头多」的形态
        // 不会凭空加宽（`locals_total` 因此与改动前一致）。
        let used_max = runtime_indices
            .iter()
            .take(params.len())
            .max()
            .map_or(0, |m| m + 1);
        let local_var_start = params.len().max(used_max);
        // WBS 7a.2：方法体是独立指令流——父帧的表（槽位表 / 循环栈 / 当前 span）
        // 整体摘进 guard，Drop 时原样放回。声明面入口（`enter_decl`）：方法体属于
        // 模块级声明，模块初始化序列的寄存器计数沿用既有口径（接着用子体终值），
        // 不改编号（C1 zero-diff）。#368：模块级已登记的具名槽位随 Drop 原样放回。
        let mut frame = NestedBodyGuard::enter_decl(self);
        frame.next_temp = local_var_start;
        // #393：本函数是独立寄存器空间——高水位与循环态地板随之清零
        frame.temp_high_water = local_var_start;
        frame.temp_floor = 0;

        // 生成指令序列
        let mut instructions = Vec::new();

        // RFC-010a 规则①：尾表达式给出函数值。沿用共用的 `generate_tail_stmt_ir`
        // （与 generate_function_ir / generate_block_ir 同一实现），
        // 否则 `Point.getX: (&Point) -> Float = { self.x }` 返回 Void；
        // 取值不以注解为条件——注解只决定检查，不决定求值（#343 根因一）。
        let last_idx = body.len().checked_sub(1);
        let mut tail_handled = false;
        for (i, stmt) in body.iter().enumerate() {
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
            frame.set_cur_span(stmt.span);
        }
        if !tail_handled {
            instructions.push(Instruction::Ret {
                value: None,
                span: frame.cur_span,
            });
        }

        // 退出作用域
        frame.exit_scope();

        // WBS 2.1.2 实测缺陷修复：此前传 param_types.len()，locals 表被截断到
        // 参数数，方法体的临时槽（高水位之内）全部丢失——local_count 低估，
        // 运行时靠 Frame::set_slot 越界 resize（frames.rs:103-105）静默容错。
        // 与 generate_function_ir / generate_lambda_body_ir 对齐：以高水位为准。
        // 行为等价论证：Frame 预分配槽位初值同为 Void，set_slot 容错路径不再
        // 触发但结果相同；调试信息 local_names 只取具名槽，不受空槽扩展影响。
        // 收尾单点：值类型 → 块切分 → 槽分配 → 预算闸 → 取表（7b.4）
        let body = frame.finalize_code_body(instructions, &param_types)?;

        // 构建函数 IR
        let func_ir = FunctionIR {
            def: None, // 由 generate_module_ir 尾部 assign_defs 填充
            name: func_name.clone(),
            params: param_types.clone(),
            return_type,
            generic_params: None,
            body: FunctionBody::Code {
                blocks: body.blocks,
                entry: 0,
                locals: body.locals,
                values: body.values,
                value_slots: body.value_slots,
            },
        };

        Ok(Some(func_ir))
    }

    /// 生成函数 IR
    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::only_used_in_recursion)]
    pub(crate) fn generate_function_ir(
        &mut self,
        name: &str,
        type_annotation: Option<&ast::Type>,
        params: &[ast::Param],
        body: &[ast::Stmt],
        constants: &mut Vec<ConstValue>,
        generic_params: Option<Vec<String>>,
    ) -> Result<Option<FunctionIR>, Diagnostic> {
        // 括号返回 / 闭包包装形态：把声明**返回位**内层 Fn 的形参递给闭包 IR 定型
        //（parser 不再填 AST 标注；入口 save、出口还原，互不串味）。
        let closure_decl_params = self.paren_return_value_params(type_annotation);
        let saved_closure_decl =
            std::mem::replace(&mut self.closure_decl_params, closure_decl_params);

        // 检测 native("symbol") 模式：函数体为空语句 + Native("...") 表达式
        // 检测 native("symbol") 模式：函数体为空语句 + Native("...") 表达式
        // 形如: my_add: (a: Int, b: Int) -> Int = Native("my_add")
        //
        // 通过 name resolution 检测，不再硬编码 Var("Native") 字符串匹配。
        // Native 是 std.ffi 模块中真实存在的函数，名称通过 ModuleRegistry 解析。
        if body.is_empty() {
            // 空函数体，无法检测 native 模式
        }

        // 阶段3修复：改进返回类型解析，更好地与类型检查集成
        let return_type = match type_annotation {
            Some(ast::Type::Fn { return_type, .. }) => (**return_type).clone().into(),
            Some(ty) => ty.clone().into(),
            None => MonoType::Void,
        };

        // WBS 7a.2：函数体是独立指令流——父帧的表（槽位表 / 循环栈 / 当前 span）
        // 整体摘进 guard，Drop 时原样返回；三项计数装到 local_var_start。
        // 声明面入口（`enter_decl`）：模块级声明的函数体不改模块初始化序列的计数
        // 编号（C1 zero-diff）；函数体内的嵌套定义由调用点的 guard 保护（第 7 处）。
        //
        // 槽位取**调用点实参序列**里的下标：绑定第 i 位 ≠ 运行期第 i 位
        //（见 `impl_head_runtime_indices`）——`f: (N: Int, M: N) -> Int = (m) => m`
        // 的 `m` 是绑定第 0 位、运行期第 1 位。
        let runtime_indices = self.impl_head_runtime_indices(type_annotation);
        // 记录局部变量起始位置（在参数之后）
        // 帧宽只看**实际用到的**槽位：`take(params.len())` 保证「声明比实现头多」的形态
        // 不会凭空加宽（`locals_total` 因此与改动前一致）。
        let used_max = runtime_indices
            .iter()
            .take(params.len())
            .max()
            .map_or(0, |m| m + 1);
        let local_var_start = params.len().max(used_max);
        let mut frame = NestedBodyGuard::enter_decl(self);
        frame.next_temp = local_var_start;
        // #393：本函数是独立寄存器空间——高水位与循环态地板随之清零
        frame.temp_high_water = local_var_start;
        frame.temp_floor = 0;

        // 生成函数体指令
        let mut instructions = Vec::new();

        // 进入函数体作用域
        frame.enter_scope();

        // 为每个参数生成 LoadArg 指令并注册
        for (i, param) in params.iter().enumerate() {
            let slot = runtime_indices.get(i).copied().unwrap_or(i);
            instructions.push(Instruction::Load {
                dst: Operand::Local(slot),
                src: Operand::Arg(slot),
                span: frame.cur_span,
            });

            frame.register_local(&param.name, slot);
        }
        // 写出类型名与形参类型同一来源（作者标注 → 声明同类位）。
        let declared = frame.declaration_runtime_param_slots(type_annotation, params.len());
        frame.note_param_written_types(params, &declared);
        // 量词事实（q=0 槽位）随声明一起落到形参槽位，供擦除前识别。
        frame.note_param_quantities(params, &declared);

        // 检查函数体是否为 FFI ExternRef 绑定
        if let Some(ConstValue::ExternRef {
            mechanism,
            lib,
            symbol,
        }) = frame.try_eval_body_as_extern_ref(body)
        {
            let lib_id = frame.get_or_create_lib_id(&mechanism, &lib);
            frame
                .ffi_bindings
                .push(crate::middle::ir::ir::FfiBinding::FuncBinding {
                    func_name: name.to_string(),
                    lib_id,
                    symbol: symbol.clone(),
                });
            return Ok(None);
        }

        // 生成语句 IR
        // RFC-010a 规则①：函数体的值 = 尾表达式。尾位置判断复用
        // `generate_tail_stmt_ir`（与 `generate_block_ir` 同一实现），
        // 否则 `{ if c {5} else {6} }` 这类尾位置 `if` 的值会被丢弃（#344）。
        //
        // 取值**不以注解为条件**：注解只决定要不要*检查*（typecheck 侧负责），
        // 不决定要不要*求值*。此前用 `return_type != Void` 挡在取值前，
        // 导致无注解的 `f = { 5 }` / `f = () => { 5 }` 尾表达式被静默丢弃、
        // 返回 Void（#343 根因一）。
        let last_idx = body.len().checked_sub(1);
        let mut tail_handled = false;
        for (i, stmt) in body.iter().enumerate() {
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
            tlog!(
                debug,
                MSG::IrGenBeforeProcessStmt,
                &frame.symbols.len().to_string()
            );
            frame.generate_local_stmt_ir(stmt, &mut instructions, constants)?;
            // NLL Release: 在语句边界消费释放计划并插入 Drop 指令
            frame.emit_release_drops(stmt.plan_id, &mut instructions);
            // #393：语句级临时寄存器回收（同 generate_block_ir——临时不跨
            // 语句存活，回滚到具名水位与循环态地板的较大者）
            let named_watermark = frame
                .cur_locals
                .iter()
                .rposition(|slot| slot.name.is_some())
                .map_or(0, |i| i + 1);
            frame.next_temp = named_watermark.max(frame.temp_floor);
            tlog!(
                debug,
                MSG::IrGenAfterProcessStmt,
                &frame.symbols.len().to_string()
            );
            frame.set_cur_span(stmt.span);
        }
        if !tail_handled {
            instructions.push(Instruction::Ret {
                value: None,
                span: frame.cur_span,
            });
        }

        // 退出函数体作用域
        tlog!(
            debug,
            MSG::IrGenAboutToExitScope,
            &frame.symbols.len().to_string()
        );
        frame.exit_scope();
        tlog!(
            debug,
            MSG::IrGenAfterExitScope,
            &frame.symbols.len().to_string()
        );

        // 收尾单点：值类型 → 块切分 → 槽分配 → 预算闸（E3014，带 span）→ 取表。
        // 函数体形态共用 finalize_code_body（7b.4）；形参类型须先算出来（它就是
        // 值类型推导的形参签名来源）。
        let param_types = frame.impl_param_types(params, type_annotation);
        let body = frame.finalize_code_body(instructions, &param_types)?;

        // 还原闭包定型上下文（避免串到下一个函数）——非寄存器状态，仍手工配对
        frame.closure_decl_params = saved_closure_decl;

        // 构建函数 IR
        let func_ir = FunctionIR {
            def: None, // 由 generate_module_ir 尾部 assign_defs 填充
            name: name.to_string(),
            // 形参类型：作者标注优先，缺标注按**函数自己的声明签名**同类位渲染
            // （parser 不再往实现头填声明类型，声明是 IR 唯一的类型来源；D54 F1 /
            // WBS 6.8.3）。拿不到声明类型时按 WBS 2.1.2 的惯例填 Int(64) 占位——
            // filter_map 丢弃会让 params 长度与帧布局错位（verify 实测）。
            params: param_types,
            return_type,
            generic_params,
            body: FunctionBody::Code {
                blocks: body.blocks,
                entry: 0,
                locals: body.locals,
                values: body.values,
                value_slots: body.value_slots,
            },
        };

        Ok(Some(func_ir))
    }

    /// 在语句边界设定当前 span：之后所有指令构造就地捕获它。
    pub(in crate::middle) fn set_cur_span(
        &mut self,
        span: Span,
    ) {
        if !span.is_dummy() {
            self.cur_span = span;
        }
    }

    /// 语句边界的释放计划消费（RFC-039 D20 / WBS 4.5.1）——**唯一消费点**。
    ///
    /// `ReleasePlan::consume` 命中即记账（消费台账随计划本体走），故
    /// 「查表 → 记账 → 发 Drop」是一次动作，台账不可能与真实消费漂移。
    /// 所有语句边界（函数体 / curry 层 / 方法体 / lambda 体 / 嵌套块 / 尾语句
    /// 路径）都必须经过它：漏一处，该语句的计划键就没有消费，义务账本结算
    /// 报 W1090（D41——差集必须为空，无白名单）。
    pub(in crate::middle) fn emit_release_drops(
        &mut self,
        statement: PlanId,
        instructions: &mut Vec<Instruction>,
    ) {
        let Some(vars) = self.release_plan.consume(statement) else {
            return;
        };
        for var in vars {
            if let Some(local_idx) = self.lookup_local(var) {
                instructions.push(Instruction::Drop {
                    src: Operand::Local(local_idx),
                    span: self.cur_span,
                });
            }
        }
    }

    pub(in crate::middle) fn generate_local_stmt_ir(
        &mut self,
        stmt: &ast::Stmt,
        instructions: &mut Vec<Instruction>,
        constants: &mut Vec<ConstValue>,
    ) -> Result<(), Diagnostic> {
        match &stmt.kind {
            ast::StmtKind::Expr(expr) => {
                let result_reg = self.next_temp_reg();
                self.generate_expr_ir(expr, result_reg, instructions, constants)?;
            }
            ast::StmtKind::Assign {
                target,
                type_annotation,
                value,
                span,
                ..
            } => {
                use crate::ast::Expr;
                // 签名形参的唯一来源：注解的 Type::Fn 链（6.5.2）
                let signature = type_annotation
                    .as_ref()
                    .map(crate::ast::Type::fn_params_flat)
                    .unwrap_or_default();
                // 字段赋值：q.x = v / m.x = v（m 是 &mut 令牌）。
                // Struct 运行时值是堆句柄，StoreField 原地写共享对象——
                // 令牌与自动借用天然写回，无需 copy-back（#266）。
                if let Expr::FieldAccess {
                    expr: obj_expr,
                    field,
                    ..
                } = target.as_ref()
                {
                    let field_index =
                        self.resolve_field_index(obj_expr, field).ok_or_else(|| {
                            ErrorCodeDefinition::ir_internal_error(&format!(
                                "无法解析字段索引: '{}'",
                                field
                            ))
                            .at(Self::get_expr_span(obj_expr))
                            .build()
                        })?;
                    let obj_reg = self.next_temp_reg();
                    self.generate_expr_ir(obj_expr, obj_reg.clone(), instructions, constants)?;
                    let val_reg = self.next_temp_reg();
                    let value_expr = value.as_ref().ok_or_else(|| {
                        ErrorCodeDefinition::ir_internal_error("字段赋值缺少右侧值")
                            .at(*span)
                            .build()
                    })?;
                    self.generate_expr_ir(value_expr, val_reg.clone(), instructions, constants)?;
                    instructions.push(Instruction::StoreField {
                        dst: obj_reg.clone(),
                        field: field_index,
                        src: val_reg.clone(),
                        type_name: None,
                        field_name: Some(field.clone()),
                        span: *span,
                    });
                    return Ok(());
                }
                // 索引赋值：a[i] = v。
                // 容器运行时值是堆句柄，StoreIndex 原地写共享对象——
                // 与字段赋值同理（#266/#360）：Array/Vec/List 均按原地写处理，
                // 语义与值语义的 list.set 区分开（后者在 std 层返回新列表）。
                if let Expr::Index {
                    expr: base_expr,
                    index: index_expr,
                    ..
                } = target.as_ref()
                {
                    let value_expr = value.as_ref().ok_or_else(|| {
                        ErrorCodeDefinition::ir_internal_error("索引赋值缺少右侧值")
                            .at(*span)
                            .build()
                    })?;
                    let base_reg = self.next_temp_reg();
                    self.generate_expr_ir(base_expr, base_reg.clone(), instructions, constants)?;
                    let index_reg = self.next_temp_reg();
                    self.generate_expr_ir(index_expr, index_reg.clone(), instructions, constants)?;
                    let val_reg = self.next_temp_reg();
                    self.generate_expr_ir(value_expr, val_reg.clone(), instructions, constants)?;
                    instructions.push(Instruction::StoreIndex {
                        dst: base_reg.clone(),
                        index: index_reg.clone(),
                        src: val_reg.clone(),
                        span: *span,
                    });
                    return Ok(());
                }
                let name = match target.as_ref() {
                    Expr::Var(n, _) => n.clone(),
                    // 不认识的赋值目标：显式报错，不静默吞掉（#266）
                    _ => {
                        return Err(ErrorCodeDefinition::ir_internal_error(&format!(
                            "不支持的赋值目标: {:?}",
                            target
                        ))
                        .at(Self::get_expr_span(target))
                        .build())
                    }
                };
                // RFC-010a 附录D：`name = { ... }` 无注解时按内容推断（块值是尾表达式）；
                // 非 Fn 注解→块值。
                let is_fn_binding = ast::Expr::block_binding_is_function(
                    type_annotation.as_ref(),
                    value.as_deref(),
                );
                let (params, body): (Vec<_>, Vec<_>) = if is_fn_binding {
                    match value {
                        Some(v) => v.callable_parts(),
                        None => (Vec::new(), Vec::new()),
                    }
                } else {
                    (Vec::new(), Vec::new())
                };
                // 如果有 params/body，是嵌套函数
                if !params.is_empty() || !body.is_empty() {
                    // 同 Assign 面：扫描面 = 整棵声明注解（含返回位）。
                    let type_positions: Vec<&ast::Type> = type_annotation.iter().collect();
                    let generic_params = crate::ast::extract_generic_param_names(
                        &signature,
                        &type_positions,
                        &self.type_level,
                    );
                    let generic_param_names = if generic_params.is_empty() {
                        None
                    } else {
                        Some(generic_params.iter().map(|p| p.name.clone()).collect())
                    };
                    // WBS 7a.2 第 7 处：嵌套函数定义自带独立帧（子体在 `enter_decl` 里
                    // 自行复位计数），**父函数的帧必须在调用返回后原样恢复**——否则父体
                    // 后续语句从子体基数继续分配临时槽：槽位互踩，且父体 `locals_total`
                    // 可能小于它已引用的槽位（越界）。此前只有 curry/lambda/anon 路径与
                    // next_temp 成对 save/restore，这条路径漏了（本条即补上）。
                    //
                    // curry 分流：如果 return_type 是 Type::Fn，走 curry 生成路径
                    // 注意：必须传签名全层形参（signature），而非 params（仅 Lambda 参数）
                    let func_result = {
                        let mut guard = NestedBodyGuard::enter(self);
                        if let Some(ast::Type::Fn { return_type, .. }) = type_annotation.as_ref() {
                            if matches!(return_type.as_ref(), ast::Type::Fn { .. }) {
                                guard.generate_curry_function_ir(
                                    &name,
                                    type_annotation.as_ref().unwrap(),
                                    &signature,
                                    &body,
                                    constants,
                                    generic_param_names,
                                )
                            } else {
                                guard.generate_function_ir(
                                    &name,
                                    type_annotation.as_ref(),
                                    &params,
                                    &body,
                                    constants,
                                    generic_param_names,
                                )
                            }
                        } else {
                            guard.generate_function_ir(
                                &name,
                                type_annotation.as_ref(),
                                &params,
                                &body,
                                constants,
                                generic_param_names,
                            )
                        }
                    };
                    match func_result {
                        Ok(Some(func_ir)) => {
                            self.nested_functions.push(func_ir);
                        }
                        Ok(None) => {}
                        Err(e) => return Err(e),
                    }
                    return Ok(());
                }
                // RFC-010：`T = unsafe { T: Type = {...}; T }` 是**类型定义绑定**
                // （编译期构造，类型已提升到本作用域），不生成运行时变量。
                // 否则体内引用 `T` 会报 E1001（未知变量）。
                if Self::is_type_def_binding(value.as_deref()) {
                    return Ok(());
                }
                // 普通变量
                let initializer = value.as_ref().map(|v| v.as_ref());
                // WBS 4.9.8 / issue #465：记下**写出**的类型名（注解原文优先，
                // 否则取构造器名），供方法调用派发归因到定义单元。
                let written_type = type_annotation
                    .as_ref()
                    .and_then(Self::written_type_name_of)
                    .map(str::to_string)
                    .or_else(|| initializer.and_then(|e| self.constructor_written_type_name(e)));
                if let Some(type_ann) = type_annotation {
                    let mono: MonoType = type_ann.clone().into();
                    let type_name = mono.type_name();
                    self.local_var_types.insert(name.clone(), type_name.clone());
                    if mono.is_constraint() {
                        if let Some(init_expr) = initializer {
                            if let Some(concrete_type_name) =
                                self.get_expr_struct_type_name(init_expr)
                            {
                                self.constraint_var_concrete_types
                                    .insert(name.clone(), concrete_type_name);
                            }
                        }
                    }
                } else if let Some(init_expr) = initializer {
                    let inferred = self.get_expr_type_name(init_expr);
                    if inferred != "<unknown>" {
                        self.local_var_types.insert(name.clone(), inferred);
                    }
                }
                // 顶层绑定（Script 模式下 `mut i = 0` 在模块初始化序列里）：
                // 重赋值写回全局槽位，不能新建局部（否则值被丢弃、循环永不推进）。
                // 此路径在循环体内尤其关键——`while` 体里的 `i = i + 1`
                // 走的就是 `generate_local_stmt_ir`。
                if self.lookup_local(&name).is_none() {
                    if let Some(slot) = self.lookup_global(&name) {
                        if let Some(expr) = initializer {
                            let reg = self.next_temp_reg();
                            self.generate_expr_ir(expr, reg.clone(), instructions, constants)?;
                            instructions.push(Instruction::Store {
                                dst: Operand::Global(slot),
                                src: reg.clone(),
                                span: Self::get_expr_span(expr),
                            });
                        }
                        return Ok(());
                    }
                }
                let var_idx = if let Some(existing_idx) = self.lookup_local(&name) {
                    existing_idx
                } else {
                    let idx = self.next_local_slot();
                    self.register_local(&name, idx);
                    idx
                };
                if let Some(written) = written_type {
                    self.note_local_written_type(&name, &written);
                }
                if let Some(expr) = initializer {
                    // #299 §2：字面量上下文落点——Array 注解直接作用于 List 字面量时，
                    // 生成定长数组构造而非 List。禁止隐式 List→Array 转换：仅字面量种子直接落 Array。
                    if let (Some(ann), ast::Expr::List(elems, lit_span)) = (type_annotation, expr) {
                        let mono: MonoType = ann.clone().into();
                        if mono.is_array() {
                            let mut elem_regs = Vec::with_capacity(elems.len());
                            for elem in elems {
                                let reg = self.next_temp_reg();
                                self.generate_expr_ir(elem, reg.clone(), instructions, constants)?;
                                elem_regs.push(reg);
                            }
                            instructions.push(Instruction::AllocFixedArray {
                                dst: Operand::Local(var_idx),
                                count: elems.len(),
                                span: *lit_span,
                            });
                            for (i, reg) in elem_regs.iter().enumerate() {
                                let idx_reg = self.next_temp_reg();
                                instructions.push(Instruction::Load {
                                    dst: idx_reg.clone(),
                                    src: Operand::Const(ConstValue::Int(i as i128)),
                                    span: self.cur_span,
                                });
                                instructions.push(Instruction::StoreIndex {
                                    dst: Operand::Local(var_idx),
                                    index: idx_reg.clone(),
                                    src: reg.clone(),
                                    span: *lit_span,
                                });
                            }
                            return Ok(());
                        }
                    }
                    // 统一走 generate_expr_ir — 把 RHS 值直接放入 var_idx 槽
                    self.generate_expr_ir(expr, Operand::Local(var_idx), instructions, constants)?;
                } else {
                    instructions.push(Instruction::Load {
                        dst: Operand::Local(var_idx),
                        src: Operand::Const(ConstValue::Int(0)),
                        span: self.cur_span,
                    });
                }
            }
            ast::StmtKind::If {
                condition,
                then_branch,
                else_if_branches,
                else_branch,
                span: _,
            } => {
                // 生成 if 语句的 IR
                self.generate_if_stmt_ir(
                    condition,
                    then_branch,
                    else_if_branches,
                    else_branch.as_deref(),
                    instructions,
                    constants,
                )?;
            }
            ast::StmtKind::For {
                var,
                var_span: _,
                var_mut,
                iterable,
                body,
            } => {
                self.generate_for_loop_ir(
                    var,
                    *var_mut,
                    iterable,
                    body,
                    None, // No result needed for statement
                    stmt.span,
                    instructions,
                    constants,
                )?;
            }
            ast::StmtKind::DestructureAssign { names, rhs, span } => {
                // 优化：如果 RHS 是元组字面量 (1, 2)，直接提取每个元素赋值
                // 否则通过 LoadIndex 索引
                if let ast::Expr::Tuple(elems, _) = rhs.as_ref() {
                    for (i, name) in names.iter().enumerate() {
                        let var_idx = self.next_local_slot();
                        self.register_local(&name.name, var_idx);

                        if let Some(elem) = elems.get(i) {
                            self.generate_expr_ir(
                                elem,
                                Operand::Local(var_idx),
                                instructions,
                                constants,
                            )?;
                        }
                    }
                } else {
                    // 非字面量元组：生成 RHS，然后通过 LoadIndex 提取
                    let rhs_reg = self.next_temp_reg();
                    self.generate_expr_ir(rhs, rhs_reg.clone(), instructions, constants)?;

                    for (i, name) in names.iter().enumerate() {
                        let var_idx = self.next_local_slot();
                        self.register_local(&name.name, var_idx);

                        let index_reg = self.next_temp_reg();
                        instructions.push(Instruction::Load {
                            dst: index_reg.clone(),
                            src: Operand::Const(ConstValue::Int(i as i128)),
                            span: self.cur_span,
                        });

                        instructions.push(Instruction::LoadIndex {
                            dst: Operand::Local(var_idx),
                            src: rhs_reg.clone(),
                            index: index_reg.clone(),
                            span: *span,
                        });
                    }
                }
            }
            ast::StmtKind::Return(expr) => match expr {
                Some(e) => {
                    let result_reg = self.next_temp_reg();
                    self.generate_expr_ir(e, result_reg.clone(), instructions, constants)?;
                    instructions.push(Instruction::Ret {
                        value: Some(result_reg.clone()),
                        span: self.cur_span,
                    });
                }
                None => {
                    instructions.push(Instruction::Ret {
                        value: None,
                        span: self.cur_span,
                    });
                }
            },
            // 合法不产生代码的语句：
            // - Use：导入解析在独立 pass 完成，无运行时代码
            // - Error：解析器恢复占位符；存在解析错误时编译已在此前终止
            ast::StmtKind::Use { .. } | ast::StmtKind::Error(_) => {}
            // 块内类型定义不受支持：此前被静默忽略（定义即无效，使用时才在远处报 E1001）
            ast::StmtKind::TypeDefinition { name, .. } => {
                return Err(ErrorCodeDefinition::ir_internal_error(&format!(
                    "type definition `{}` inside a block is not supported; type definitions belong at the top level",
                    name
                ))
                .at(stmt.span)
                .build());
            }
        }
        Ok(())
    }

    /// 生成 if 语句的 IR
    fn generate_if_stmt_ir(
        &mut self,
        condition: &ast::Expr,
        then_branch: &ast::Block,
        else_if_branches: &[(Box<ast::Expr>, Box<ast::Block>)],
        else_branch: Option<&ast::Block>,
        instructions: &mut Vec<Instruction>,
        constants: &mut Vec<ConstValue>,
    ) -> Result<(), Diagnostic> {
        // 进入新的作用域
        self.enter_scope();

        // 1. 评估条件
        let condition_reg = self.next_temp_reg();
        self.generate_expr_ir(condition, condition_reg.clone(), instructions, constants)?;

        // 2. 跳转到下一个分支的占位符 (JmpIfNot to next_branch)
        let jump_to_next_branch_idx = instructions.len();
        instructions.push(Instruction::JmpIfNot {
            cond: condition_reg.clone(),
            target: 0,
            span: self.cur_span,
        }); // 占位符

        // 3. 生成 then 分支
        self.generate_block_ir(then_branch, None, instructions, constants)?;

        // 4. then 分支结束后，跳转到整个 if 语句的结束 (Jmp to end)
        let mut jump_to_end_indices = Vec::new();
        // 只有当有 else/else if 时才需要跳过它们，否则这里已经是 end
        if !else_if_branches.is_empty() || else_branch.is_some() {
            let idx = instructions.len();
            instructions.push(Instruction::Jmp {
                target: 0,
                span: self.cur_span,
            }); // 占位符
            jump_to_end_indices.push(idx);
        }

        // 5. 修复条件跳转 (JmpIfNot)，使其指向 else if 或 else (即当前位置)
        let len = instructions.len();
        if let Instruction::JmpIfNot {
            cond: _,
            ref mut target,
            span: _,
        } = instructions[jump_to_next_branch_idx]
        {
            *target = len;
        }

        // 6. 处理 else if 分支
        for (else_if_condition, else_if_body) in else_if_branches.iter() {
            // 评估 else if 条件
            let else_if_condition_reg = self.next_temp_reg();
            self.generate_expr_ir(
                else_if_condition,
                else_if_condition_reg.clone(),
                instructions,
                constants,
            )?;

            // 跳转到下一个分支 (JmpIfNot)
            let jump_to_next_else_if_idx = instructions.len();
            instructions.push(Instruction::JmpIfNot {
                cond: else_if_condition_reg.clone(),
                target: 0,
                span: self.cur_span,
            });

            // 生成 else if 分支
            self.generate_block_ir(else_if_body, None, instructions, constants)?;

            // else if 分支结束后跳转到结束
            let idx = instructions.len();
            instructions.push(Instruction::Jmp {
                target: 0,
                span: self.cur_span,
            }); // 占位符
            jump_to_end_indices.push(idx);

            // 修复条件跳转
            let len = instructions.len();
            if let Instruction::JmpIfNot {
                cond: _,
                ref mut target,
                span: _,
            } = instructions[jump_to_next_else_if_idx]
            {
                *target = len;
            }
        }

        // 7. 生成 else 分支
        if let Some(else_body) = else_branch {
            self.generate_block_ir(else_body, None, instructions, constants)?;
        }

        // 8. 修复所有跳转到结束的指令
        let end_pos = instructions.len();
        for idx in jump_to_end_indices {
            if let Instruction::Jmp {
                ref mut target,
                span: _,
            } = instructions[idx]
            {
                *target = end_pos;
            }
        }

        // 退出作用域
        self.exit_scope();

        Ok(())
    }

    /// 生成代码块的 IR
    ///
    /// 当 `result_reg` 为 `Some(reg)` 时，表示块作为表达式使用：
    /// 块中最后一条语句如果是表达式（`StmtKind::Expr`），
    /// 其值直接写入 `reg` 作为块的返回值。
    /// 块中没有 return 也没有尾部表达式时，`reg` 保持默认（Void）。
    ///
    /// 当 `result_reg` 为 `None` 时，块作为语句序列执行，不关心返回值。
    /// 尾位置语句求值（RFC-010a 规则①）：把语句作为块值写入 `result_reg`。
    ///
    /// 返回 `true` 表示已处理（值已写入），调用方应跳过普通语句生成。
    ///
    /// 提取为共用方法是因为 `generate_block_ir`（块）与 `generate_function_ir`（函数体）
    /// 各写了一套尾位置判断，两套不一致——函数体只认 `StmtKind::Expr`，
    /// 导致 `f: () -> Int = { if c {5} else {6} }` 的 `if` 值被丢弃（#344）。
    pub(in crate::middle) fn generate_tail_stmt_ir(
        &mut self,
        stmt: &ast::Stmt,
        result_reg: Operand,
        instructions: &mut Vec<Instruction>,
        constants: &mut Vec<ConstValue>,
    ) -> Result<bool, Diagnostic> {
        match &stmt.kind {
            ast::StmtKind::Expr(expr) => {
                self.generate_expr_ir(expr, result_reg, instructions, constants)?;
                Ok(true)
            }
            ast::StmtKind::If {
                condition,
                then_branch,
                else_if_branches,
                else_branch,
                ..
            } => {
                self.generate_if_expr_ir(
                    condition,
                    then_branch,
                    else_if_branches,
                    else_branch.as_deref(),
                    result_reg,
                    instructions,
                    constants,
                )?;
                Ok(true)
            }
            // 其余语句形态（赋值 / return / 循环 / …）不贡献块值：
            // 末位赋值语句值为 Void（RFC-010a 规则①）
            _ => Ok(false),
        }
    }

    pub(in crate::middle) fn generate_block_ir(
        &mut self,
        block: &ast::Block,
        result_reg: Option<Operand>,
        instructions: &mut Vec<Instruction>,
        constants: &mut Vec<ConstValue>,
    ) -> Result<(), Diagnostic> {
        // 进入新的作用域
        self.enter_scope();

        let last_idx = block.stmts.len().checked_sub(1);
        for (i, stmt) in block.stmts.iter().enumerate() {
            let is_last = Some(i) == last_idx;
            // 语句边界设定当前 span，供本条语句的所有指令构造就地捕获
            self.set_cur_span(stmt.span);
            // 块作为表达式 + 最后一条语句贡献块值 → 值写入 result_reg
            if let (Some(reg), true) = (result_reg.clone(), is_last) {
                if self.generate_tail_stmt_ir(stmt, reg, instructions, constants)? {
                    // 尾语句路径同款消费（块值已进 reg，Drop 排在块出口之前）
                    self.emit_release_drops(stmt.plan_id, instructions);
                    self.set_cur_span(stmt.span);
                    continue;
                }
            }
            // 其他情况正常生成语句
            self.generate_local_stmt_ir(stmt, instructions, constants)?;
            // NLL Release: 块内每条语句边界都消费释放计划（RFC-039 D20）——
            // 嵌套块（if/for/while/块表达式的体）的语句也各有自己的条目，
            // 作用域退出的 Drop 就归属在块的最后一条语句上
            self.emit_release_drops(stmt.plan_id, instructions);
            // #393：语句级临时寄存器回收——临时不跨语句存活，语句生成完
            // 回滚到「具名局部水位 ⋈ 循环态地板」。具名水位取 cur_locals
            // 最后一个具名槽（参数与全部具名绑定，含已退出作用域者，保守
            // 不复用）；temp_floor 保住 for 迭代器等循环态临时。仅语句语境
            // 块回收：result_reg=Some 的块处于表达式操作数位，外层可能持有
            // 跨块存活的兄弟实参临时，回滚会踩坏它们。
            if result_reg.is_none() {
                let named_watermark = self
                    .cur_locals
                    .iter()
                    .rposition(|slot| slot.name.is_some())
                    .map_or(0, |i| i + 1);
                self.next_temp = named_watermark.max(self.temp_floor);
            }
            self.set_cur_span(stmt.span);
        }

        // 退出作用域
        self.exit_scope();

        Ok(())
    }
}
