//! 函数单态化子模块
//!
//! 提供函数单态化相关的辅助函数和trait

use crate::frontend::core::parser::ast::Type as AstType;
use crate::frontend::core::typecheck::MonoType;
use crate::middle::core::ir::{BasicBlock, FunctionBody, FunctionIR, Instruction, ModuleIR};
use crate::middle::passes::mono::instance::{FunctionId, InstantiationRequest};
use std::collections::HashMap;

/// 函数单态化相关trait
pub trait FunctionMonomorphizer {
    /// 检查函数是否是泛型函数
    fn is_generic_function(
        &self,
        func: &FunctionIR,
    ) -> bool;

    /// 检查类型是否包含类型变量
    fn contains_type_var(
        &self,
        ty: &MonoType,
    ) -> bool;

    /// 提取函数的类型参数
    fn extract_type_params(
        &self,
        func: &FunctionIR,
    ) -> Vec<String>;

    /// 实例化单个函数
    fn instantiate_function(
        &mut self,
        request: &InstantiationRequest,
    ) -> Option<FunctionId>;

    /// 生成特化函数名称
    fn generate_specialized_name(
        base_name: &str,
        type_args: &[MonoType],
    ) -> String;

    /// 类型替换
    fn substitute_types(
        &self,
        generic_func: &FunctionIR,
        func_id: &FunctionId,
        type_args: &[MonoType],
    ) -> FunctionIR;

    /// 单个类型替换
    fn substitute_single_type(
        &self,
        ty: &MonoType,
        type_map: &HashMap<usize, MonoType>,
    ) -> MonoType;

    /// 替换基本块中的指令
    fn substitute_block(
        &self,
        block: &BasicBlock,
        type_map: &HashMap<usize, MonoType>,
    ) -> BasicBlock;

    /// 替换指令中的类型
    fn substitute_instruction(
        &self,
        instr: &Instruction,
        type_map: &HashMap<usize, MonoType>,
    ) -> Instruction;

    /// 替换AST类型
    fn substitute_type_ast(
        &self,
        ty: &AstType,
        type_map: &HashMap<usize, MonoType>,
    ) -> AstType;

    /// 替换 ast::Type 中的类型参数（按名称匹配）
    fn substitute_type_in_ast(
        &self,
        ty: &AstType,
        name_map: &HashMap<String, MonoType>,
    ) -> AstType;
    /// 构建输出模块
    fn build_output_module(
        &self,
        original_module: &ModuleIR,
    ) -> ModuleIR;
}

/// 函数单态化器的默认实现
#[allow(clippy::only_used_in_recursion)]
impl FunctionMonomorphizer for super::Monomorphizer {
    fn is_generic_function(
        &self,
        func: &FunctionIR,
    ) -> bool {
        func.generic_params.is_some()
    }

    fn contains_type_var(
        &self,
        ty: &MonoType,
    ) -> bool {
        match ty {
            MonoType::TypeVar(_) => true,
            MonoType::List(elem) => self.contains_type_var(elem),
            MonoType::Dict(key, value) => {
                self.contains_type_var(key) || self.contains_type_var(value)
            }
            MonoType::Set(elem) => self.contains_type_var(elem),
            MonoType::Tuple(types) => types.iter().any(|t| self.contains_type_var(t)),
            MonoType::Fn {
                params,
                return_type,
                ..
            } => {
                params.iter().any(|t| self.contains_type_var(t))
                    || self.contains_type_var(return_type)
            }
            _ => false,
        }
    }

    fn extract_type_params(
        &self,
        func: &FunctionIR,
    ) -> Vec<String> {
        func.generic_params.clone().unwrap_or_default()
    }

