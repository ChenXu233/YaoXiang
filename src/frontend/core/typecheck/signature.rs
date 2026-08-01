//! 签名解析模块
//!
//! 解析函数签名字符串为 MonoType
//!
//! std `NativeExport.signature` 的全部模式（issue #242）：
//! - 泛型前缀 `[T](...)` 与 `(T: Type)(...)`
//! - 泛型实参三种分隔符 `List(T)` / `List[T]` / `List<T>`
//! - 结构化容器 `Option` / `Result` / `Arc` / `Weak` / `Tuple`
//! - 变参 `...args`、可选参数 `?msg`、无标注参数、裸容器 `List`、`()` 返回

use std::collections::HashSet;

use crate::frontend::core::types::MonoType;

use super::environment::TypeEnvironment;
use crate::util::diagnostic::ErrorCodeDefinition;

/// 解析函数签名字符串为 MonoType
///
/// 格式: `[T](param1: Type1, param2: Type2) -> ReturnType`
/// 支持泛型前缀 `T`、函数类型参数 `(item: T) -> T`
/// 例如: `[T](list: List<T>, fn: (item: T) -> T) -> List<T>`
pub fn parse_signature(
    signature: &str,
    env: &mut TypeEnvironment,
) -> MonoType {
    let signature = signature.trim();

    // 解析可选的泛型参数前缀 (T: Type) 或 (T: Type, U: Type)
    let (generic_params, rest) = parse_generic_prefix(signature);

    // 如果不以 ( 开头且没有泛型前缀，视为常量类型签名（如 "Float"）
    if !rest.starts_with('(') && generic_params.is_empty() {
        return parse_type_str_with_generics(rest, &generic_params);
    }

    // 检查泛型参数是否有重复
    {
        let mut seen = HashSet::new();
        for gp in &generic_params {
            if !seen.insert(gp.as_str()) {
                let diag = ErrorCodeDefinition::invalid_signature_duplicate_param(gp).build();
                eprintln!("[Error] {}: {}", diag.code, diag.message);
                return MonoType::Fn {
                    params: vec![env.solver().new_var()],
                    return_type: Box::new(MonoType::Void),
                };
            }
        }
    }

    // 验证括号：必须以 ( 开头
    if !rest.starts_with('(') {
        let diag = ErrorCodeDefinition::invalid_signature("must start with '('").build();
        eprintln!("[Error] {}: {}", diag.code, diag.message);
        return MonoType::Fn {
            params: vec![env.solver().new_var()],
            return_type: Box::new(MonoType::Void),
        };
    }

    // 找到与首个 ( 匹配的 )
    let closing_paren = find_matching_close(rest, 0);
    let Some(closing_paren) = closing_paren else {
        let diag = ErrorCodeDefinition::invalid_signature("unmatched '('").build();
        eprintln!("[Error] {}: {}", diag.code, diag.message);
        return MonoType::Fn {
            params: vec![env.solver().new_var()],
            return_type: Box::new(MonoType::Void),
        };
    };

    let params_str = &rest[1..closing_paren];
    let after_params = rest[closing_paren + 1..].trim();

    // 验证签名格式：匹配的 ) 之后必须有 ->
    if !after_params.starts_with("->") {
        let diag = ErrorCodeDefinition::invalid_signature_missing_arrow().build();
        eprintln!("[Error] {}: {}", diag.code, diag.message);
        return MonoType::Fn {
            params: vec![env.solver().new_var()],
            return_type: Box::new(MonoType::Void),
        };
    }

    let return_str = after_params[2..].trim();

    // 解析参数（并验证参数名）
    let (params, param_names) = parse_params_with_names(params_str, &generic_params);

    // 检查参数名是否重复
    {
        let mut seen = HashSet::new();
        for name in &param_names {
            if !name.is_empty() && !seen.insert(name.as_str()) {
                let diag = ErrorCodeDefinition::invalid_signature_duplicate_param(name).build();
                eprintln!("[Error] {}: {}", diag.code, diag.message);
                return MonoType::Fn {
                    params: vec![env.solver().new_var()],
                    return_type: Box::new(MonoType::Void),
                };
            }
        }
    }

    // 检查参数名是否与泛型参数同名
    for name in &param_names {
        if !name.is_empty() && generic_params.contains(name) {
            let diag = ErrorCodeDefinition::invalid_signature_param_shadows_generic(name).build();
            eprintln!("[Error] {}: {}", diag.code, diag.message);
            return MonoType::Fn {
                params: vec![env.solver().new_var()],
                return_type: Box::new(MonoType::Void),
            };
        }
    }

    // 解析返回类型
    let return_type = Box::new(parse_type_str_with_generics(return_str, &generic_params));

    // 泛型绑定变量：TypeRef("T") → 共享 solver 类型变量。
    // monomorphize 在每次调用时 freshen 全部 TypeVar，
    // 因此签名模板中的变量不会跨调用点串扰。
    if generic_params.is_empty() {
        return MonoType::Fn {
            params,
            return_type,
        };
    }
    let binders: Vec<(String, MonoType)> = generic_params
        .iter()
        .map(|name| (name.clone(), env.solver().new_var()))
        .collect();
    let params = params
        .into_iter()
        .map(|ty| bind_generic_vars(ty, &binders))
        .collect();
    let return_type = Box::new(bind_generic_vars(*return_type, &binders));

    MonoType::Fn {
        params,
        return_type,
    }
}

