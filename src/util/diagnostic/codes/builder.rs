//! 通用诊断构建器
//!
//! 支持模板参数化的错误消息构建器，替代 trait-per-error 设计

use crate::util::span::Span;
use crate::util::diagnostic::{Diagnostic, Severity};
use crate::util::i18n::error_lang;
use std::cell::RefCell;
use std::collections::HashMap;

thread_local! {
    /// 当前类型检查 walk 正在访问的节点 span（#324）
    static CURRENT_SPAN: RefCell<Option<Span>> = const { RefCell::new(None) };
}

/// walk 入口挂当前节点 span；返回的 guard Drop 时恢复前值（嵌套安全、早退安全）。
///
/// builder 构造诊断时的 span 解析优先级：显式 `.at()` > 此处上下文自动填 >
/// 都没有则按 `requires_span` 处理（debug panic / release 降级 E8001）。
pub fn push_current_span(span: Span) -> SpanGuard {
    let prev = CURRENT_SPAN.with(|cell| cell.replace(Some(span)));
    SpanGuard { prev }
}

/// 取当前 walk 上下文 span（不在 walk 内时为 None）
pub fn current_span() -> Option<Span> {
    CURRENT_SPAN.with(|cell| *cell.borrow())
}

/// [`push_current_span`] 的作用域句柄
pub struct SpanGuard {
    prev: Option<Span>,
}

impl Drop for SpanGuard {
    fn drop(&mut self) {
        CURRENT_SPAN.with(|cell| *cell.borrow_mut() = self.prev);
    }
}

/// 诊断构建器（支持模板参数）
#[derive(Debug, Clone)]
pub struct DiagnosticBuilder {
    code: &'static str,
    params: Vec<(&'static str, String)>,
    span: Option<Span>,
    related: Vec<Diagnostic>,
    severity: Option<Severity>,
}

impl DiagnosticBuilder {
    /// 创建新的诊断构建器
    pub fn new(code: &'static str) -> Self {
        Self {
            code,
            params: Vec::new(),
            span: None,
            related: Vec::new(),
            severity: None,
        }
    }

    /// 添加模板参数
    pub fn param(
        mut self,
        key: &'static str,
        value: impl Into<String>,
    ) -> Self {
        self.params.push((key, value.into()));
        self
    }

    /// 设置位置
    #[inline]
    pub fn at(
        mut self,
        span: Span,
    ) -> Self {
        self.span = Some(span);
        self
    }

    /// 添加相关诊断
    #[inline]
    pub fn with_related(
        mut self,
        related: Vec<Diagnostic>,
    ) -> Self {
        self.related = related;
        self
    }

    /// 设置严重级别（默认 Error）
    #[inline]
    pub fn severity(
        mut self,
        severity: Severity,
    ) -> Self {
        self.severity = Some(severity);
        self
    }

