//! 类型格式化处理器

use crate::frontend::core::parser::ast::*;

/// 格式化类型
pub fn format_type(ty: &Type) -> String {
    match ty {
        Type::Name { name, .. } => name.clone(),
        Type::Int(size) => format!("i{}", size),
        Type::Float(size) => format!("f{}", size),
        Type::Char => "Char".to_string(),
        Type::String => "String".to_string(),
        Type::Bytes => "Bytes".to_string(),
        Type::Bool => "Bool".to_string(),
        Type::Void => "()".to_string(),
        Type::Struct {
            fields,
            bindings,
            interfaces,
        } => format_struct_type(fields, bindings, interfaces),
        Type::NamedStruct { name, fields, .. } => {
            let fields_str = format_struct_fields(fields);
            format!("{} {{ {} }}", name, fields_str)
        }
        Type::Union(variants) => {
            let items: Vec<String> = variants
                .iter()
                .map(|(name, ty)| {
                    if let Some(t) = ty {
                        format!("{}({})", name, format_type(t))
                    } else {
                        name.clone()
                    }
                })
                .collect();
            items.join(" | ")
        }
        Type::Enum(variants) => {
            let items = variants.join(" | ");
            items
        }
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
                                    format!("{}: {}", n, format_type(ty))
                                } else {
                                    format_type(ty)
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
            let items: Vec<String> = types.iter().map(format_type).collect();
            format!("({})", items.join(", "))
        }
        Type::Fn {
            params,
            return_type,
        } => {
            let params_str: Vec<String> = params.iter().map(format_type).collect();
            format!(
                "({}) -> {}",
                params_str.join(", "),
                format_type(return_type)
            )
        }
        Type::Option(inner) => format!("{}?", format_type(inner)),
        Type::Result(ok, err) => {
            format!("Result[{}, {}]", format_type(ok), format_type(err))
        }
        Type::Generic { name, args, .. } => {
            let args_str: Vec<String> = args.iter().map(format_type).collect();
            format!("{}[{}]", name, args_str.join(", "))
        }
        Type::AssocType {
            host_type,
            assoc_name,
            assoc_args,
            ..
        } => {
            let base = format!("{}::{}", format_type(host_type), assoc_name);
            if assoc_args.is_empty() {
                base
            } else {
                let args_str: Vec<String> = assoc_args.iter().map(format_type).collect();
                format!("{}[{}]", base, args_str.join(", "))
            }
        }
        Type::Sum(types) => {
            let items: Vec<String> = types.iter().map(format_type).collect();
            items.join(" + ")
        }
        Type::Literal {
            name, base_type: _, ..
        } => name.clone(),
        Type::Ptr(inner) => format!("*{}", format_type(inner)),
        Type::MetaType { args } => {
            if args.is_empty() {
                "Type".to_string()
            } else {
                let args_str: Vec<String> = args.iter().map(format_type).collect();
                format!("Type[{}]", args_str.join(", "))
            }
        }
    }
}

/// 格式化结构体类型
fn format_struct_type(
    fields: &[StructField],
    bindings: &[TypeBodyBinding],
    interfaces: &[String],
) -> String {
    let mut parts = Vec::new();

    if !interfaces.is_empty() {
        parts.push(format!("impl {}", interfaces.join(", ")));
    }

    let fields_str = format_struct_fields(fields);
    if !fields_str.is_empty() {
        parts.push(fields_str);
    }

    for binding in bindings {
        parts.push(format!("{}: ...", binding.name));
    }

    format!("{{ {} }}", parts.join(", "))
}

/// 格式化结构体字段列表
fn format_struct_fields(fields: &[StructField]) -> String {
    let items: Vec<String> = fields
        .iter()
        .map(|f| {
            let mut s = String::new();
            if f.is_mut {
                s.push_str("mut ");
            }
            s.push_str(&f.name);
            s.push_str(": ");
            s.push_str(&format_type(&f.ty));
            if let Some(default) = &f.default {
                let ctx = super::super::context::FormatContext::new(
                    super::super::options::FormatOptions::default(),
                );
                s.push_str(&format!(" = {}", super::expr::format_expr(default, &ctx)));
            }
            s
        })
        .collect();
    items.join(", ")
}