/// 将类型中的 TypeRef(泛型绑定名) 替换为共享类型变量
fn bind_generic_vars(
    ty: MonoType,
    binders: &[(String, MonoType)],
) -> MonoType {
    match ty {
        MonoType::TypeRef(ref name) => binders
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, var)| var.clone())
            .unwrap_or(ty),
        MonoType::List(inner) => MonoType::List(Box::new(bind_generic_vars(*inner, binders))),
        MonoType::Dict(k, v) => MonoType::Dict(
            Box::new(bind_generic_vars(*k, binders)),
            Box::new(bind_generic_vars(*v, binders)),
        ),
        MonoType::Set(inner) => MonoType::Set(Box::new(bind_generic_vars(*inner, binders))),
        MonoType::Tuple(elems) => MonoType::Tuple(
            elems
                .into_iter()
                .map(|t| bind_generic_vars(t, binders))
                .collect(),
        ),
        MonoType::Fn {
            params,
            return_type,
        } => MonoType::Fn {
            params: params
                .into_iter()
                .map(|t| bind_generic_vars(t, binders))
                .collect(),
            return_type: Box::new(bind_generic_vars(*return_type, binders)),
        },
        MonoType::Option(inner) => MonoType::Option(Box::new(bind_generic_vars(*inner, binders))),
        MonoType::Result(ok, err) => MonoType::Result(
            Box::new(bind_generic_vars(*ok, binders)),
            Box::new(bind_generic_vars(*err, binders)),
        ),
        MonoType::Arc(inner) => MonoType::Arc(Box::new(bind_generic_vars(*inner, binders))),
        MonoType::Weak(inner) => MonoType::Weak(Box::new(bind_generic_vars(*inner, binders))),
        MonoType::Generic { name, args } => MonoType::Generic {
            name,
            args: args
                .into_iter()
                .map(|t| bind_generic_vars(t, binders))
                .collect(),
        },
        other => other,
    }
}

/// 解析泛型参数前缀 `[T, E]` 或 `(T: Type) / (T: Type, U: Type)`
/// 返回 (泛型参数列表, 剩余字符串)
///
/// 通过前瞻区分泛型前缀和函数参数列表：
/// - 方括号前缀：`[T](list: List<T>) -> T`
/// - 圆括号前缀后紧跟 `(`，如 `(T: Type)(list: List(T)) -> T`
/// - 函数参数列表后紧跟 `->`，如 `(a: Int, b: Int) -> Int`
fn parse_generic_prefix(s: &str) -> (Vec<String>, &str) {
    let s = s.trim();
    // 方括号形式：[T] 或 [T, E]
    if s.starts_with('[') {
        if let Some(close) = s.find(']') {
            let params: Vec<String> = s[1..close]
                .split(',')
                .map(|p| p.trim().to_string())
                .filter(|p| !p.is_empty())
                .collect();
            if !params.is_empty() {
                return (params, s[close + 1..].trim());
            }
        }
        return (Vec::new(), s);
    }
    if s.starts_with('(') {
        if let Some(close) = find_matching_close(s, 0) {
            let inner = &s[1..close];
            if inner.trim().is_empty() {
                return (Vec::new(), s);
            }
            let after = s[close + 1..].trim_start();
            if after.starts_with('(') {
                let params: Vec<String> = inner
                    .split(',')
                    .map(|p| p.trim().split(':').next().unwrap_or("").trim().to_string())
                    .filter(|p| !p.is_empty())
                    .collect();
                return (params, s[close + 1..].trim());
            }
        }
    }
    (Vec::new(), s)
}