    /// 使用 error_lang() 自动获取语言构建 Diagnostic
    pub fn build(&self) -> Diagnostic {
        let i18n = I18nRegistry::new(error_lang());
        let template = i18n
            .get_template(self.code)
            .unwrap_or("Internal error: missing i18n template");

        // 在 debug 模式下保持原有行为（会 panic）
        if cfg!(debug_assertions) {
            self.validate_params(template);
        } else {
            // release 下回落：检查缺失参数并返回 E8001 (避免进程崩溃)
            let missing = self.find_missing_params(template);
            if !missing.is_empty() {
                let message = format!(
                    "Internal diagnostic error: missing template parameter(s) for '{}'. template='{}', missing={:?}",
                    self.code, template, missing
                );
                let help = format!(
                    "Please report this issue. Available params: {:?}",
                    self.params.iter().map(|(k, _)| *k).collect::<Vec<_>>()
                );

                let mut diagnostic =
                    Diagnostic::error("E8001".to_string(), message, help, self.span);

                if !self.related.is_empty() {
                    diagnostic = diagnostic.with_related(self.related.clone());
                }

                return diagnostic;
            }
        }

        // 正常路径：渲染并返回 Diagnostic
        let message = if template.is_empty() {
            // 对于 E1090 等特殊错误码，从 zen_message 获取消息
            i18n.get_zen_message(self.code)
                .unwrap_or_else(|| i18n.render(template, &self.params))
        } else {
            i18n.render(template, &self.params)
        };
        let help = i18n.render_help(self.code, &self.params);

        // #324：span 强制——显式 .at() > walk 上下文自动填 > 都没有则按模式处理
        // （debug panic 拒绝构造；release 降级 E8001，与上方参数校验同策略）
        let span = match self.effective_span() {
            Ok(span) => span,
            Err(violation) => {
                if cfg!(debug_assertions) {
                    panic!("{}", violation);
                }
                let mut diagnostic = Diagnostic::error(
                    "E8001".to_string(),
                    violation,
                    "请在调用点补 .at(span)，或在类型检查 walk 上下文内构造".to_string(),
                    None,
                );
                if !self.related.is_empty() {
                    diagnostic = diagnostic.with_related(self.related.clone());
                }
                return diagnostic;
            }
        };

        // 根据 severity 创建诊断（W 前缀警告码缺省 Warning，#321 M2）
        let effective_severity = self.severity.or_else(|| {
            if self.code.starts_with('W') {
                Some(Severity::Warning)
            } else {
                None
            }
        });
        let mut diagnostic = match effective_severity {
            Some(Severity::Warning) => {
                Diagnostic::warning(self.code.to_string(), message, help, span)
            }
            Some(Severity::Info) => Diagnostic::info(self.code.to_string(), message, help, span),
            Some(Severity::Hint) => Diagnostic::hint(self.code.to_string(), message, help, span),
            Some(Severity::Error) | None => {
                Diagnostic::error(self.code.to_string(), message, help, span)
            }
        };

        if !self.related.is_empty() {
            diagnostic = diagnostic.with_related(self.related.clone());
        }

        diagnostic
    }

    /// #324：span 解析——显式 `.at()` > walk 上下文自动填 > 都没有且要求 span 时 Err
    fn effective_span(&self) -> Result<Option<Span>, String> {
        if let Some(span) = self.span {
            return Ok(Some(span));
        }
        if let Some(span) = current_span() {
            return Ok(Some(span));
        }
        if super::code_requires_span(self.code) {
            Err(format!(
                "诊断 {} 缺少 span：调用点未 .at(span) 且不在类型检查 walk 上下文中（编译器 bug，见 #324）",
                self.code
            ))
        } else {
            Ok(None)
        }
    }

    /// 查找模板中缺失的参数
    fn find_missing_params(
        &self,
        template: &str,
    ) -> Vec<String> {
        let param_keys: std::collections::HashSet<&'static str> =
            self.params.iter().map(|(k, _)| *k).collect();

        let mut chars = template.chars().peekable();
        let mut missing = Vec::new();
        while let Some(c) = chars.next() {
            if c == '{' {
                let mut key = String::new();
                while let Some(&c) = chars.peek() {
                    if c == '}' {
                        chars.next();
                        if !key.is_empty() && !param_keys.contains(key.as_str()) {
                            missing.push(key.clone());
                        }
                        break;
                    }
                    key.push(c);
                    chars.next();
                }
            }
        }
        missing
    }

    /// 验证所有占位符都有对应参数（debug 模式下 panic）
    fn validate_params(
        &self,
        template: &str,
    ) {
        let missing = self.find_missing_params(template);
        if !missing.is_empty() {
            panic!(
                "Missing parameter(s) {:?} for error code '{}'. Available: {:?}",
                missing,
                self.code,
                self.params.iter().map(|(k, _)| *k).collect::<Vec<_>>()
            );
        }
    }
}

/// 单个错误码信息（用于 I18nRegistry）
#[derive(Debug, Clone)]
pub struct ErrorInfo<'a> {
    pub title: &'a str,
    pub help: &'a str,
    pub example: Option<&'a str>,
    pub error_output: Option<&'a str>,
}

