//! 前端验证统一入口
//!
//! 提供 `validate_source` 函数，作为从前端（词法分析 / 语法分析 / 类型检查）
//! 产生诊断信息的唯一接口。结果按需缓存，避免重复计算。

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::Mutex;

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;
use crate::frontend::core::parser::ast::Module;
use crate::frontend::core::typecheck::check_module;
use crate::frontend::pipeline::compilation_cache::content_hash;
use crate::util::diagnostic::Diagnostic;
use std::sync::LazyLock;

/// 前端验证结果
#[derive(Debug, Clone)]
pub struct ValidateResult {
    /// 所有诊断信息（词法、语法、类型检查）
    pub diagnostics: Vec<Diagnostic>,
    /// 模块 AST（解析成功时存在；词法/语法失败时为 `None`）
    pub module: Option<Module>,
}

/// 基于内容哈希的内存缓存，避免重复验证相同源码
static VALIDATE_CACHE: LazyLock<Mutex<HashMap<u64, Arc<ValidateResult>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// 前端验证唯一入口
///
/// 按顺序执行：词法分析 → 语法分析 → 类型检查。
/// 任一阶段失败（词法/语法）则后续阶段不执行，`module` 为 `None`。
/// 类型检查即使产生诊断，模块 AST 仍会被返回。
pub fn validate_source(source: &str) -> ValidateResult {
    let hash = content_hash(source);

    // 缓存查询
    {
        let cache = VALIDATE_CACHE.lock();
        if let Some(cached) = cache.get(&hash) {
            return cached.as_ref().clone();
        }
    }

    let result = {
        // ---- 词法分析 ----
        let tokens = match tokenize(source) {
            Ok(tokens) => tokens,
            Err(err) => {
                let result = ValidateResult {
                    diagnostics: vec![Diagnostic::error(
                        "E0001".to_string(),
                        err.to_string(),
                        String::new(),
                        None,
                    )],
                    module: None,
                };
                let mut cache = VALIDATE_CACHE.lock();
                cache.insert(hash, Arc::new(result.clone()));
                return result;
            }
        };

        // ---- 语法分析 ----
        let parse_result = parse(&tokens);
        if parse_result.has_errors {
            let result = ValidateResult {
                diagnostics: parse_result.errors,
                module: None,
            };
            let mut cache = VALIDATE_CACHE.lock();
            cache.insert(hash, Arc::new(result.clone()));
            return result;
        }

        // ---- 类型检查（语法成功则始终执行）----
        let typecheck_result = check_module(&parse_result.module, &mut None);

        ValidateResult {
            diagnostics: typecheck_result.diagnostics,
            module: Some(parse_result.module),
        }
    };

    // 缓存结果
    let mut cache = VALIDATE_CACHE.lock();
    cache.insert(hash, Arc::new(result.clone()));
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_input() {
        let result = validate_source("");
        assert!(result.diagnostics.is_empty(), "空输入不应产生诊断");
        assert!(result.module.is_some(), "空输入应产生模块 AST");
    }

    #[test]
    fn test_undeclared_let() {
        // `let` 作为未声明的标识符使用 —— 解析成功但类型检查报错
        let result = validate_source("let x = 1");
        assert!(!result.diagnostics.is_empty(), "未声明的 `let` 应产生诊断");
        assert!(result.module.is_some(), "解析成功时模块 AST 应存在");
    }

    #[test]
    fn test_valid_assignment() {
        let result = validate_source("x = 1");
        assert!(result.diagnostics.is_empty(), "合法赋值不应产生诊断");
        assert!(result.module.is_some(), "合法赋值应产生模块 AST");
    }

    #[test]
    fn test_cache_hit() {
        // 连续两次调用相同源码应返回相同结果（内容等价 + 缓存命中）
        let a = validate_source("x = 1");
        let b = validate_source("x = 1");
        assert_eq!(a.diagnostics.len(), b.diagnostics.len());
        assert_eq!(a.module.is_some(), b.module.is_some());
    }

    #[test]
    fn test_syntax_error() {
        // `@` 不能作为语句开始 —— 显式产生语法错误
        let result = validate_source("@");
        assert!(!result.diagnostics.is_empty(), "语法错误应产生诊断");
        assert!(result.module.is_none(), "语法错误时模块 AST 应为 None");
    }
}
