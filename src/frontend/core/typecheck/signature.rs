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

    // 把裸容器（无显式实参的 List/Option/Result 等）提升为隐式泛型：
    // 在解析前为每个容器参数合成名字（A/B/C...），解析时容器会展开成
    // `List(A)`，随后 A 作为隐式泛型参数与显式前缀参数一起绑定为类型变量。
    // 这样 native 签名与用户 yx 模块（裸 List 等价 List(T) 调用点推断）语义一致。
    let (generic_params, rest) = hoist_implicit_generics(rest, generic_params);
    let rest = rest.as_str();

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

/// 裸容器提升：把签名中的裸容器提升为隐式泛型参数。
///
/// 把签名中**不带实参**的 `List`/`Dict`/`Set`/`Option`/`Result`/`Arc`/`Weak`
/// 展开为 `List(<gen>)`，并把 `<gen>` 加入 generic_params。
/// 例如 `(list: List, item: Any) -> List` → 泛型 `[A]`，`(list: List(A), item: Any) -> List(A)`。
///
/// 这样 native 签名与用户 yx 模块语义一致：裸 `List` 等价 `List(T)`，调用点推断 T。
/// 未知裸类型（如 `File`/`DateTime`/`Error`）保持 TypeRef，不动。
/// 已带实参的 `Result(Int, Error)` 保持原样。
fn hoist_implicit_generics(
    signature: &str,
    mut generic_params: Vec<String>,
) -> (Vec<String>, String) {
    const NAKED: &[&str] = &["List", "Dict", "Set", "Option", "Result", "Arc", "Weak"];

    let mut gen = (b'A'..=b'Z').map(|c| (c as char).to_string());
    let mut names: std::collections::HashMap<String, String> = std::collections::HashMap::new();

    // 递归处理一个类型串：`Name(...)` / `(...) -> ...` / 裸标识符
    fn hoist_type(
        type_str: &str,
        generic_params: &mut Vec<String>,
        names: &mut std::collections::HashMap<String, String>,
        gen: &mut dyn Iterator<Item = String>,
    ) -> String {
        let t = type_str.trim();
        if t.is_empty() {
            return String::new();
        }
        // 引用类型 &T / &mut T：剥前缀递归，再拼回
        if t.starts_with('&') {
            let (mutable, rest) = if let Some(r) = t.strip_prefix("&mut ") {
                (true, r)
            } else {
                // 上面已确认 t.starts_with('&')，此处前缀必然存在
                (
                    false,
                    t.strip_prefix('&').expect("prefix '&' checked above"),
                )
            };
            let inner = hoist_type(rest, generic_params, names, gen);
            return if mutable {
                format!("&mut {inner}")
            } else {
                format!("&{inner}")
            };
        }
        // 函数类型 `(...) -> ...`
        if t.starts_with('(') {
            if let Some(close) = find_matching_close(t, 0) {
                let after = t[close + 1..].trim();
                if let Some(ret) = after.strip_prefix("->") {
                    let inner = &t[1..close];
                    let new_inner = hoist_params(inner, generic_params, names, gen);
                    let new_ret = hoist_type(ret, generic_params, names, gen);
                    return format!("({new_inner}) -> {new_ret}");
                }
            }
        }
        // 带实参的容器 `Name(args)` / `Name[args]` / `Name<args>`：递归处理实参
        if let Some(open) = t.find(['(', '[', '<']) {
            let base = t[..open].trim();
            let close_ch = match t.as_bytes()[open] {
                b'(' => ')',
                b'[' => ']',
                _ => '>',
            };
            if !base.is_empty() && t.ends_with(close_ch) {
                let inner = &t[open + 1..t.len() - 1];
                // 隐式泛型提升：容器实参中的单大写字母（T/E/R/K/V/U...）
                // 若未在显式泛型前缀中声明，视为隐式泛型参数（与用户 yx 模块
                // `Result(T, E)` 等价 `(T: Type, E: Type) -> Type` 语义一致）。
                // 多字母（Error/File/DateTime）为具体类型，不提升。
                for a in split_by_top_level_comma(inner) {
                    let a = a.trim();
                    if is_implicit_generic_name(a) && !generic_params.iter().any(|g| g == a) {
                        generic_params.push(a.to_string());
                    }
                }
                let args: Vec<String> = split_by_top_level_comma(inner)
                    .iter()
                    .map(|a| hoist_type(a, generic_params, names, gen))
                    .collect();
                return format!("{base}({})", args.join(", "));
            }
        }
        // 裸标识符：是裸容器 → 提升；否则原样
        if NAKED.contains(&t) {
            // 二元容器（Dict/Result）需要两个泛型参数，其余一个
            let arity = match t {
                "Dict" | "Result" => 2,
                _ => 1,
            };
            let mut args = Vec::with_capacity(arity);
            for k in 0..arity {
                // 同一容器同槽位复用同名（(a: Dict, b: Dict) 的 K/V 一致）；
                // 不同槽位（Dict 的 K 与 V）各自独立名字。
                let key = if arity == 1 {
                    t.to_string()
                } else {
                    format!("{t}#{k}")
                };
                let name = names
                    .entry(key)
                    .or_insert_with(|| fresh_name(gen, generic_params))
                    .clone();
                args.push(name.clone());
                let already = generic_params.iter().any(|g| g == &name);
                if !already {
                    generic_params.push(name);
                }
            }
            if args.len() == 1 {
                format!("{t}({})", args[0])
            } else {
                format!("{t}({}, {})", args[0], args[1])
            }
        } else if is_implicit_generic_name(t) && !generic_params.iter().any(|g| g == t) {
            // 裸单字母泛型（如函数类型参数 `(x: T) -> R` 中的 T/R）
            generic_params.push(t.to_string());
            t.to_string()
        } else {
            t.to_string()
        }
    }

    fn hoist_params(
        params_str: &str,
        generic_params: &mut Vec<String>,
        names: &mut std::collections::HashMap<String, String>,
        gen: &mut dyn Iterator<Item = String>,
    ) -> String {
        if params_str.trim().is_empty() {
            return String::new();
        }
        split_by_top_level_comma(params_str)
            .iter()
            .map(|param| {
                let p = param.trim();
                // 顶层冒号分割 name: type（变参 ...args / 无标注参数无冒号）
                let mut depth = 0i32;
                let mut colon = None;
                for (i, c) in p.char_indices() {
                    match c {
                        '(' | '<' | '[' => depth += 1,
                        ')' | '>' | ']' => depth = depth.saturating_sub(1),
                        ':' if depth == 0 => {
                            colon = Some(i);
                            break;
                        }
                        _ => {}
                    }
                }
                match colon {
                    Some(pos) => {
                        let name = p[..pos].trim();
                        let ty = hoist_type(&p[pos + 1..], generic_params, names, gen);
                        format!("{name}: {ty}")
                    }
                    None => hoist_type(p, generic_params, names, gen),
                }
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    // 签名顶层：`(params) -> return`
    if let Some(close) = find_matching_close(signature, 0) {
        let after = signature[close + 1..].trim();
        if let Some(ret) = after.strip_prefix("->") {
            let inner = &signature[1..close];
            let new_inner = hoist_params(inner, &mut generic_params, &mut names, &mut gen);
            let new_ret = hoist_type(ret, &mut generic_params, &mut names, &mut gen);
            return (generic_params, format!("({new_inner}) -> {new_ret}"));
        }
        // 无 ->（元组/常量签名）：原样
        let new_inner = hoist_params(
            &signature[1..close],
            &mut generic_params,
            &mut names,
            &mut gen,
        );
        return (generic_params, format!("({new_inner})"));
    }
    // 非常量/非函数签名：按类型处理
    let new = hoist_type(signature, &mut generic_params, &mut names, &mut gen);
    (generic_params, new)
}

/// 生成一个尚未被占用的泛型名字（A-Z，用尽后 G{n}）
fn fresh_name(
    gen: &mut dyn Iterator<Item = String>,
    generic_params: &[String],
) -> String {
    loop {
        let n = gen
            .next()
            .unwrap_or_else(|| format!("G{}", generic_params.len()));
        if !generic_params.iter().any(|g| g == &n) {
            return n;
        }
    }
}

/// 是否隐式泛型名：单个大写字母（T/E/R/K/V/U/N...）。
/// std 签名惯例：多字母大写（Error/File/DateTime）是具体类型，单字母是泛型。
fn is_implicit_generic_name(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 1 && b[0].is_ascii_uppercase()
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

    // 引用类型：&T / &mut T（RFC-009 借用令牌）
    // 只读操作 std 签名（如 `(list: &List) -> Int`）在调用点触发自动借用。
    if let Some(inner) = type_str.strip_prefix("&mut ") {
        return MonoType::Ref {
            mutable: true,
            inner: Box::new(parse_type_str_with_generics(inner, generic_params)),
        };
    }
    if let Some(inner) = type_str.strip_prefix('&') {
        return MonoType::Ref {
            mutable: false,
            inner: Box::new(parse_type_str_with_generics(inner, generic_params)),
        };
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
        // 注意：正常路径经 hoist_implicit_generics 已展开为 List(<gen>)；
        // 这里仅兜底（如常量类型签名 "Float" 内嵌裸容器或非参数位置），保留 Any 兼容旧行为。
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