    fn instantiate_function(
        &mut self,
        request: &InstantiationRequest,
    ) -> Option<FunctionId> {
        let key = request.specialization_key();

        if self.processed.contains(&key) {
            return None;
        }

        let generic_id = request.generic_id();
        let generic_func = self.generic_functions.get(generic_id.name())?;

        let type_args = request.type_args.clone();
        let specialized_name = Self::generate_specialized_name(generic_id.name(), &type_args);
        let func_id = FunctionId::new(specialized_name.clone(), type_args);

        let specialized_func = self.substitute_types(generic_func, &func_id, &request.type_args);

        self.processed.insert(key);
        self.specialized_functions
            .insert(specialized_name, specialized_func);

        Some(func_id)
    }

    fn generate_specialized_name(
        base_name: &str,
        type_args: &[MonoType],
    ) -> String {
        if type_args.is_empty() {
            base_name.to_string()
        } else {
            let args_str = type_args
                .iter()
                .map(|t| {
                    t.type_name()
                        .replace("/", "_")
                        .replace("<", "_")
                        .replace(">", "_")
                })
                .collect::<Vec<_>>()
                .join("_");
            format!("{}_{}", base_name, args_str)
        }
    }

    fn substitute_types(
        &self,
        generic_func: &FunctionIR,
        func_id: &FunctionId,
        type_args: &[MonoType],
    ) -> FunctionIR {
        let type_param_map: HashMap<usize, MonoType> = generic_func
            .params
            .iter()
            .enumerate()
            .filter_map(|(idx, ty)| {
                if let MonoType::TypeVar(tv) = ty {
                    if idx < type_args.len() {
                        return Some((tv.index(), type_args[idx].clone()));
                    }
                }
                None
            })
            .collect();

        let new_params: Vec<MonoType> = generic_func
            .params
            .iter()
            .map(|ty| self.substitute_single_type(ty, &type_param_map))
            .collect();
        let new_return_type =
            self.substitute_single_type(&generic_func.return_type, &type_param_map);
        let new_locals: Vec<MonoType> = match &generic_func.body {
            FunctionBody::Code { locals, .. } => locals
                .iter()
                .map(|ty| self.substitute_single_type(ty, &type_param_map))
                .collect(),
            _ => Vec::new(),
        };
        let new_blocks: Vec<BasicBlock> = match &generic_func.body {
            FunctionBody::Code { blocks, .. } => blocks
                .iter()
                .map(|block| self.substitute_block(block, &type_param_map))
                .collect(),
            _ => Vec::new(),
        };
        let new_entry = match &generic_func.body {
            FunctionBody::Code { entry, .. } => *entry,
            _ => 0,
        };

        FunctionIR {
            name: func_id.name().to_string(),
            // 特化函数是泛型的新名字实例，不复用原 DefId——mono 路径保持按名分发
            def: None,
            params: new_params,
            return_type: new_return_type,
            generic_params: None,
            body: FunctionBody::Code {
                blocks: new_blocks,
                entry: new_entry,
                locals: new_locals,
            },
        }
    }

    #[allow(clippy::only_used_in_recursion)]
    fn substitute_single_type(
        &self,
        ty: &MonoType,
        type_map: &HashMap<usize, MonoType>,
    ) -> MonoType {
        match ty {
            MonoType::TypeVar(tv) => type_map
                .get(&tv.index())
                .cloned()
                .unwrap_or_else(|| ty.clone()),
            MonoType::List(elem) => {
                MonoType::List(Box::new(self.substitute_single_type(elem, type_map)))
            }
            MonoType::Dict(key, value) => MonoType::Dict(
                Box::new(self.substitute_single_type(key, type_map)),
                Box::new(self.substitute_single_type(value, type_map)),
            ),
            MonoType::Set(elem) => {
                MonoType::Set(Box::new(self.substitute_single_type(elem, type_map)))
            }
            MonoType::Tuple(types) => MonoType::Tuple(
                types
                    .iter()
                    .map(|t| self.substitute_single_type(t, type_map))
                    .collect(),
            ),
            MonoType::Fn {
                params,
                return_type,
            } => MonoType::Fn {
                params: params
                    .iter()
                    .map(|t| self.substitute_single_type(t, type_map))
                    .collect(),
                return_type: Box::new(self.substitute_single_type(return_type, type_map)),
            },
            MonoType::Generic { name, args } => MonoType::Generic {
                name: name.clone(),
                args: args
                    .iter()
                    .map(|a| self.substitute_single_type(a, type_map))
                    .collect(),
            },
            _ => ty.clone(),
        }
    }

