//! 签名解析模块
//!
//! 解析函数签名字符串为 MonoType
//!
//! std `NativeExport.signature` 的全部模式（issue #242）：
//! - 泛型前缀 `[T](...)` 与 `(T: Type)(...)`
//! - 泛型实参 `List(T)` 形态（SPEC §4.1 唯一语法）
//! - 结构化容器 `Option` / `Result` / `Arc` / `Weak`
//! - 变参 `...args`、可选参数 `?msg`、无标注参数、`()` 返回
//!
//! #391：畸形签名（结构错误、裸容器名、垃圾 token）一律 `Err` 硬拒绝，
//! 不再打印后降级为新类型变量——静默退化把签名错误推迟到用户代码处以
//! 无关错误码（E1002 等）暴露且不指向真因。std 签名是编译器静态资产，
//! 注册路径（`add_native_function_types`）对 `Err` 硬失败。

use std::collections::HashSet;

use crate::frontend::core::types::MonoType;
use crate::util::diagnostic::Diagnostic;

use super::environment::TypeEnvironment;
use crate::util::diagnostic::ErrorCodeDefinition;

/// 解析函数签名字符串为 MonoType
///
/// 格式: `[T](param1: Type1, param2: Type2) -> ReturnType`
/// 支持泛型前缀 `T`、函数类型参数 `(item: T) -> T`
/// 例如: `[T](list: List<T>, fn: (item: T) -> T) -> List<T>`
///
/// 畸形输入返回 `Err`（#391 构造期拒绝）：结构错误、裸容器名、无法识别的
/// 类型 token 都在解析点报错，由调用方决定传播方式。
pub fn parse_signature(
    signature: &str,
    env: &mut TypeEnvironment,
) -> Result<MonoType, Diagnostic> {
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
                return Err(ErrorCodeDefinition::invalid_signature_duplicate_param(gp).build());
            }
        }
    }

    // 验证括号：必须以 ( 开头
    if !rest.starts_with('(') {
        return Err(ErrorCodeDefinition::invalid_signature("must start with '('").build());
    }

    // 找到与首个 ( 匹配的 )
    let Some(closing_paren) = find_matching_close(rest, 0) else {
        return Err(ErrorCodeDefinition::invalid_signature("unmatched '('").build());
    };

    let params_str = &rest[1..closing_paren];
    let after_params = rest[closing_paren + 1..].trim();

    // 验证签名格式：匹配的 ) 之后必须有 ->
    if !after_params.starts_with("->") {
        return Err(ErrorCodeDefinition::invalid_signature_missing_arrow().build());
    }

    let return_str = after_params[2..].trim();

    // 解析参数（并验证参数名）
    let (params, param_names) = parse_params_with_names(params_str, &generic_params)?;

    // 检查参数名是否重复
    {
        let mut seen = HashSet::new();
        for name in &param_names {
            if !name.is_empty() && !seen.insert(name.as_str()) {
                return Err(ErrorCodeDefinition::invalid_signature_duplicate_param(name).build());
            }
        }
    }

    // 检查参数名是否与泛型参数同名
    for name in &param_names {
        if !name.is_empty() && generic_params.contains(name) {
            return Err(ErrorCodeDefinition::invalid_signature_param_shadows_generic(name).build());
        }
    }

    // 解析返回类型
    let return_type = Box::new(parse_type_str_with_generics(return_str, &generic_params)?);

    // 泛型绑定变量：TypeRef("T") → 共享 solver 类型变量。
    // monomorphize 在每次调用时 freshen 全部 TypeVar，
    // 因此签名模板中的变量不会跨调用点串扰。
    if generic_params.is_empty() {
        return Ok(MonoType::Fn {
            params,
            return_type,
        });
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

    Ok(MonoType::Fn {
        params,
        return_type,
    })
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
        MonoType::Ref { mutable, inner } => MonoType::Ref {
            mutable,
            inner: Box::new(bind_generic_vars(*inner, binders)),
        },
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

/// 解析签名字符串的 arity 区间（#387）
///
/// `?` 前缀参数名可选（`?msg: String`），`...` 前缀变参（`...args`）。
/// 返回 (min, max)：变参 max=None（不设上限）；常量签名/畸形输入返回
/// (0, Some(0))——纯字符串层面的区间统计，畸形签名的报错由
/// `parse_signature` 负责，这里维持宽容。
pub fn parse_signature_arity(signature: &str) -> (usize, Option<usize>) {
    let signature = signature.trim();
    let (_generic_params, rest) = parse_generic_prefix(signature);
    if !rest.starts_with('(') {
        return (0, Some(0));
    }
    let Some(closing) = find_matching_close(rest, 0) else {
        return (0, Some(0));
    };
    let params_str = &rest[1..closing];
    if params_str.trim().is_empty() {
        return (0, Some(0));
    }
    let mut min = 0usize;
    let mut max = 0usize;
    let mut variadic = false;
    for piece in split_params_top_level(params_str) {
        if piece.starts_with("...") {
            variadic = true;
        } else {
            max += 1;
            if !piece.starts_with('?') {
                min += 1;
            }
        }
    }
    if variadic {
        (min, None)
    } else {
        (min, Some(max))
    }
}

/// 按顶层逗号切分参数串（嵌套 `<>`/`()`/`[]` 内的逗号不计）
fn split_params_top_level(params_str: &str) -> Vec<String> {
    let mut pieces = Vec::new();
    let mut depth: i32 = 0;
    let mut start = 0;
    for (i, c) in params_str.char_indices() {
        match c {
            '<' | '(' | '[' => depth += 1,
            '>' | ')' | ']' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                let piece = params_str[start..i].trim();
                if !piece.is_empty() {
                    pieces.push(piece.to_string());
                }
                start = i + 1;
            }
            _ => {}
        }
    }
    let last = params_str[start..].trim();
    if !last.is_empty() {
        pieces.push(last.to_string());
    }
    pieces
}

