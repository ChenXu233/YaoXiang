//! 签名解析模块
//!
//! 解析函数签名字符串为 MonoType

use std::collections::HashSet;

use crate::frontend::core::type_system::MonoType;

use super::environment::TypeEnvironment;
use crate::util::diagnostic::ErrorCodeDefinition;

/// 解析函数签名字符串为 MonoType
///
/// 格式: "[T](param1: Type1, param2: Type2) -> ReturnType"
/// 支持泛型前缀 [T]、函数类型参数 (item: T) -> T
/// 例如: "[T](list: List<T>, fn: (item: T) -> T) -> List<T>"
pub fn parse_signature(
    signature: &str,
    env: &mut TypeEnvironment,
) -> MonoType {
    let signature = signature.trim();

    // 解析可选的泛型参数前缀 [T] 或 [T, U]
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
                    is_async: false,
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
            is_async: false,
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
            is_async: false,
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
            is_async: false,
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
                    is_async: false,
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
                is_async: false,
            };
        }
    }

    // 解析返回类型
    let return_type = Box::new(parse_type_str_with_generics(return_str, &generic_params));

    MonoType::Fn {
        params,
        return_type,
        is_async: false,
    }
}

/// 解析泛型参数前缀 [T] 或 [T, U]
/// 返回 (泛型参数列表, 剩余字符串)
fn parse_generic_prefix(s: &str) -> (Vec<String>, &str) {
    let s = s.trim();
    if s.starts_with('[') {
        if let Some(close) = s.find(']') {
            let inner = &s[1..close];
            let params: Vec<String> = inner
                .split(',')
                .map(|p| p.trim().to_string())
                .filter(|p| !p.is_empty())
                .collect();
            return (params, s[close + 1..].trim());
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
fn parse_param_with_name(
    param: &str,
    generic_params: &[String],
) -> (MonoType, String) {
    let param = param.trim();

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

    // 处理函数类型: (item: T) -> T 或元组类型: (String, Int)
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
                    is_async: false,
                };
            } else if after.is_empty() {
                // 没有 ->，是元组类型: (String, Int)
                let inner = &type_str[1..close];
                let elements = split_by_top_level_comma(inner);
                let tuple_types: Vec<MonoType> = elements
                    .iter()
                    .map(|s| parse_type_str_with_generics(s, generic_params))
                    .collect();
                return MonoType::Tuple(tuple_types);
            }
        }
    }

    // 处理泛型类型: List<T>, Dict<String, Int>
    if let Some(angle_bracket) = type_str.find('<') {
        let base = &type_str[..angle_bracket];
        let inner_start = angle_bracket + 1;
        let inner_end = type_str.len() - 1;

        if inner_end > inner_start && type_str.ends_with('>') {
            let inner = &type_str[inner_start..inner_end];

            match base {
                "List" => {
                    let inner_types = split_by_top_level_comma(inner);
                    if inner_types.len() == 1 {
                        let inner_type =
                            Box::new(parse_type_str_with_generics(inner_types[0], generic_params));
                        return MonoType::List(inner_type);
                    }
                }
                "Dict" => {
                    let parts: Vec<&str> = split_by_top_level_comma(inner);
                    if parts.len() == 2 {
                        let k = Box::new(parse_type_str_with_generics(parts[0], generic_params));
                        let v = Box::new(parse_type_str_with_generics(parts[1], generic_params));
                        return MonoType::Dict(k, v);
                    }
                }
                "Set" => {
                    let inner_types = split_by_top_level_comma(inner);
                    if inner_types.len() == 1 {
                        let inner_type =
                            Box::new(parse_type_str_with_generics(inner_types[0], generic_params));
                        return MonoType::Set(inner_type);
                    }
                }
                _ => {}
            }
        }
    }

    // 检查是否是泛型参数引用
    if generic_params.iter().any(|gp| gp == type_str) {
        // 泛型参数 → 使用 TypeRef 表示（类型检查时将其视为 Any）
        return MonoType::TypeRef(type_str.to_string());
    }

    // 基本类型
    match type_str {
        "Void" | "void" => MonoType::Void,
        "Bool" | "bool" => MonoType::Bool,
        "Int" | "int" => MonoType::Int(32),
        "Float" | "float" => MonoType::Float(64),
        "Char" | "char" => MonoType::Char,
        "String" | "string" => MonoType::String,
        "Bytes" | "bytes" => MonoType::Bytes,
        "Any" => MonoType::TypeRef("Any".to_string()),
        _ => {
            // 未知类型 → 创建 TypeRef（可能是自定义类型）
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
            '<' | '(' => depth += 1,
            '>' | ')' => depth = depth.saturating_sub(1),
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
