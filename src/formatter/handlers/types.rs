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
            format!("Result({}, {})", format_type(ok), format_type(err))
        }
        Type::Generic { name, args, .. } => {
            let args_str: Vec<String> = args.iter().map(format_type).collect();
            format!("{}({})", name, args_str.join(", "))
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
                format!("{}({})", base, args_str.join(", "))
            }
        }
        Type::Sum(types) => {
            let items: Vec<String> = types.iter().map(format_type).collect();
            items.join(" + ")
        }
        Type::Literal {
            name, base_type: _, ..
        } => name.clone(),
        Type::Ref { mutable, inner, .. } => {
            if *mutable {
                format!("&mut {}", format_type(inner))
            } else {
                format!("&{}", format_type(inner))
            }
        }
        Type::Ptr(inner) => format!("*{}", format_type(inner)),
        Type::MetaType { args, .. } => {
            if args.is_empty() {
                "Type".to_string()
            } else {
                let args_str: Vec<String> = args.iter().map(format_type).collect();
                format!("Type({})", args_str.join(", "))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_type_int() {
        assert_eq!(format_type(&Type::Int(32)), "i32");
        assert_eq!(format_type(&Type::Int(64)), "i64");
    }

    #[test]
    fn test_format_type_float() {
        assert_eq!(format_type(&Type::Float(32)), "f32");
        assert_eq!(format_type(&Type::Float(64)), "f64");
    }

    #[test]
    fn test_format_type_bool() {
        assert_eq!(format_type(&Type::Bool), "Bool");
    }

    #[test]
    fn test_format_type_string() {
        assert_eq!(format_type(&Type::String), "String");
    }

    #[test]
    fn test_format_type_char() {
        assert_eq!(format_type(&Type::Char), "Char");
    }

    #[test]
    fn test_format_type_void() {
        assert_eq!(format_type(&Type::Void), "()");
    }

    #[test]
    fn test_format_type_tuple() {
        let ty = Type::Tuple(vec![Type::Int(32), Type::Bool]);
        assert_eq!(format_type(&ty), "(i32, Bool)");
    }

    #[test]
    fn test_format_type_option() {
        let ty = Type::Option(Box::new(Type::Int(32)));
        assert_eq!(format_type(&ty), "i32?");
    }

    #[test]
    fn test_format_type_fn() {
        let ty = Type::Fn {
            params: vec![Type::Int(32), Type::Bool],
            return_type: Box::new(Type::String),
        };
        assert_eq!(format_type(&ty), "(i32, Bool) -> String");
    }

    #[test]
    fn test_format_type_ref() {
        let ty = Type::Ref {
            mutable: false,
            inner: Box::new(Type::Int(32)),
            span: crate::util::span::Span::dummy(),
        };
        assert_eq!(format_type(&ty), "&i32");
    }

    #[test]
    fn test_format_type_mut_ref() {
        let ty = Type::Ref {
            mutable: true,
            inner: Box::new(Type::Int(32)),
            span: crate::util::span::Span::dummy(),
        };
        assert_eq!(format_type(&ty), "&mut i32");
    }

    #[test]
    fn test_format_type_ptr() {
        let ty = Type::Ptr(Box::new(Type::Int(32)));
        assert_eq!(format_type(&ty), "*i32");
    }

    #[test]
    fn test_format_type_name() {
        let ty = Type::Name {
            name: "MyType".to_string(),
            span: crate::util::span::Span::dummy(),
        };
        assert_eq!(format_type(&ty), "MyType");
    }

    #[test]
    fn test_format_type_enum() {
        let ty = Type::Enum(vec!["A".to_string(), "B".to_string(), "C".to_string()]);
        assert_eq!(format_type(&ty), "A | B | C");
    }

    #[test]
    fn test_format_type_sum() {
        let ty = Type::Sum(vec![Type::Int(32), Type::Bool]);
        assert_eq!(format_type(&ty), "i32 + Bool");
    }
}