    fn substitute_block(
        &self,
        block: &BasicBlock,
        type_map: &HashMap<usize, MonoType>,
    ) -> BasicBlock {
        let new_instructions: Vec<Instruction> = block
            .instructions
            .iter()
            .map(|instr| self.substitute_instruction(instr, type_map))
            .collect();
        BasicBlock {
            label: block.label,
            instructions: new_instructions,
            successors: block.successors.clone(),
        }
    }

    fn substitute_instruction(
        &self,
        instr: &Instruction,
        type_map: &HashMap<usize, MonoType>,
    ) -> Instruction {
        match instr {
            Instruction::Cast {
                dst,
                src,
                target_type,
            } => {
                let new_target = self.substitute_type_ast(target_type, type_map);
                Instruction::Cast {
                    dst: dst.clone(),
                    src: src.clone(),
                    target_type: new_target,
                }
            }
            Instruction::TypeTest(operand, test_type) => {
                let new_test_type = self.substitute_type_ast(test_type, type_map);
                Instruction::TypeTest(operand.clone(), new_test_type)
            }
            _ => instr.clone(),
        }
    }

    fn substitute_type_ast(
        &self,
        ty: &AstType,
        type_map: &HashMap<usize, MonoType>,
    ) -> AstType {
        match ty {
            // 基本类型直接返回
            AstType::Name { .. }
            | AstType::Int(_)
            | AstType::Float(_)
            | AstType::Char
            | AstType::String
            | AstType::Bytes
            | AstType::Bool
            | AstType::Void
            | AstType::Enum(_) => ty.clone(),

            // 结构体：递归替换字段类型
            AstType::Struct { body } => AstType::Struct {
                body: body
                    .iter()
                    .map(|it| match it {
                        crate::frontend::core::parser::ast::TypeBodyItem::Field(f) => {
                            crate::frontend::core::parser::ast::TypeBodyItem::Field(
                                crate::frontend::core::parser::ast::StructField {
                                    name: f.name.clone(),
                                    is_mut: f.is_mut,
                                    ty: self.substitute_type_ast(&f.ty, type_map),
                                    default: f.default.clone(),
                                },
                            )
                        }
                        other => other.clone(),
                    })
                    .collect(),
            },

            // 命名结构体
            AstType::NamedStruct {
                name,
                name_span,
                fields,
            } => AstType::NamedStruct {
                name: name.clone(),
                name_span: *name_span,
                fields: fields
                    .iter()
                    .map(|f| crate::frontend::core::parser::ast::StructField {
                        name: f.name.clone(),
                        is_mut: f.is_mut,
                        ty: self.substitute_type_ast(&f.ty, type_map),
                        default: f.default.clone(),
                    })
                    .collect(),
            },

            // 联合类型
            AstType::Union(members) => AstType::Union(
                members
                    .iter()
                    .map(|(name, ty)| {
                        (
                            name.clone(),
                            ty.as_ref().map(|t| self.substitute_type_ast(t, type_map)),
                        )
                    })
                    .collect(),
            ),

            // 元组：递归替换元素类型
            AstType::Tuple(types) => AstType::Tuple(
                types
                    .iter()
                    .map(|t| self.substitute_type_ast(t, type_map))
                    .collect(),
            ),

            // 函数类型：替换参数和返回类型
            AstType::Fn {
                params,
                return_type,
            } => AstType::Fn {
                params: params
                    .iter()
                    .map(|t| self.substitute_type_ast(t, type_map))
                    .collect(),
                return_type: Box::new(self.substitute_type_ast(return_type, type_map)),
            },

            // Option：替换内部类型
            AstType::Option(inner) => {
                AstType::Option(Box::new(self.substitute_type_ast(inner, type_map)))
            }

            // Result：替换 Ok 和 Err 类型
            AstType::Result(ok, err) => AstType::Result(
                Box::new(self.substitute_type_ast(ok, type_map)),
                Box::new(self.substitute_type_ast(err, type_map)),
            ),

            // 泛型类型：替换类型参数
            AstType::Generic {
                name,
                name_span,
                args,
            } => AstType::Generic {
                name: name.clone(),
                name_span: *name_span,
                args: args
                    .iter()
                    .map(|t| self.substitute_type_ast(t, type_map))
                    .collect(),
            },

            // 关联类型：递归替换
            AstType::AssocType {
                host_type,
                assoc_name,
                assoc_name_span,
                assoc_args,
            } => AstType::AssocType {
                host_type: Box::new(self.substitute_type_ast(host_type, type_map)),
                assoc_name: assoc_name.clone(),
                assoc_name_span: *assoc_name_span,
                assoc_args: assoc_args
                    .iter()
                    .map(|t| self.substitute_type_ast(t, type_map))
                    .collect(),
            },

            // Sum 类型
            AstType::Sum(types) => AstType::Sum(
                types
                    .iter()
                    .map(|t| self.substitute_type_ast(t, type_map))
                    .collect(),
            ),

            // 字面量类型：替换基础类型
            AstType::Literal {
                name,
                name_span,
                base_type,
            } => AstType::Literal {
                name: name.clone(),
                name_span: *name_span,
                base_type: Box::new(self.substitute_type_ast(base_type, type_map)),
            },
            AstType::Ptr(inner) => {
                AstType::Ptr(Box::new(self.substitute_type_ast(inner, type_map)))
            }

            // 元类型：直接返回
            AstType::MetaType { .. } => ty.clone(),
            AstType::Ref { mutable, inner, .. } => AstType::Ref {
                mutable: *mutable,
                inner: Box::new(self.substitute_type_ast(inner, type_map)),
                span: crate::util::span::Span::default(),
            },
            AstType::ConstExpr(_) => ty.clone(),
        }
    }

