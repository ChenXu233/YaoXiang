//! 类型格式化处理器

use crate::frontend::core::parser::ast::*;
use super::super::context::FormatContext;
use super::super::source_map::SourceMap;

/// 格式化类型
pub fn format_type(
    ty: &Type,
    ctx: &FormatContext,
    source_map: &SourceMap,
) -> String {
    match ty {
        Type::Name { name, .. } => name.clone(),
        Type::Int(size) => format!("i{}", size),
        Type::Float(size) => format!("f{}", size),
        Type::Char => "Char".to_string(),
        Type::String => "String".to_string(),
        Type::Bytes => "Bytes".to_string(),
        Type::Bool => "Bool".to_string(),
        Type::Void => "()".to_string(),
        Type::Struct { body } => format_struct_type(body, ctx, source_map),
        Type::NamedStruct { name, fields, .. } => {
            let fields_str = format_struct_fields(fields, ctx, source_map);
            format!("{} {{ {} }}", name, fields_str)
        }
        Type::Union(variants) => {
            let items: Vec<String> = variants
                .iter()
                .map(|(name, ty)| {
                    if let Some(t) = ty {
                        format!("{}({})", name, format_type(t, ctx, source_map))
                    } else {
                        name.clone()
                    }
                })
                .collect();
            items.join(" | ")
        }
        Type::Enum(variants) => variants.join(" | "),
        Type::Variant(defs) => {
            let items: Vec<String> = defs
                .iter()
                .map(|v| {
                    if v.params.is_empty() {
                        v.name.clone()
                    } else {
                        let params: Vec<String> = v
                            .params
                            .iter()
                            .map(|(name, ty)| {
                                if let Some(n) = name {
                                    format!("{}: {}", n, format_type(ty, ctx, source_map))
                                } else {
                                    format_type(ty, ctx, source_map)
                                }
                            })
                            .collect();
                        format!("{}({})", v.name, params.join(", "))
                    }
                })
                .collect();
            items.join(" | ")
        }
        Type::Tuple(types) => {
            let items: Vec<String> = types
                .iter()
                .map(|t| format_type(t, ctx, source_map))
                .collect();
            format!("({})", items.join(", "))
        }
        Type::Fn {
            params,
            return_type,
        } => {
            let params_str: Vec<String> = params
                .iter()
                .map(|t| format_type(t, ctx, source_map))
                .collect();
            format!(
                "({}) -> {}",
                params_str.join(", "),
                format_type(return_type, ctx, source_map)
            )
        }
        Type::Option(inner) => format!("{}?", format_type(inner, ctx, source_map)),
        Type::Result(ok, err) => {
            format!(
                "Result({}, {})",
                format_type(ok, ctx, source_map),
                format_type(err, ctx, source_map)
            )
        }
        Type::Generic { name, args, .. } => {
            let args_str: Vec<String> = args
                .iter()
                .map(|t| format_type(t, ctx, source_map))
                .collect();
            format!("{}({})", name, args_str.join(", "))
        }
        Type::AssocType {
            host_type,
            assoc_name,
            assoc_args,
            ..
        } => {
            let base = format!(
                "{}::{}",
                format_type(host_type, ctx, source_map),
                assoc_name
            );
            if assoc_args.is_empty() {
                base
            } else {
                let args_str: Vec<String> = assoc_args
                    .iter()
                    .map(|t| format_type(t, ctx, source_map))
                    .collect();
                format!("{}({})", base, args_str.join(", "))
            }
        }
        Type::Sum(types) => {
            let items: Vec<String> = types
                .iter()
                .map(|t| format_type(t, ctx, source_map))
                .collect();
            items.join(" + ")
        }
        Type::Literal { name, .. } => name.clone(),
        Type::Ref { mutable, inner, .. } => {
            if *mutable {
                format!("&mut {}", format_type(inner, ctx, source_map))
            } else {
                format!("&{}", format_type(inner, ctx, source_map))
            }
        }
        Type::Ptr(inner) => format!("*{}", format_type(inner, ctx, source_map)),
        Type::MetaType { args, .. } => {
            if args.is_empty() {
                "Type".to_string()
            } else {
                let args_str: Vec<String> = args
                    .iter()
                    .map(|t| format_type(t, ctx, source_map))
                    .collect();
                format!("Type({})", args_str.join(", "))
            }
        }
        Type::ConstExpr(expr) => super::expr::format_expr(expr, ctx, source_map),
    }
}

