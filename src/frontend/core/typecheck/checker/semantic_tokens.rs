impl TypeChecker {
    fn semantic_token_type_for_export(
        export: &crate::frontend::module::Export
    ) -> semantic_db::SemanticTokenType {
        match export.kind {
            crate::frontend::module::ExportKind::Function => {
                semantic_db::SemanticTokenType::Function
            }
            crate::frontend::module::ExportKind::SubModule => {
                semantic_db::SemanticTokenType::Namespace
            }
            crate::frontend::module::ExportKind::Type => semantic_db::SemanticTokenType::Type,
            crate::frontend::module::ExportKind::Constant => {
                semantic_db::SemanticTokenType::Variable
            }
        }
    }

    fn collect_use_stmt_tokens(
        &mut self,
        file_path: &str,
        path: &str,
        path_parts: &[crate::frontend::core::parser::ast::SpannedIdent],
        items: &Option<Vec<String>>,
        alias: &Option<Vec<String>>,
    ) {
        // use path.{...} 的 path 部分始终是模块命名空间
        if items.is_some() {
            for part in path_parts {
                self.semantic_db.add_token(
                    file_path,
                    semantic_db::SemanticToken {
                        name: part.name.clone(),
                        token_type: semantic_db::SemanticTokenType::Namespace,
                        modifiers: vec![],
                        span: part.span,
                    },
                );
            }
            return;
        }

        // use path as alias：path 是命名空间，alias 是目标符号名（由别名决定）
        if alias.is_some() {
            for part in path_parts {
                self.semantic_db.add_token(
                    file_path,
                    semantic_db::SemanticToken {
                        name: part.name.clone(),
                        token_type: semantic_db::SemanticTokenType::Namespace,
                        modifiers: vec![],
                        span: part.span,
                    },
                );
            }
            return;
        }

        if self.env.module_registry.has_module(path) {
            for part in path_parts {
                self.semantic_db.add_token(
                    file_path,
                    semantic_db::SemanticToken {
                        name: part.name.clone(),
                        token_type: semantic_db::SemanticTokenType::Namespace,
                        modifiers: vec![],
                        span: part.span,
                    },
                );
            }
            return;
        }

        if let Ok(export) = self.env.module_registry.resolve_export(path) {
            for (idx, part) in path_parts.iter().enumerate() {
                let token_type = if idx + 1 == path_parts.len() {
                    Self::semantic_token_type_for_export(export)
                } else {
                    semantic_db::SemanticTokenType::Namespace
                };
                self.semantic_db.add_token(
                    file_path,
                    semantic_db::SemanticToken {
                        name: part.name.clone(),
                        token_type,
                        modifiers: vec![],
                        span: part.span,
                    },
                );
            }
            return;
        }

        // fallback
        for part in path_parts {
            self.semantic_db.add_token(
                file_path,
                semantic_db::SemanticToken {
                    name: part.name.clone(),
                    token_type: semantic_db::SemanticTokenType::Namespace,
                    modifiers: vec![],
                    span: part.span,
                },
            );
        }
    }

    fn collect_type_tokens(
        &mut self,
        file_path: &str,
        ty: &crate::frontend::core::parser::ast::Type,
    ) {
        use crate::frontend::core::parser::ast::Type;

        match ty {
            Type::Name { name, span } => {
                self.semantic_db.add_token(
                    file_path,
                    semantic_db::SemanticToken {
                        name: name.clone(),
                        token_type: semantic_db::SemanticTokenType::Type,
                        modifiers: vec![],
                        span: *span,
                    },
                );
            }
            Type::Generic {
                name,
                name_span,
                args,
            } => {
                self.semantic_db.add_token(
                    file_path,
                    semantic_db::SemanticToken {
                        name: name.clone(),
                        token_type: semantic_db::SemanticTokenType::Type,
                        modifiers: vec![],
                        span: *name_span,
                    },
                );
                for arg in args {
                    self.collect_type_tokens(file_path, arg);
                }
            }
            Type::NamedStruct {
                name,
                name_span,
                fields,
            } => {
                self.semantic_db.add_token(
                    file_path,
                    semantic_db::SemanticToken {
                        name: name.clone(),
                        token_type: semantic_db::SemanticTokenType::Type,
                        modifiers: vec![],
                        span: *name_span,
                    },
                );
                for f in fields {
                    self.collect_type_tokens(file_path, &f.ty);
                }
            }
            Type::Struct {
                fields, bindings, ..
            } => {
                for f in fields {
                    self.collect_type_tokens(file_path, &f.ty);
                }
                for b in bindings {
                    match &b.kind {
                        crate::frontend::core::parser::ast::BindingKind::Anonymous {
                            params,
                            return_type,
                            ..
                        } => {
                            for p in params {
                                if let Some(t) = &p.ty {
                                    self.collect_type_tokens(file_path, t);
                                }
                            }
                            self.collect_type_tokens(file_path, return_type);
                        }
                        crate::frontend::core::parser::ast::BindingKind::External { .. }
                        | crate::frontend::core::parser::ast::BindingKind::DefaultExternal {
                            ..
                        } => {}
                    }
                }
            }
            Type::Union(variants) => {
                for (_name, maybe_ty) in variants {
                    if let Some(t) = maybe_ty {
                        self.collect_type_tokens(file_path, t);
                    }
                }
            }
            Type::Variant(variants) => {
                for v in variants {
                    for (_param_name, t) in &v.params {
                        self.collect_type_tokens(file_path, t);
                    }
                }
            }
            Type::Tuple(types) => {
                for t in types {
                    self.collect_type_tokens(file_path, t);
                }
            }
            Type::Fn {
                params,
                return_type,
            } => {
                for t in params {
                    self.collect_type_tokens(file_path, t);
                }
                self.collect_type_tokens(file_path, return_type);
            }
            Type::Option(inner) => self.collect_type_tokens(file_path, inner),
            Type::Result(ok, err) => {
                self.collect_type_tokens(file_path, ok);
                self.collect_type_tokens(file_path, err);
            }
            Type::AssocType {
                host_type,
                assoc_name,
                assoc_name_span,
                assoc_args,
            } => {
                self.collect_type_tokens(file_path, host_type);
                self.semantic_db.add_token(
                    file_path,
                    semantic_db::SemanticToken {
                        name: assoc_name.clone(),
                        token_type: semantic_db::SemanticTokenType::Type,
                        modifiers: vec![],
                        span: *assoc_name_span,
                    },
                );
                for t in assoc_args {
                    self.collect_type_tokens(file_path, t);
                }
            }
            Type::Sum(types) => {
                for t in types {
                    self.collect_type_tokens(file_path, t);
                }
            }
            Type::Literal {
                name,
                name_span,
                base_type,
            } => {
                self.semantic_db.add_token(
                    file_path,
                    semantic_db::SemanticToken {
                        name: name.clone(),
                        token_type: semantic_db::SemanticTokenType::Type,
                        modifiers: vec![],
                        span: *name_span,
                    },
                );
                self.collect_type_tokens(file_path, base_type);
            }
            Type::Ptr(inner) => self.collect_type_tokens(file_path, inner),
            Type::MetaType { name_span, args } => {
                self.semantic_db.add_token(
                    file_path,
                    semantic_db::SemanticToken {
                        name: "Type".to_string(),
                        token_type: semantic_db::SemanticTokenType::Type,
                        modifiers: vec![],
                        span: *name_span,
                    },
                );
                for t in args {
                    self.collect_type_tokens(file_path, t);
                }
            }
            Type::Int(_)
            | Type::Float(_)
            | Type::Char
            | Type::String
            | Type::Bytes
            | Type::Bool
            | Type::Void
            | Type::Enum(_)
            | Type::ConstExpr(_) => {}
            Type::Ref { inner, .. } => self.collect_type_tokens(file_path, inner),
        }
    }

    fn is_struct_binding(
        &self,
        name: &str,
    ) -> bool {
        self.env
            .get_var(name)
            .is_some_and(|poly| matches!(poly.body, MonoType::Struct(_)))
    }

    #[allow(clippy::too_many_arguments)]
    fn collect_call_target_tokens(
        &mut self,
        file_path: &str,
        expr: &Expr,
        scope_idx: usize,
        declared: &mut HashMap<usize, HashSet<String>>,
        constructor_names: &HashSet<String>,
        imported_module_roots: &mut HashSet<String>,
        is_terminal: bool,
    ) {
        use semantic_db::SemanticTokenType;

        match expr {
            Expr::FieldAccess {
                expr: inner,
                field,
                span,
            } => {
                self.collect_call_target_tokens(
                    file_path,
                    inner,
                    scope_idx,
                    declared,
                    constructor_names,
                    imported_module_roots,
                    false,
                );

                let is_module_path = Self::is_module_path_expr(inner, imported_module_roots);
                let token_type = if is_terminal {
                    if is_module_path {
                        SemanticTokenType::Function
                    } else {
                        SemanticTokenType::Method
                    }
                } else if is_module_path {
                    SemanticTokenType::Namespace
                } else {
                    SemanticTokenType::Property
                };

                self.semantic_db.add_token(
                    file_path,
                    semantic_db::SemanticToken {
                        name: field.clone(),
                        token_type,
                        modifiers: vec![],
                        span: *span,
                    },
                );
            }
            Expr::Var(name, span) => {
                let token_type = if imported_module_roots.contains(name) {
                    SemanticTokenType::Namespace
                } else if constructor_names.contains(name) {
                    SemanticTokenType::EnumMember
                } else if self.is_struct_binding(name) {
                    SemanticTokenType::Type
                } else if is_terminal {
                    SemanticTokenType::Function
                } else if let Some(poly) = self.env.get_var(name) {
                    if matches!(poly.body, MonoType::Fn { .. }) {
                        SemanticTokenType::Function
                    } else {
                        SemanticTokenType::Variable
                    }
                } else {
                    SemanticTokenType::Variable
                };

                self.semantic_db.add_token(
                    file_path,
                    semantic_db::SemanticToken {
                        name: name.clone(),
                        token_type,
                        modifiers: vec![],
                        span: *span,
                    },
                );
            }
            _ => {
                self.collect_expr_tokens(
                    file_path,
                    expr,
                    scope_idx,
                    declared,
                    constructor_names,
                    imported_module_roots,
                );
            }
        }
    }

    fn is_module_path_expr(
        expr: &Expr,
        imported_module_roots: &HashSet<String>,
    ) -> bool {
        match expr {
            Expr::Var(name, _) => imported_module_roots.contains(name),
            Expr::FieldAccess { expr: inner, .. } => {
                Self::is_module_path_expr(inner, imported_module_roots)
            }
            _ => false,
        }
    }

    fn collect_semantic_tokens(
        &mut self,
        module: &Module,
    ) {
        use crate::frontend::core::parser::ast::StmtKind;
        use semantic_db::{
            SemanticToken, SemanticTokenType, SemanticTokenModifier, ScopeInfo, ScopeKind,
        };

        let fp = self.env.module_name.clone();

        let mut declared: HashMap<usize, HashSet<String>> = HashMap::new();
        declared.insert(0, HashSet::new());
        let constructor_names = Self::constructor_names_from_module(module);
        let mut imported_module_roots = HashSet::new();
        imported_module_roots.insert("std".to_string());

        // 添加全局作用域
        self.semantic_db.add_scope(
            &fp,
            ScopeInfo {
                span: module.span,
                parent: None,
                symbols: Vec::new(),
                kind: ScopeKind::Global,
            },
        );

        let mut global_symbols = Vec::new();

        for stmt in &module.items {
            match &stmt.kind {
                StmtKind::Binding {
                    name,
                    type_name,
                    method_type,
                    generic_params,
                    type_annotation,
                    params,
                    body,
                    is_pub,
                    ..
                } => {
                    // 根据字段值区分类型：方法绑定 / 类型定义 / 函数
                    let is_method = type_name.is_some();
                    // 函数定义：body 有语句
                    let has_body = !body.is_empty();
                    // 类型定义：没有 body 且有 type_annotation
                    let is_type_def = !has_body && type_annotation.is_some();

                    if is_method {
                        // 方法绑定 → Method (定义)
                        self.semantic_db.add_token(
                            &fp,
                            SemanticToken {
                                name: name.clone(),
                                token_type: SemanticTokenType::Method,
                                modifiers: vec![SemanticTokenModifier::Declaration],
                                span: stmt.span,
                            },
                        );

                        // 参数 → Parameter (定义)
                        for param in params {
                            self.semantic_db.add_token(
                                &fp,
                                SemanticToken {
                                    name: param.name.clone(),
                                    token_type: SemanticTokenType::Parameter,
                                    modifiers: vec![SemanticTokenModifier::Declaration],
                                    span: param.span,
                                },
                            );
                        }

                        if let Some(mt) = method_type {
                            self.collect_type_tokens(&fp, mt);
                        }
                    } else if is_type_def {
                        // 类型定义 → Type (定义)
                        let mut modifiers = vec![SemanticTokenModifier::Declaration];
                        if !generic_params.is_empty() {
                            modifiers.push(SemanticTokenModifier::Generic);
                        }
                        self.semantic_db.add_token(
                            &fp,
                            SemanticToken {
                                name: name.clone(),
                                token_type: SemanticTokenType::Type,
                                modifiers,
                                span: stmt.span,
                            },
                        );
                        global_symbols.push(name.clone());

                        // 类型定义体中的类型引用
                        if let Some(def) = type_annotation {
                            self.collect_type_tokens(&fp, def);

                            // Variant constructors → EnumMember (定义)
                            if let crate::frontend::core::parser::ast::Type::Variant(variants) = def
                            {
                                for v in variants {
                                    self.semantic_db.add_token(
                                        &fp,
                                        SemanticToken {
                                            name: v.name.clone(),
                                            token_type: SemanticTokenType::EnumMember,
                                            modifiers: vec![SemanticTokenModifier::Declaration],
                                            span: v.name_span,
                                        },
                                    );
                                }
                            }
                        }
                    } else {
                        // 函数定义 → Function (定义)
                        let mut modifiers = vec![SemanticTokenModifier::Declaration];
                        if *is_pub {
                            modifiers.push(SemanticTokenModifier::Public);
                        }
                        if !generic_params.is_empty() {
                            modifiers.push(SemanticTokenModifier::Generic);
                        }
                        self.semantic_db.add_token(
                            &fp,
                            SemanticToken {
                                name: name.clone(),
                                token_type: SemanticTokenType::Function,
                                modifiers,
                                span: stmt.span,
                            },
                        );
                        global_symbols.push(name.clone());

                        // 参数 → Parameter (定义)
                        for param in params {
                            self.semantic_db.add_token(
                                &fp,
                                SemanticToken {
                                    name: param.name.clone(),
                                    token_type: SemanticTokenType::Parameter,
                                    modifiers: vec![SemanticTokenModifier::Declaration],
                                    span: param.span,
                                },
                            );
                        }

                        // 泛型参数 → TypeParameter (定义)
                        for gp in generic_params {
                            let gp_name = match &gp.kind {
                                crate::frontend::core::parser::ast::GenericParamKind::Type => {
                                    gp.name.clone()
                                }
                                _ => gp.name.clone(),
                            };
                            self.semantic_db.add_token(
                                &fp,
                                SemanticToken {
                                    name: gp_name,
                                    token_type: SemanticTokenType::TypeParameter,
                                    modifiers: vec![SemanticTokenModifier::Declaration],
                                    span: stmt.span,
                                },
                            );
                        }

                        // 泛型约束中的类型引用
                        for gp in generic_params {
                            for c in &gp.constraints {
                                self.collect_type_tokens(&fp, c);
                            }
                        }

                        // 函数签名中的类型引用
                        if let Some(ty) = type_annotation {
                            self.collect_type_tokens(&fp, ty);
                        }

                        let scope_idx = self
                            .semantic_db
                            .get_scopes(&fp)
                            .map(|s| s.len())
                            .unwrap_or(0);
                        declared.insert(scope_idx, params.iter().map(|p| p.name.clone()).collect());
                        self.semantic_db.add_scope(
                            &fp,
                            ScopeInfo {
                                span: stmt.span,
                                parent: Some(0),
                                symbols: params.iter().map(|p| p.name.clone()).collect(),
                                kind: ScopeKind::Function,
                            },
                        );

                        // 递归收集函数体中的表达式
                        let mut fn_roots = imported_module_roots.clone();
                        for body_stmt in body {
                            self.collect_stmt_tokens(
                                &fp,
                                body_stmt,
                                scope_idx,
                                &mut declared,
                                &constructor_names,
                                &mut fn_roots,
                            );
                        }
                    }
                }
                StmtKind::Var {
                    name,
                    name_span,
                    type_annotation,
                    initializer,
                    ..
                } => {
                    // 变量名 → Variable (定义)
                    let is_declaration = declared.entry(0).or_default().insert(name.clone());
                    let modifiers = if is_declaration {
                        vec![SemanticTokenModifier::Declaration]
                    } else {
                        vec![SemanticTokenModifier::Mutable]
                    };
                    self.semantic_db.add_token(
                        &fp,
                        SemanticToken {
                            name: name.clone(),
                            token_type: SemanticTokenType::Variable,
                            modifiers,
                            span: *name_span,
                        },
                    );
                    if is_declaration {
                        global_symbols.push(name.clone());
                    }

                    if let Some(ty) = type_annotation {
                        self.collect_type_tokens(&fp, ty);
                    }

                    if let Some(init) = initializer {
                        self.collect_expr_tokens(
                            &fp,
                            init,
                            0,
                            &mut declared,
                            &constructor_names,
                            &mut imported_module_roots,
                        );
                    }
                }
                StmtKind::Use {
                    path,
                    path_parts,
                    items,
                    alias,
                    ..
                } => {
                    self.collect_use_stmt_tokens(&fp, path, path_parts, items, alias);
                    self.add_use_module_root(&mut imported_module_roots, path, items, alias);
                }
                StmtKind::Expr(expr) => {
                    self.collect_expr_tokens(
                        &fp,
                        expr,
                        0,
                        &mut declared,
                        &constructor_names,
                        &mut imported_module_roots,
                    );
                }
                StmtKind::For {
                    var,
                    var_span,
                    iterable,
                    body,
                    ..
                } => {
                    declared.entry(0).or_default().insert(var.clone());
                    self.semantic_db.add_token(
                        &fp,
                        SemanticToken {
                            name: var.clone(),
                            token_type: SemanticTokenType::Variable,
                            modifiers: vec![SemanticTokenModifier::Declaration],
                            span: *var_span,
                        },
                    );
                    self.collect_expr_tokens(
                        &fp,
                        iterable,
                        0,
                        &mut declared,
                        &constructor_names,
                        &mut imported_module_roots,
                    );
                    for body_stmt in &body.stmts {
                        self.collect_stmt_tokens(
                            &fp,
                            body_stmt,
                            0,
                            &mut declared,
                            &constructor_names,
                            &mut imported_module_roots,
                        );
                    }
                }
                StmtKind::DestructureAssign { names, rhs, .. } => {
                    for name in names {
                        let is_declaration = declared.entry(0).or_default().insert(name.name.clone());
                        let modifiers = if is_declaration {
                            vec![SemanticTokenModifier::Declaration]
                        } else {
                            vec![SemanticTokenModifier::Mutable]
                        };
                        self.semantic_db.add_token(
                            &fp,
                            SemanticToken {
                                name: name.name.clone(),
                                token_type: SemanticTokenType::Variable,
                                modifiers,
                                span: name.span,
                            },
                        );
                        if is_declaration {
                            global_symbols.push(name.name.clone());
                        }
                    }
                    self.collect_expr_tokens(
                        &fp,
                        rhs,
                        0,
                        &mut declared,
                        &constructor_names,
                        &mut imported_module_roots,
                    );
                }
                StmtKind::If { .. } | StmtKind::Error(_) | StmtKind::ExternalBindingStmt { .. } => {
                }
                StmtKind::Return(expr_opt) => {
                    if let Some(expr) = expr_opt {
                        self.collect_expr_tokens(
                            &fp,
                            expr,
                            0,
                            &mut declared,
                            &constructor_names,
                            &mut imported_module_roots,
                        );
                    }
                }
            }
        }

        // 更新全局作用域的符号列表
        if let Some(file_info) = self.semantic_db.get_file_info(&self.env.module_name) {
            if !file_info.scopes.is_empty() {
                // We need mutable access; use set_file_info approach or direct access
                // For simplicity, we recorded symbols inline already
            }
        }
    }

    /// 收集语句中的语义 tokens（递归）
    fn collect_stmt_tokens(
        &mut self,
        file_path: &str,
        stmt: &crate::frontend::core::parser::ast::Stmt,
        scope_idx: usize,
        declared: &mut HashMap<usize, HashSet<String>>,
        constructor_names: &HashSet<String>,
        imported_module_roots: &mut HashSet<String>,
    ) {
        use crate::frontend::core::parser::ast::StmtKind;
        use semantic_db::SemanticTokenModifier;

        match &stmt.kind {
            StmtKind::Var {
                name,
                name_span,
                type_annotation,
                initializer,
                ..
            } => {
                let is_declaration = declared.entry(scope_idx).or_default().insert(name.clone());
                let modifiers = if is_declaration {
                    vec![SemanticTokenModifier::Declaration]
                } else {
                    vec![SemanticTokenModifier::Mutable]
                };
                self.semantic_db.add_token(
                    file_path,
                    semantic_db::SemanticToken {
                        name: name.clone(),
                        token_type: semantic_db::SemanticTokenType::LocalVariable,
                        modifiers,
                        span: *name_span,
                    },
                );
                if let Some(ty) = type_annotation {
                    self.collect_type_tokens(file_path, ty);
                }
                if let Some(init) = initializer {
                    self.collect_expr_tokens(
                        file_path,
                        init,
                        scope_idx,
                        declared,
                        constructor_names,
                        imported_module_roots,
                    );
                }
            }
            StmtKind::Expr(expr) => {
                self.collect_expr_tokens(
                    file_path,
                    expr,
                    scope_idx,
                    declared,
                    constructor_names,
                    imported_module_roots,
                );
            }
            StmtKind::Use {
                path,
                path_parts,
                items,
                alias,
                ..
            } => {
                self.collect_use_stmt_tokens(file_path, path, path_parts, items, alias);
                self.add_use_module_root(imported_module_roots, path, items, alias);
            }
            StmtKind::For {
                var,
                var_span,
                iterable,
                body,
                ..
            } => {
                declared.entry(scope_idx).or_default().insert(var.clone());
                self.semantic_db.add_token(
                    file_path,
                    semantic_db::SemanticToken {
                        name: var.clone(),
                        token_type: semantic_db::SemanticTokenType::Variable,
                        modifiers: vec![semantic_db::SemanticTokenModifier::Declaration],
                        span: *var_span,
                    },
                );
                self.collect_expr_tokens(
                    file_path,
                    iterable,
                    scope_idx,
                    declared,
                    constructor_names,
                    imported_module_roots,
                );
                for body_stmt in &body.stmts {
                    self.collect_stmt_tokens(
                        file_path,
                        body_stmt,
                        scope_idx,
                        declared,
                        constructor_names,
                        imported_module_roots,
                    );
                }
            }
            StmtKind::If {
                condition,
                then_branch,
                elif_branches,
                else_branch,
                ..
            } => {
                self.collect_expr_tokens(
                    file_path,
                    condition,
                    scope_idx,
                    declared,
                    constructor_names,
                    imported_module_roots,
                );

                let mut then_roots = imported_module_roots.clone();
                for s in &then_branch.stmts {
                    self.collect_stmt_tokens(
                        file_path,
                        s,
                        scope_idx,
                        declared,
                        constructor_names,
                        &mut then_roots,
                    );
                }

                for (elif_cond, elif_block) in elif_branches {
                    self.collect_expr_tokens(
                        file_path,
                        elif_cond,
                        scope_idx,
                        declared,
                        constructor_names,
                        imported_module_roots,
                    );

                    let mut elif_roots = imported_module_roots.clone();
                    for s in &elif_block.stmts {
                        self.collect_stmt_tokens(
                            file_path,
                            s,
                            scope_idx,
                            declared,
                            constructor_names,
                            &mut elif_roots,
                        );
                    }
                }

                if let Some(else_block) = else_branch {
                    let mut else_roots = imported_module_roots.clone();
                    for s in &else_block.stmts {
                        self.collect_stmt_tokens(
                            file_path,
                            s,
                            scope_idx,
                            declared,
                            constructor_names,
                            &mut else_roots,
                        );
                    }
                }
            }
            StmtKind::Binding {
                name,
                params,
                generic_params,
                type_annotation,
                body,
                ..
            } => {
                // 嵌套函数
                self.semantic_db.add_token(
                    file_path,
                    semantic_db::SemanticToken {
                        name: name.clone(),
                        token_type: semantic_db::SemanticTokenType::Function,
                        modifiers: vec![semantic_db::SemanticTokenModifier::Declaration],
                        span: stmt.span,
                    },
                );
                for param in params {
                    self.semantic_db.add_token(
                        file_path,
                        semantic_db::SemanticToken {
                            name: param.name.clone(),
                            token_type: semantic_db::SemanticTokenType::Parameter,
                            modifiers: vec![semantic_db::SemanticTokenModifier::Declaration],
                            span: param.span,
                        },
                    );
                }
                for gp in generic_params {
                    for c in &gp.constraints {
                        self.collect_type_tokens(file_path, c);
                    }
                }
                if let Some(ty) = type_annotation {
                    self.collect_type_tokens(file_path, ty);
                }
                let mut fn_roots = imported_module_roots.clone();
                for body_stmt in body {
                    self.collect_stmt_tokens(
                        file_path,
                        body_stmt,
                        scope_idx,
                        declared,
                        constructor_names,
                        &mut fn_roots,
                    );
                }
            }
            StmtKind::DestructureAssign { names, rhs, .. } => {
                for name in names {
                    let is_declaration = declared.entry(scope_idx).or_default().insert(name.name.clone());
                    let modifiers = if is_declaration {
                        vec![SemanticTokenModifier::Declaration]
                    } else {
                        vec![SemanticTokenModifier::Mutable]
                    };
                    self.semantic_db.add_token(
                        file_path,
                        semantic_db::SemanticToken {
                            name: name.name.clone(),
                            token_type: semantic_db::SemanticTokenType::LocalVariable,
                            modifiers,
                            span: name.span,
                        },
                    );
                }
                self.collect_expr_tokens(
                    file_path,
                    rhs,
                    scope_idx,
                    declared,
                    constructor_names,
                    imported_module_roots,
                );
            }
            _ => {}
        }
    }

    /// 收集表达式中的语义 tokens（递归）
    fn collect_expr_tokens(
        &mut self,
        file_path: &str,
        expr: &Expr,
        scope_idx: usize,
        declared: &mut HashMap<usize, HashSet<String>>,
        constructor_names: &HashSet<String>,
        imported_module_roots: &mut HashSet<String>,
    ) {
        use crate::frontend::core::parser::ast::Expr;

        match expr {
            Expr::Var(name, span) => {
                // 判断是函数引用还是变量引用
                let token_type = if imported_module_roots.contains(name) {
                    semantic_db::SemanticTokenType::Namespace
                } else if constructor_names.contains(name) {
                    semantic_db::SemanticTokenType::EnumMember
                } else if let Some(poly) = self.env.get_var(name) {
                    if matches!(poly.body, MonoType::Fn { .. }) {
                        semantic_db::SemanticTokenType::Function
                    } else {
                        semantic_db::SemanticTokenType::Variable
                    }
                } else {
                    semantic_db::SemanticTokenType::Variable
                };
                self.semantic_db.add_token(
                    file_path,
                    semantic_db::SemanticToken {
                        name: name.clone(),
                        token_type,
                        modifiers: vec![],
                        span: *span,
                    },
                );
            }
            Expr::Call { func, args, .. } => {
                self.collect_call_target_tokens(
                    file_path,
                    func,
                    scope_idx,
                    declared,
                    constructor_names,
                    imported_module_roots,
                    true,
                );
                for arg in args {
                    self.collect_expr_tokens(
                        file_path,
                        arg,
                        scope_idx,
                        declared,
                        constructor_names,
                        imported_module_roots,
                    );
                }
            }
            Expr::FieldAccess {
                expr: inner,
                field,
                span,
            } => {
                let is_module_path = Self::is_module_path_expr(inner, imported_module_roots);
                self.collect_expr_tokens(
                    file_path,
                    inner,
                    scope_idx,
                    declared,
                    constructor_names,
                    imported_module_roots,
                );
                self.semantic_db.add_token(
                    file_path,
                    semantic_db::SemanticToken {
                        name: field.clone(),
                        token_type: if is_module_path {
                            semantic_db::SemanticTokenType::Namespace
                        } else {
                            semantic_db::SemanticTokenType::Property
                        },
                        modifiers: vec![],
                        span: *span,
                    },
                );
            }
            Expr::Cast {
                expr: inner,
                target_type,
                span: _,
            } => {
                self.collect_expr_tokens(
                    file_path,
                    inner,
                    scope_idx,
                    declared,
                    constructor_names,
                    imported_module_roots,
                );
                // Cast 目标类型 → Type (引用)
                self.collect_type_tokens(file_path, target_type);
            }
            Expr::BinOp { left, right, .. } => {
                self.collect_expr_tokens(
                    file_path,
                    left,
                    scope_idx,
                    declared,
                    constructor_names,
                    imported_module_roots,
                );
                self.collect_expr_tokens(
                    file_path,
                    right,
                    scope_idx,
                    declared,
                    constructor_names,
                    imported_module_roots,
                );
            }
            Expr::UnOp { expr: inner, .. } => {
                self.collect_expr_tokens(
                    file_path,
                    inner,
                    scope_idx,
                    declared,
                    constructor_names,
                    imported_module_roots,
                );
            }
            Expr::If {
                condition,
                then_branch,
                elif_branches,
                else_branch,
                ..
            } => {
                self.collect_expr_tokens(
                    file_path,
                    condition,
                    scope_idx,
                    declared,
                    constructor_names,
                    imported_module_roots,
                );
                let mut then_roots = imported_module_roots.clone();
                for s in &then_branch.stmts {
                    self.collect_stmt_tokens(
                        file_path,
                        s,
                        scope_idx,
                        declared,
                        constructor_names,
                        &mut then_roots,
                    );
                }
                for (cond, block) in elif_branches {
                    self.collect_expr_tokens(
                        file_path,
                        cond,
                        scope_idx,
                        declared,
                        constructor_names,
                        imported_module_roots,
                    );
                    let mut elif_roots = imported_module_roots.clone();
                    for s in &block.stmts {
                        self.collect_stmt_tokens(
                            file_path,
                            s,
                            scope_idx,
                            declared,
                            constructor_names,
                            &mut elif_roots,
                        );
                    }
                }
                if let Some(block) = else_branch {
                    let mut else_roots = imported_module_roots.clone();
                    for s in &block.stmts {
                        self.collect_stmt_tokens(
                            file_path,
                            s,
                            scope_idx,
                            declared,
                            constructor_names,
                            &mut else_roots,
                        );
                    }
                }
                // 后续还有其他类似的块...
            }
            Expr::While {
                condition, body, ..
            } => {
                self.collect_expr_tokens(
                    file_path,
                    condition,
                    scope_idx,
                    declared,
                    constructor_names,
                    imported_module_roots,
                );
                let mut while_roots = imported_module_roots.clone();
                for s in &body.stmts {
                    self.collect_stmt_tokens(
                        file_path,
                        s,
                        scope_idx,
                        declared,
                        constructor_names,
                        &mut while_roots,
                    );
                }
            }
            Expr::For {
                iterable,
                body,
                var,
                span,
                ..
            } => {
                declared.entry(scope_idx).or_default().insert(var.clone());
                self.semantic_db.add_token(
                    file_path,
                    semantic_db::SemanticToken {
                        name: var.clone(),
                        token_type: semantic_db::SemanticTokenType::Variable,
                        modifiers: vec![semantic_db::SemanticTokenModifier::Declaration],
                        span: *span,
                    },
                );
                self.collect_expr_tokens(
                    file_path,
                    iterable,
                    scope_idx,
                    declared,
                    constructor_names,
                    imported_module_roots,
                );
                let mut for_roots = imported_module_roots.clone();
                for s in &body.stmts {
                    self.collect_stmt_tokens(
                        file_path,
                        s,
                        scope_idx,
                        declared,
                        constructor_names,
                        &mut for_roots,
                    );
                }
            }
            Expr::Lambda { params, body, span } => {
                // Lambda 参数
                for param in params {
                    self.semantic_db.add_token(
                        file_path,
                        semantic_db::SemanticToken {
                            name: param.name.clone(),
                            token_type: semantic_db::SemanticTokenType::Parameter,
                            modifiers: vec![semantic_db::SemanticTokenModifier::Declaration],
                            span: param.span,
                        },
                    );
                }
                // Lambda 作用域
                let lambda_scope_idx = self
                    .semantic_db
                    .get_scopes(file_path)
                    .map(|s| s.len())
                    .unwrap_or(0);
                declared.insert(
                    lambda_scope_idx,
                    params.iter().map(|p| p.name.clone()).collect(),
                );
                self.semantic_db.add_scope(
                    file_path,
                    semantic_db::ScopeInfo {
                        span: *span,
                        parent: None, // 简化：不追踪精确父级
                        symbols: params.iter().map(|p| p.name.clone()).collect(),
                        kind: semantic_db::ScopeKind::Lambda,
                    },
                );
                let mut lambda_roots = imported_module_roots.clone();
                for s in &body.stmts {
                    self.collect_stmt_tokens(
                        file_path,
                        s,
                        lambda_scope_idx,
                        declared,
                        constructor_names,
                        &mut lambda_roots,
                    );
                }
            }
            Expr::Block(block) => {
                let mut block_roots = imported_module_roots.clone();
                for s in &block.stmts {
                    self.collect_stmt_tokens(
                        file_path,
                        s,
                        scope_idx,
                        declared,
                        constructor_names,
                        &mut block_roots,
                    );
                }
            }
            Expr::Return(Some(inner), _) => {
                self.collect_expr_tokens(
                    file_path,
                    inner,
                    scope_idx,
                    declared,
                    constructor_names,
                    imported_module_roots,
                );
            }
            Expr::Match {
                expr: inner, arms, ..
            } => {
                self.collect_expr_tokens(
                    file_path,
                    inner,
                    scope_idx,
                    declared,
                    constructor_names,
                    imported_module_roots,
                );
                for arm in arms {
                    let mut arm_roots = imported_module_roots.clone();
                    for s in &arm.body.stmts {
                        self.collect_stmt_tokens(
                            file_path,
                            s,
                            scope_idx,
                            declared,
                            constructor_names,
                            &mut arm_roots,
                        );
                    }
                }
            }
            Expr::Tuple(elements, _) | Expr::List(elements, _) => {
                for elem in elements {
                    self.collect_expr_tokens(
                        file_path,
                        elem,
                        scope_idx,
                        declared,
                        constructor_names,
                        imported_module_roots,
                    );
                }
            }
            Expr::Dict(pairs, _) => {
                for (k, v) in pairs {
                    self.collect_expr_tokens(
                        file_path,
                        k,
                        scope_idx,
                        declared,
                        constructor_names,
                        imported_module_roots,
                    );
                    self.collect_expr_tokens(
                        file_path,
                        v,
                        scope_idx,
                        declared,
                        constructor_names,
                        imported_module_roots,
                    );
                }
            }
            Expr::Index {
                expr: inner, index, ..
            } => {
                self.collect_expr_tokens(
                    file_path,
                    inner,
                    scope_idx,
                    declared,
                    constructor_names,
                    imported_module_roots,
                );
                self.collect_expr_tokens(
                    file_path,
                    index,
                    scope_idx,
                    declared,
                    constructor_names,
                    imported_module_roots,
                );
            }
            Expr::Try { expr: inner, .. } | Expr::Ref { expr: inner, .. } => {
                self.collect_expr_tokens(
                    file_path,
                    inner,
                    scope_idx,
                    declared,
                    constructor_names,
                    imported_module_roots,
                );
            }
            Expr::FnDef {
                name,
                params,
                return_type,
                body,
                span,
                ..
            } => {
                self.semantic_db.add_token(
                    file_path,
                    semantic_db::SemanticToken {
                        name: name.clone(),
                        token_type: semantic_db::SemanticTokenType::Function,
                        modifiers: vec![semantic_db::SemanticTokenModifier::Declaration],
                        span: *span,
                    },
                );
                for param in params {
                    self.semantic_db.add_token(
                        file_path,
                        semantic_db::SemanticToken {
                            name: param.name.clone(),
                            token_type: semantic_db::SemanticTokenType::Parameter,
                            modifiers: vec![semantic_db::SemanticTokenModifier::Declaration],
                            span: param.span,
                        },
                    );
                    if let Some(t) = &param.ty {
                        self.collect_type_tokens(file_path, t);
                    }
                }
                if let Some(t) = return_type {
                    self.collect_type_tokens(file_path, t);
                }
                for s in &body.stmts {
                    self.collect_stmt_tokens(
                        file_path,
                        s,
                        scope_idx,
                        declared,
                        constructor_names,
                        imported_module_roots,
                    );
                }
            }
            Expr::ListComp {
                element,
                iterable,
                condition,
                var,
                span,
                ..
            } => {
                declared.entry(scope_idx).or_default().insert(var.clone());
                self.semantic_db.add_token(
                    file_path,
                    semantic_db::SemanticToken {
                        name: var.clone(),
                        token_type: semantic_db::SemanticTokenType::Variable,
                        modifiers: vec![semantic_db::SemanticTokenModifier::Declaration],
                        span: *span,
                    },
                );
                self.collect_expr_tokens(
                    file_path,
                    element,
                    scope_idx,
                    declared,
                    constructor_names,
                    imported_module_roots,
                );
                self.collect_expr_tokens(
                    file_path,
                    iterable,
                    scope_idx,
                    declared,
                    constructor_names,
                    imported_module_roots,
                );
                if let Some(cond) = condition {
                    self.collect_expr_tokens(
                        file_path,
                        cond,
                        scope_idx,
                        declared,
                        constructor_names,
                        imported_module_roots,
                    );
                }
            }
            Expr::FString { segments, .. } => {
                for seg in segments {
                    if let crate::frontend::core::parser::ast::FStringSegment::Interpolation {
                        expr,
                        ..
                    } = seg
                    {
                        self.collect_expr_tokens(
                            file_path,
                            expr,
                            scope_idx,
                            declared,
                            constructor_names,
                            imported_module_roots,
                        );
                    }
                }
            }
            Expr::Unsafe { body, .. } => {
                for s in &body.stmts {
                    self.collect_stmt_tokens(
                        file_path,
                        s,
                        scope_idx,
                        declared,
                        constructor_names,
                        imported_module_roots,
                    );
                }
            }
            Expr::Spawn { body, .. } => {
                for s in &body.stmts {
                    self.collect_stmt_tokens(
                        file_path,
                        s,
                        scope_idx,
                        declared,
                        constructor_names,
                        imported_module_roots,
                    );
                }
            }
            // 字面量、Error、Break、Continue 等不需要收集
            _ => {}
        }
    }
}