    fn substitute_type_in_ast(
        &self,
        ty: &AstType,
        name_map: &HashMap<String, MonoType>,
    ) -> AstType {
        use crate::frontend::core::parser::ast::{self, Type as AstType};

        /// 将 MonoType 转换为 AST 类型表示，用于类型替换
        fn mono_to_ast_type(
            mono: &MonoType,
            span: crate::util::span::Span,
        ) -> AstType {
            match mono {
                MonoType::Int(n) => AstType::Int(*n),
                MonoType::Float(n) => AstType::Float(*n),
                MonoType::Bool => AstType::Bool,
                MonoType::String => AstType::String,
                MonoType::Char => AstType::Char,
                MonoType::Void => AstType::Void,
                MonoType::TypeRef(name) => AstType::Name {
                    name: name.clone(),
                    span,
                },
                _ => AstType::Name {
                    name: mono.type_name(),
                    span,
                },
            }
        }

        match ty {
            AstType::Name { name, span } => {
                if let Some(replacement) = name_map.get(name) {
                    mono_to_ast_type(replacement, *span)
                } else {
                    ty.clone()
                }
            }
            AstType::Struct { body } => {
                let new_body = body
                    .iter()
                    .map(|item| match item {
                        ast::TypeBodyItem::Field(f) => ast::TypeBodyItem::Field(ast::StructField {
                            name: f.name.clone(),
                            is_mut: f.is_mut,
                            ty: self.substitute_type_in_ast(&f.ty, name_map),
                            default: f.default.clone(),
                        }),
                        _ => item.clone(),
                    })
                    .collect();
                AstType::Struct { body: new_body }
            }
            AstType::NamedStruct {
                name,
                name_span,
                fields,
            } => AstType::NamedStruct {
                name: name.clone(),
                name_span: *name_span,
                fields: fields
                    .iter()
                    .map(|f| ast::StructField {
                        name: f.name.clone(),
                        is_mut: f.is_mut,
                        ty: self.substitute_type_in_ast(&f.ty, name_map),
                        default: f.default.clone(),
                    })
                    .collect(),
            },
            AstType::Tuple(types) => AstType::Tuple(
                types
                    .iter()
                    .map(|t| self.substitute_type_in_ast(t, name_map))
                    .collect(),
            ),
            AstType::Fn {
                params,
                return_type,
            } => AstType::Fn {
                params: params
                    .iter()
                    .map(|t| self.substitute_type_in_ast(t, name_map))
                    .collect(),
                return_type: Box::new(self.substitute_type_in_ast(return_type, name_map)),
            },
            AstType::Generic {
                name,
                name_span,
                args,
            } => AstType::Generic {
                name: name.clone(),
                name_span: *name_span,
                args: args
                    .iter()
                    .map(|t| self.substitute_type_in_ast(t, name_map))
                    .collect(),
            },
            AstType::Option(t) => {
                AstType::Option(Box::new(self.substitute_type_in_ast(t, name_map)))
            }
            AstType::Result(ok, err) => AstType::Result(
                Box::new(self.substitute_type_in_ast(ok, name_map)),
                Box::new(self.substitute_type_in_ast(err, name_map)),
            ),
            AstType::Union(members) => AstType::Union(
                members
                    .iter()
                    .map(|(name, ty)| {
                        (
                            name.clone(),
                            ty.as_ref()
                                .map(|t| self.substitute_type_in_ast(t, name_map)),
                        )
                    })
                    .collect(),
            ),
            AstType::AssocType {
                host_type,
                assoc_name,
                assoc_name_span,
                assoc_args,
            } => AstType::AssocType {
                host_type: Box::new(self.substitute_type_in_ast(host_type, name_map)),
                assoc_name: assoc_name.clone(),
                assoc_name_span: *assoc_name_span,
                assoc_args: assoc_args
                    .iter()
                    .map(|t| self.substitute_type_in_ast(t, name_map))
                    .collect(),
            },
            AstType::Sum(types) => AstType::Sum(
                types
                    .iter()
                    .map(|t| self.substitute_type_in_ast(t, name_map))
                    .collect(),
            ),
            AstType::Literal {
                name,
                name_span,
                base_type,
            } => AstType::Literal {
                name: name.clone(),
                name_span: *name_span,
                base_type: Box::new(self.substitute_type_in_ast(base_type, name_map)),
            },
            AstType::Ptr(inner) => {
                AstType::Ptr(Box::new(self.substitute_type_in_ast(inner, name_map)))
            }
            AstType::Ref { mutable, inner, .. } => AstType::Ref {
                mutable: *mutable,
                inner: Box::new(self.substitute_type_in_ast(inner, name_map)),
                span: crate::util::span::Span::default(),
            },
            _ => ty.clone(),
        }
    }

    fn build_output_module(
        &self,
        original_module: &ModuleIR,
    ) -> ModuleIR {
        let mut output_funcs: Vec<FunctionIR> = original_module
            .functions
            .iter()
            .filter(|f| !self.is_generic_function(f))
            .cloned()
            .collect();
        for func in self.specialized_functions.values() {
            output_funcs.push(func.clone());
        }
        ModuleIR {
            globals: original_module.globals.clone(),
            functions: output_funcs,
            ffi_libs: original_module.ffi_libs.clone(),
            ffi_bindings: original_module.ffi_bindings.clone(),
            entry_function: original_module.entry_function.clone(),
            source_files: original_module.source_files.clone(),
            function_files: original_module.function_files.clone(),
        }
    }
}