/// 格式化结构体类型 — 类型体是有序代码块，按声明顺序输出
fn format_struct_type(
    body: &[TypeBodyItem],
    ctx: &FormatContext,
    source_map: &SourceMap,
) -> String {
    let mut parts: Vec<String> = Vec::new();
    let mut interfaces: Vec<&str> = Vec::new();

    for item in body {
        match item {
            TypeBodyItem::Interface(name) => interfaces.push(name.as_str()),
            TypeBodyItem::Field(f) => {
                flush_interfaces(&mut parts, &mut interfaces);
                parts.push(format_struct_field(f, ctx, source_map));
            }
            TypeBodyItem::Binding(b) => {
                flush_interfaces(&mut parts, &mut interfaces);
                parts.push(format_type_body_binding(b, ctx, source_map));
            }
            TypeBodyItem::Expr(ty) => {
                flush_interfaces(&mut parts, &mut interfaces);
                parts.push(format_type(ty, ctx, source_map));
            }
        }
    }
    flush_interfaces(&mut parts, &mut interfaces);

    format!("{{ {} }}", parts.join(", "))
}

/// 把累计的连续 Interface 项合并为一个 impl 子句推入 parts
fn flush_interfaces(
    parts: &mut Vec<String>,
    interfaces: &mut Vec<&str>,
) {
    if !interfaces.is_empty() {
        parts.push(format!("impl {}", interfaces.join(", ")));
        interfaces.clear();
    }
}

/// 格式化类型体绑定项
fn format_type_body_binding(
    binding: &TypeBodyBinding,
    ctx: &FormatContext,
    source_map: &SourceMap,
) -> String {
    match &binding.kind {
        BindingKind::External {
            function,
            positions,
        } => {
            if positions.is_empty() {
                format!("{} = {}", binding.name, function)
            } else {
                let pos: Vec<String> = positions.iter().map(|p| p.to_string()).collect();
                format!("{} = {}[{}]", binding.name, function, pos.join(", "))
            }
        }
        BindingKind::DefaultExternal { function } => {
            format!("{} = {}", binding.name, function)
        }
        BindingKind::Anonymous {
            params,
            return_type,
            positions,
            body,
        } => {
            let params_str = super::expr::format_params(params, ctx, source_map);
            let ret_str = format_type(return_type, ctx, source_map);
            let pos_str = if positions.is_empty() {
                String::new()
            } else {
                let pos: Vec<String> = positions.iter().map(|p| p.to_string()).collect();
                format!("[{}]", pos.join(", "))
            };
            let body_str = super::expr::format_expr(body, ctx, source_map);
            format!(
                "{}: ({} -> {}){} = ({} => {})",
                binding.name, params_str, ret_str, pos_str, params_str, body_str
            )
        }
    }
}

/// 格式化单个结构体字段
fn format_struct_field(
    f: &StructField,
    ctx: &FormatContext,
    source_map: &SourceMap,
) -> String {
    let mut s = String::new();
    if f.is_mut {
        s.push_str("mut ");
    }
    s.push_str(&f.name);
    s.push_str(": ");
    s.push_str(&format_type(&f.ty, ctx, source_map));
    if let Some(default) = &f.default {
        s.push_str(&format!(
            " = {}",
            super::expr::format_expr(default, ctx, source_map)
        ));
    }
    s
}

/// 格式化结构体字段列表
fn format_struct_fields(
    fields: &[StructField],
    ctx: &FormatContext,
    source_map: &SourceMap,
) -> String {
    let items: Vec<String> = fields
        .iter()
        .map(|f| format_struct_field(f, ctx, source_map))
        .collect();
    items.join(", ")
}