/// i18n 展示文案注册表（编译期从 JSON 加载，运行时零查表）
#[derive(Debug, Clone)]
pub struct I18nRegistry {
    /// 消息模板（含 {param} 占位符）
    templates: HashMap<&'static str, &'static str>,
    /// 标题
    titles: HashMap<&'static str, &'static str>,
    /// 帮助信息
    helps: HashMap<&'static str, &'static str>,
    /// 示例代码
    examples: HashMap<&'static str, &'static str>,
    /// 错误输出示例
    error_outputs: HashMap<&'static str, &'static str>,
    /// 禅意消息（用于 E1090 彩蛋）
    zen_messages: HashMap<&'static str, &'static str>,
}

/// 将 String 转换为 &'static str
fn to_static_string(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

/// 加载 i18n 数据（从统一 locale 加载器的 ErrorEntry 构建）
fn build_registry(lang: &str) -> I18nRegistry {
    let mut templates = HashMap::new();
    let mut titles = HashMap::new();
    let mut helps = HashMap::new();
    let mut examples = HashMap::new();
    let mut error_outputs = HashMap::new();
    let mut zen_messages = HashMap::new();

    if let Some(entries) = crate::util::i18n::error_entries(lang) {
        for (code, info) in entries {
            let code_static: &'static str = to_static_string(code.clone());
            if let Some(tmpl) = &info.template {
                templates.insert(code_static, to_static_string(tmpl.clone()));
            }
            titles.insert(code_static, to_static_string(info.title.clone()));
            helps.insert(code_static, to_static_string(info.help.clone()));

            if let Some(ex) = &info.example {
                examples.insert(code_static, to_static_string(ex.clone()));
            }
            if let Some(out) = &info.error_output {
                error_outputs.insert(code_static, to_static_string(out.clone()));
            }
            if let Some(zen) = &info.zen_message {
                zen_messages.insert(code_static, to_static_string(zen.clone()));
            }
        }
    }

    I18nRegistry {
        templates,
        titles,
        helps,
        examples,
        error_outputs,
        zen_messages,
    }
}

/// 沿回退链查找第一个命中的注册表（#325：bot 异步补齐窗口期输出仍可读）
fn chain_lookup<'a, T>(
    chain: &[&'a I18nRegistry],
    f: impl Fn(&'a I18nRegistry) -> Option<T>,
) -> Option<T> {
    chain.iter().find_map(|r| f(r))
}

impl I18nRegistry {
    /// key 级回退链：请求语言 → en → zh（zh 为构建期门槛保证齐全的人工源）
    fn fallbacks(&self) -> Vec<&Self> {
        let en = Self::new("en");
        let zh = Self::new("zh");
        let mut chain = vec![self];
        if !std::ptr::eq(self, en) {
            chain.push(en);
        }
        if !std::ptr::eq(self, zh) && !std::ptr::eq(en, zh) {
            chain.push(zh);
        }
        chain
    }

    /// 根据语言代码获取注册表（从统一 locale 加载器读取，与 MSG 翻译同源）
    pub fn new(lang: &str) -> &'static Self {
        use std::sync::LazyLock;
        use std::collections::HashMap;

        static REGISTRIES: LazyLock<HashMap<String, I18nRegistry>> = LazyLock::new(|| {
            let mut map = HashMap::new();
            for lang in crate::util::i18n::available_langs() {
                let registry = build_registry(lang);
                map.insert(lang.to_string(), registry);
            }
            map
        });

        REGISTRIES
            .get(lang)
            .or_else(|| REGISTRIES.get("zh"))
            .or_else(|| REGISTRIES.get("en"))
            .expect("No i18n registry found")
    }

    /// 获取错误信息
    pub fn get_info(
        &self,
        code: &str,
    ) -> Option<ErrorInfo<'_>> {
        chain_lookup(&self.fallbacks(), |r| {
            let title = r.titles.get(code)?;
            Some(ErrorInfo {
                title,
                help: r.helps.get(code).copied().unwrap_or(""),
                example: r.examples.get(code).copied(),
                error_output: r.error_outputs.get(code).copied(),
            })
        })
    }

    /// 获取消息模板（含 {param} 占位符）
    pub fn get_template(
        &self,
        code: &str,
    ) -> Option<&'static str> {
        chain_lookup(&self.fallbacks(), |r| r.templates.get(code).copied())
    }

    /// 获取标题
    pub fn get_title(
        &self,
        code: &str,
    ) -> String {
        chain_lookup(&self.fallbacks(), |r| r.titles.get(code).copied())
            .map(|s| s.to_string())
            .unwrap_or_else(|| code.to_string())
    }

    /// 获取帮助信息
    pub fn get_help(
        &self,
        code: &str,
    ) -> String {
        chain_lookup(&self.fallbacks(), |r| r.helps.get(code).copied())
            .map(|s| s.to_string())
            .unwrap_or_default()
    }

    /// 获取示例代码
    pub fn get_example(
        &self,
        code: &str,
    ) -> Option<String> {
        chain_lookup(&self.fallbacks(), |r| r.examples.get(code).copied()).map(|s| s.to_string())
    }

    /// 获取错误输出示例
    pub fn get_error_output(
        &self,
        code: &str,
    ) -> Option<String> {
        chain_lookup(&self.fallbacks(), |r| r.error_outputs.get(code).copied())
            .map(|s| s.to_string())
    }

    /// 获取禅意消息（用于 E1090 彩蛋）
    pub fn get_zen_message(
        &self,
        code: &str,
    ) -> Option<String> {
        self.zen_messages.get(code).map(|s| s.to_string())
    }

    /// 渲染模板（编译期完成，运行时零开销）
    pub fn render(
        &self,
        template: &'static str,
        params: &[(&'static str, String)],
    ) -> String {
        let mut result = String::with_capacity(template.len() + 64);
        let mut chars = template.chars().peekable();
        let param_map: HashMap<&str, &str> = params.iter().map(|(k, v)| (*k, v.as_str())).collect();

        while let Some(c) = chars.next() {
            if c == '{' {
                let mut key = String::new();
                while let Some(&c) = chars.peek() {
                    if c == '}' {
                        chars.next();
                        if let Some(value) = param_map.get(key.as_str()) {
                            result.push_str(value);
                        } else {
                            // 占位符不存在时保留原样
                            result.push('{');
                            result.push_str(&key);
                            result.push('}');
                        }
                        break;
                    }
                    key.push(c);
                    chars.next();
                }
            } else {
                result.push(c);
            }
        }
        result
    }

    /// 渲染帮助信息
    pub fn render_help(
        &self,
        code: &str,
        params: &[(&'static str, String)],
    ) -> String {
        if let Some(help) = chain_lookup(&self.fallbacks(), |r| r.helps.get(code).copied()) {
            self.render(help, params)
        } else {
            String::new()
        }
    }
}

#[cfg(test)]
mod fallback_tests {
    use super::*;
    use std::collections::HashMap;

    /// 构造仅含指定 (code, template) 条目的注册表（title 取 code 自身便于断言）
    fn registry_with(entries: &[(&'static str, &'static str)]) -> I18nRegistry {
        let mut templates = HashMap::new();
        let mut titles = HashMap::new();
        for (code, tpl) in entries {
            templates.insert(*code, *tpl);
            titles.insert(*code, *code);
        }
        I18nRegistry {
            templates,
            titles,
            helps: HashMap::new(),
            examples: HashMap::new(),
            error_outputs: HashMap::new(),
            zen_messages: HashMap::new(),
        }
    }

    #[test]
    fn test_chain_lookup_falls_back_to_next_registry() {
        let en = registry_with(&[("E9000", "en E9000")]);
        let zh = registry_with(&[("E9000", "zh E9000"), ("E9001", "zh E9001")]);
        let chain = [&en, &zh];

        // 命中链首：不回退
        assert_eq!(
            chain_lookup(&chain, |r| r.templates.get("E9000").copied()),
            Some("en E9000")
        );
        // 链首缺失：回退到下一级
        assert_eq!(
            chain_lookup(&chain, |r| r.templates.get("E9001").copied()),
            Some("zh E9001")
        );
        // 全链缺失：None（上层决定缺省值）
        assert_eq!(
            chain_lookup(&chain, |r| r.templates.get("E9999").copied()),
            None
        );
    }

    #[test]
    fn test_fallbacks_dedup_real_registries() {
        // ja/ja 回退链 = [ja, en, zh] 三级
        let ja = I18nRegistry::new("ja");
        assert_eq!(ja.fallbacks().len(), 3);
        // zh 自身即链尾，不重复入链
        let zh = I18nRegistry::new("zh");
        assert_eq!(zh.fallbacks().len(), 2);
        // en 在链中只出现一次
        let en = I18nRegistry::new("en");
        assert_eq!(en.fallbacks().len(), 2);
    }

    #[test]
    fn test_real_registry_fallback_template_readable() {
        // 真实注册表：所有码在 zh 齐全（构建期门槛保证），任意语言查询都能取到模板
        let ja = I18nRegistry::new("ja");
        assert!(
            ja.get_template("E1001").is_some(),
            "E1001 模板经回退链应可读"
        );
    }
}

#[cfg(test)]
mod span_enforcement_tests {
    use super::*;
    use crate::util::diagnostic::ErrorCodeDefinition;
    use crate::util::span::Position;

    fn test_span() -> Span {
        Span::new(Position::new(1, 1), Position::new(1, 5))
    }

    #[test]
    #[should_panic(expected = "缺少 span")]
    fn test_required_span_panics_without_context() {
        // E1002 非豁免码：无显式 .at() 且不在 walk 上下文 → 拒绝构造
        let _ = ErrorCodeDefinition::type_mismatch("Int", "String").build();
    }

    #[test]
    fn test_exempt_code_builds_without_span() {
        // E8001 豁免：无上下文可直接构造
        let diag = ErrorCodeDefinition::internal_error("boom").build();
        assert!(diag.span.is_none());
    }

    #[test]
    fn test_walk_context_autofills_span() {
        let span = test_span();
        let guard = push_current_span(span);
        let diag = ErrorCodeDefinition::type_mismatch("Int", "String").build();
        drop(guard);
        assert_eq!(diag.span, Some(span), "walk 上下文应自动填入 span");
    }

    #[test]
    fn test_explicit_at_overrides_context() {
        let ctx_span = test_span();
        let explicit = Span::new(Position::new(9, 9), Position::new(9, 12));
        let guard = push_current_span(ctx_span);
        let diag = ErrorCodeDefinition::type_mismatch("Int", "String")
            .at(explicit)
            .build();
        drop(guard);
        assert_eq!(diag.span, Some(explicit), "显式 .at() 应优先于上下文");
    }

    #[test]
    fn test_guard_nesting_restores_previous() {
        let outer = test_span();
        let inner = Span::new(Position::new(5, 5), Position::new(5, 8));

        let g1 = push_current_span(outer);
        assert_eq!(current_span(), Some(outer));
        {
            let g2 = push_current_span(inner);
            assert_eq!(current_span(), Some(inner));
            drop(g2);
        }
        assert_eq!(current_span(), Some(outer), "内层 drop 应恢复外层");
        drop(g1);
        assert_eq!(current_span(), None, "最外层 drop 应清空上下文");
    }
}