/// 解析参数字符串，返回类型列表和参数名列表
fn parse_params_with_names(
    params_str: &str,
    generic_params: &[String],
) -> Result<(Vec<MonoType>, Vec<String>), Diagnostic> {
    if params_str.trim().is_empty() {
        return Ok((Vec::new(), Vec::new()));
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
                    let (ty, name) = parse_param_with_name(param, generic_params)?;
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
        let (ty, name) = parse_param_with_name(param, generic_params)?;
        params.push(ty);
        names.push(name);
    }

    Ok((params, names))
}

/// 解析单个参数，返回 (类型, 参数名)
/// 支持 "name: Type" 格式和函数类型 "name: (item: T) -> T"
/// 变参 `...args` 映射为 Any 占位（多参调用走宽容路径，与现状一致）
fn parse_param_with_name(
    param: &str,
    generic_params: &[String],
) -> Result<(MonoType, String), Diagnostic> {
    let param = param.trim();

    // 变参：...args
    if param.starts_with("...") {
        return Ok((MonoType::TypeRef("Any".to_string()), String::new()));
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
        let ty = parse_type_str_with_generics(type_str, generic_params)?;
        Ok((ty, name))
    } else {
        let ty = parse_type_str_with_generics(param, generic_params)?;
        Ok((ty, String::new()))
    }
}