/// 找到从 pos 开始的 ( 对应的匹配 )，正确处理嵌套
fn find_matching_close(
    s: &str,
    pos: usize,
) -> Option<usize> {
    let bytes = s.as_bytes();
    if bytes.get(pos) != Some(&b'(') {
        return None;
    }
    let mut depth: i32 = 0;
    for (i, &byte) in bytes.iter().enumerate().skip(pos) {
        match byte {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

/// 解析参数字符串，返回类型列表和参数名列表
fn parse_params_with_names(
    params_str: &str,
    generic_params: &[String],
) -> (Vec<MonoType>, Vec<String>) {
    if params_str.trim().is_empty() {
        return (Vec::new(), Vec::new());
    }

    let mut params = Vec::new();
    let mut names = Vec::new();
    let mut depth: i32 = 0;
    let mut start = 0;

    for (i, c) in params_str.char_indices() {
        match c {
            '<' | '(' | '[' => depth += 1,
            '>' | ')' | ']' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                let param = params_str[start..i].trim();
                if !param.is_empty() {
                    let (ty, name) = parse_param_with_name(param, generic_params);
                    params.push(ty);
                    names.push(name);
                }
                start = i + 1;
            }
            _ => {}
        }
    }

    // 最后一个参数
    let param = params_str[start..].trim();
    if !param.is_empty() {
        let (ty, name) = parse_param_with_name(param, generic_params);
        params.push(ty);
        names.push(name);
    }

    (params, names)
}

/// 解析单个参数，返回 (类型, 参数名)
/// 支持 "name: Type" 格式和函数类型 "name: (item: T) -> T"
/// 变参 `...args` 映射为 Any 占位（多参调用走宽容路径，与现状一致）
fn parse_param_with_name(
    param: &str,
    generic_params: &[String],
) -> (MonoType, String) {
    let param = param.trim();

    // 变参：...args
    if param.starts_with("...") {
        return (MonoType::TypeRef("Any".to_string()), String::new());
    }

    // 找到顶层的冒号（在括号/尖括号外面的第一个冒号）
    let mut depth: i32 = 0;
    let mut colon_pos = None;
    for (i, c) in param.char_indices() {
        match c {
            '(' | '<' | '[' => depth += 1,
            ')' | '>' | ']' => depth = depth.saturating_sub(1),
            ':' if depth == 0 => {
                colon_pos = Some(i);
                break;
            }
            _ => {}
        }
    }

    if let Some(pos) = colon_pos {
        let name = param[..pos].trim().to_string();
        let type_str = param[pos + 1..].trim();
        let ty = parse_type_str_with_generics(type_str, generic_params);
        (ty, name)
    } else {
        let ty = parse_type_str_with_generics(param, generic_params);
        (ty, String::new())
    }
}

/// 解析类型字符串为 MonoType，支持泛型参数引用和函数类型
fn parse_type_str_with_generics(
    type_str: &str,
    generic_params: &[String],
) -> MonoType {
    let type_str = type_str.trim();

    // 处理函数类型: (item: T) -> T 或元组类型: (String, Int) 或单位 ()
    if type_str.starts_with('(') {
        // 找到匹配的 )
        if let Some(close) = find_matching_close(type_str, 0) {
            let after = type_str[close + 1..].trim();
            if let Some(after_arrow) = after.strip_prefix("->") {
                // 这是函数类型: (params) -> ReturnType
                let params_part = &type_str[1..close];
                let return_part = after_arrow.trim();

                let (fn_params, _fn_param_names) =
                    parse_params_with_names(params_part, generic_params);
                let fn_return = parse_type_str_with_generics(return_part, generic_params);

                return MonoType::Fn {
                    params: fn_params,
                    return_type: Box::new(fn_return),
                };
            } else if after.is_empty() {
                let inner = &type_str[1..close];
                // `()` 是单位类型（std 签名中 `-> ()` 表示无返回值）
                if inner.trim().is_empty() {
                    return MonoType::Void;
                }
                // 没有 ->，是元组类型: (String, Int)
                let elements = split_by_top_level_comma(inner);
                let tuple_types: Vec<MonoType> = elements
                    .iter()
                    .map(|s| parse_type_str_with_generics(s, generic_params))
                    .collect();
                return MonoType::Tuple(tuple_types);
            }
        }
    }

    // 处理泛型类型实参：List(T) / List[T] / List<T> 三种分隔符统一
    let any = || MonoType::TypeRef("Any".to_string());
    if let Some(open_pos) = type_str.find(['(', '[', '<']) {
        let base = type_str[..open_pos].trim();
        let open_ch = type_str.as_bytes()[open_pos];
        let close_ch = match open_ch {
            b'(' => ')',
            b'[' => ']',
            _ => '>',
        };
        if !base.is_empty() && type_str.ends_with(close_ch) {
            let inner = &type_str[open_pos + 1..type_str.len() - 1];
            let args: Vec<MonoType> = split_by_top_level_comma(inner)
                .iter()
                .map(|s| parse_type_str_with_generics(s, generic_params))
                .collect();
            let arg = |i: usize| args.get(i).cloned().unwrap_or_else(any);
            match base {
                "List" => return MonoType::List(Box::new(arg(0))),
                "Dict" => return MonoType::Dict(Box::new(arg(0)), Box::new(arg(1))),
                "Set" => return MonoType::Set(Box::new(arg(0))),
                "Option" => return MonoType::Option(Box::new(arg(0))),
                "Result" => return MonoType::Result(Box::new(arg(0)), Box::new(arg(1))),
                "Arc" => return MonoType::Arc(Box::new(arg(0))),
                "Weak" => return MonoType::Weak(Box::new(arg(0))),
                "Tuple" => return MonoType::Tuple(args),
                _ => {
                    if !args.is_empty() {
                        return MonoType::Generic {
                            name: base.to_string(),
                            args,
                        };
                    }
                }
            }
        }
    }

    // 检查是否是泛型参数引用
    if generic_params.iter().any(|gp| gp == type_str) {
        // 泛型参数 → TypeRef 占位，由 bind_generic_vars 绑到共享类型变量
        return MonoType::TypeRef(type_str.to_string());
    }

    // 基本类型与裸容器
    match type_str {
        "Void" | "void" => MonoType::Void,
        "Never" | "never" => MonoType::Never,
        "Bool" | "bool" => MonoType::Bool,
        "Int" | "int" => MonoType::Int(64),
        "Float" | "float" => MonoType::Float(64),
        "Char" | "char" => MonoType::Char,
        "String" | "string" => MonoType::String,
        "Bytes" | "bytes" => MonoType::Bytes,
        "Any" => any(),
        // 裸容器：元素类型未知，用 Any 占位（dispatch 跳过未解析 TypeRef，不产生误报）
        "List" => MonoType::List(Box::new(any())),
        "Dict" => MonoType::Dict(Box::new(any()), Box::new(any())),
        "Set" => MonoType::Set(Box::new(any())),
        "Option" => MonoType::Option(Box::new(any())),
        "Result" => MonoType::Result(Box::new(any()), Box::new(any())),
        "Arc" => MonoType::Arc(Box::new(any())),
        "Weak" => MonoType::Weak(Box::new(any())),
        _ => {
            // 未知类型 → 创建 TypeRef（可能是自定义类型，如 File/DateTime/Error/Tuple）
            MonoType::TypeRef(type_str.to_string())
        }
    }
}

/// 按顶层逗号分割字符串，正确处理嵌套的 < > ( )
pub fn split_by_top_level_comma(s: &str) -> Vec<&str> {
    let mut result = Vec::new();
    let mut depth: i32 = 0;
    let mut start = 0;

    for (i, c) in s.char_indices() {
        match c {
            '<' | '(' | '[' => depth += 1,
            '>' | ')' | ']' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                let part = s[start..i].trim();
                if !part.is_empty() {
                    result.push(part);
                }
                start = i + 1;
            }
            _ => {}
        }
    }

    // 最后一个元素
    let part = s[start..].trim();
    if !part.is_empty() {
        result.push(part);
    }

    result
}