/// 解析类型字符串为 MonoType，支持泛型参数引用和函数类型
fn parse_type_str_with_generics(
    type_str: &str,
    generic_params: &[String],
) -> Result<MonoType, Diagnostic> {
    let type_str = type_str.trim();

    // 引用类型：&T / &mut T（RFC-009 借用令牌）
    // 只读操作 std 签名（如 `(list: &Vec(A)) -> Int`）在调用点触发自动借用。
    if let Some(inner) = type_str.strip_prefix("&mut ") {
        return Ok(MonoType::Ref {
            mutable: true,
            inner: Box::new(parse_type_str_with_generics(inner, generic_params)?),
        });
    }
    if let Some(inner) = type_str.strip_prefix('&') {
        return Ok(MonoType::Ref {
            mutable: false,
            inner: Box::new(parse_type_str_with_generics(inner, generic_params)?),
        });
    }

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
                    parse_params_with_names(params_part, generic_params)?;
                let fn_return = parse_type_str_with_generics(return_part, generic_params)?;

                return Ok(MonoType::Fn {
                    params: fn_params,
                    return_type: Box::new(fn_return),
                });
            } else if after.is_empty() {
                let inner = &type_str[1..close];
                // `()` 是单位类型（std 签名中 `-> ()` 表示无返回值）
                if inner.trim().is_empty() {
                    return Ok(MonoType::Void);
                }
                // 没有 ->，是元组类型: (String, Int)
                let elements = split_by_top_level_comma(inner);
                let mut tuple_types = Vec::new();
                for s in elements {
                    tuple_types.push(parse_type_str_with_generics(s, generic_params)?);
                }
                return Ok(MonoType::make_tuple(tuple_types));
            }
        }
    }

    // 处理泛型类型实参：List(T)（SPEC §4.1 唯一语法）
    let any = || MonoType::TypeRef("Any".to_string());
    if let Some(open_pos) = type_str.find('(') {
        let base = type_str[..open_pos].trim();
        if !base.is_empty() && type_str.ends_with(')') {
            let inner = &type_str[open_pos + 1..type_str.len() - 1];
            let mut args = Vec::new();
            for s in split_by_top_level_comma(inner) {
                args.push(parse_type_str_with_generics(s, generic_params)?);
            }
            if !args.is_empty() {
                return Ok(MonoType::Generic {
                    name: base.to_string(),
                    args,
                });
            }
        }
    }

    // 检查是否是泛型参数引用
    if generic_params.iter().any(|gp| gp == type_str) {
        // 泛型参数 → TypeRef 占位，由 bind_generic_vars 绑到共享类型变量
        return Ok(MonoType::TypeRef(type_str.to_string()));
    }

    // #391：容器类型是泛型类型构造器（SPEC type-system §4.1.1），裸名不是
    // 合法类型表达式——放行只会在用户代码处以 E1002「不可索引/不可统一」
    // 暴露且不指向真因（裸容器签名曾经真实存在：旧 std.os.read_dir 写过
    // `-> List`）。泛型参数名（如形参恰好叫 List 的合法遮蔽）已在上方
    // 提前返回，不受此检查影响。
    if matches!(
        type_str,
        "List" | "Dict" | "Vec" | "Array" | "Set" | "Tuple"
    ) {
        return Err(ErrorCodeDefinition::invalid_signature_bare_container(type_str).build());
    }

    // 类型表达式到此处应为单 token（参数化/函数/元组/引用形态均已在上方
    // 分支处理）；含空白或 `->` 的残余说明上游切分失败，按畸形签名拒绝
    // 而非静默落成垃圾 TypeRef。
    if type_str.is_empty() || type_str.contains("->") || type_str.contains(char::is_whitespace) {
        return Err(ErrorCodeDefinition::invalid_signature(&format!(
            "unrecognized type `{type_str}`"
        ))
        .build());
    }

    // 基本类型（SPEC 规范名；单 token 未知名 → TypeRef，可能是自定义类型
    // 如 File/DateTime/Error）
    let ty = match type_str {
        "Void" => MonoType::Void,
        "Never" => MonoType::Never,
        "Bool" => MonoType::Bool,
        "Int" => MonoType::Int(64),
        "Float" => MonoType::Float(64),
        "Char" => MonoType::Char,
        "String" => MonoType::make_string(),
        "Bytes" => MonoType::Generic {
            name: "Bytes".into(),
            args: vec![],
        },
        "Any" => any(),
        _ => MonoType::TypeRef(type_str.to_string()),
    };
    Ok(ty)
}

/// 按顶层逗号分割字符串，正确处理嵌套的括号
pub fn split_by_top_level_comma(s: &str) -> Vec<&str> {
    let mut result = Vec::new();
    let mut depth: i32 = 0;
    let mut start = 0;

    for (i, c) in s.char_indices() {
        match c {
            '(' | '[' | '<' => depth += 1,
            ')' | ']' | '>' => depth = depth.saturating_sub(1),
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
