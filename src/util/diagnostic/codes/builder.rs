//! 通用诊断构建器
//!
//! 支持模板参数化的错误消息构建器，替代 trait-per-error 设计

use crate::util::span::Span;
use crate::util::diagnostic::Diagnostic;
use std::collections::HashMap;

/// 诊断构建器（支持模板参数）
#[derive(Debug, Clone)]
pub struct DiagnosticBuilder {
    code: &'static str,
    message_template: &'static str,
    params: Vec<(&'static str, String)>,
    span: Option<Span>,
    related: Vec<Diagnostic>,
}

impl DiagnosticBuilder {
    /// 创建新的诊断构建器
    pub fn new(
        code: &'static str,
        template: &'static str,
    ) -> Self {
        Self {
            code,
            message_template: template,
            params: Vec::new(),
            span: None,
            related: Vec::new(),
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

    /// 构建 Diagnostic
    pub fn build(
        &self,
        i18n: &I18nRegistry,
    ) -> Diagnostic {
        // 在 debug 模式下保持原有行为（会 panic）
        if cfg!(debug_assertions) {
            self.validate_params();
        } else {
            // release 下回落：检查缺失参数并返回 E8001 (避免进程崩溃)
            let param_keys: std::collections::HashSet<&'static str> =
                self.params.iter().map(|(k, _)| *k).collect();

            let mut chars = self.message_template.chars().peekable();
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

            if !missing.is_empty() {
                let message = format!(
                    "Internal diagnostic error: missing template parameter(s) for '{}'. template='{}', missing={:?}",
                    self.code, self.message_template, missing
                );
                let help = format!(
                    "Please report this issue. Available params: {:?}",
                    param_keys.iter().copied().collect::<Vec<_>>()
                );

                let mut diagnostic =
                    Diagnostic::error("E8001".to_string(), message, help, self.span);

                if !self.related.is_empty() {
                    diagnostic = diagnostic.with_related(self.related.clone());
                }

                return diagnostic;
            }
        }

        // 正常路径：渲染并返回原始 Diagnostic
        let message = i18n.render(self.message_template, &self.params);
        let help = i18n.render_help(self.code, &self.params);

        let mut diagnostic = Diagnostic::error(self.code.to_string(), message, help, self.span);

        if !self.related.is_empty() {
            diagnostic = diagnostic.with_related(self.related.clone());
        }

        diagnostic
    }

    /// 验证所有占位符都有对应参数
    fn validate_params(&self) {
        let param_keys: std::collections::HashSet<&'static str> =
            self.params.iter().map(|(k, _)| *k).collect();

        let mut chars = self.message_template.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '{' {
                let mut key = String::new();
                while let Some(&c) = chars.peek() {
                    if c == '}' {
                        chars.next();
                        let key_str: &str = &key;
                        if !param_keys.contains(key_str) && !key.is_empty() {
                            panic!(
                                "Missing parameter '{}' for error code '{}'. Available: {:?}",
                                key,
                                self.code,
                                param_keys.iter().copied().collect::<Vec<_>>()
                            );
                        }
                        break;
                    }
                    key.push(c);
                    chars.next();
                }
            }
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
    /// 标题
    titles: HashMap<&'static str, &'static str>,
    /// 帮助信息
    helps: HashMap<&'static str, &'static str>,
    /// 示例代码
    examples: HashMap<&'static str, &'static str>,
    /// 错误输出示例
    error_outputs: HashMap<&'static str, &'static str>,
}

/// JSON 结构（与 i18n/*.json 对应）
#[derive(serde::Deserialize)]
struct ErrorInfoJson {
    title: String,
    help: String,
    example: Option<String>,
    error_output: Option<String>,
}

/// 将 String 转换为 &'static str
fn to_static_string(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

/// 加载 i18n 数据
fn load_i18n_data(json: &str) -> I18nRegistry {
    let data: HashMap<String, ErrorInfoJson> = serde_json::from_str(json).unwrap();

    let mut titles = HashMap::new();
    let mut helps = HashMap::new();
    let mut examples = HashMap::new();
    let mut error_outputs = HashMap::new();

    for (code, info) in data {
        let code_static: &'static str = to_static_string(code);
        titles.insert(code_static, to_static_string(info.title));
        helps.insert(code_static, to_static_string(info.help));

        if let Some(ex) = info.example {
            examples.insert(code_static, to_static_string(ex));
        }
        if let Some(out) = info.error_output {
            error_outputs.insert(code_static, to_static_string(out));
        }
    }

    I18nRegistry {
        titles,
        helps,
        examples,
        error_outputs,
    }
}

impl I18nRegistry {
    /// 获取英文注册表
    pub fn en() -> &'static Self {
        static REGISTRY: std::sync::LazyLock<I18nRegistry> =
            std::sync::LazyLock::new(|| load_i18n_data(include_str!("i18n/en.json")));
        &REGISTRY
    }

    /// 获取中文注册表
    pub fn zh() -> &'static Self {
        static REGISTRY: std::sync::LazyLock<I18nRegistry> =
            std::sync::LazyLock::new(|| load_i18n_data(include_str!("i18n/zh.json")));
        &REGISTRY
    }

    /// 根据语言代码获取注册表
    pub fn new(lang: &str) -> &'static Self {
        match lang {
            "zh" => Self::zh(),
            _ => Self::en(),
        }
    }

    /// 获取错误信息
    pub fn get_info(
        &self,
        code: &str,
    ) -> Option<ErrorInfo<'_>> {
        Some(ErrorInfo {
            title: self.titles.get(code)?,
            help: self.helps.get(code).copied().unwrap_or(""),
            example: self.examples.get(code).copied(),
            error_output: self.error_outputs.get(code).copied(),
        })
    }

    /// 获取标题
    pub fn get_title(
        &self,
        code: &str,
    ) -> String {
        self.titles
            .get(code)
            .map(|s| s.to_string())
            .unwrap_or_else(|| code.to_string())
    }

    /// 获取帮助信息
    pub fn get_help(
        &self,
        code: &str,
    ) -> String {
        self.helps
            .get(code)
            .map(|s| s.to_string())
            .unwrap_or_default()
    }

    /// 获取示例代码
    pub fn get_example(
        &self,
        code: &str,
    ) -> Option<String> {
        self.examples.get(code).map(|s| s.to_string())
    }

    /// 获取错误输出示例
    pub fn get_error_output(
        &self,
        code: &str,
    ) -> Option<String> {
        self.error_outputs.get(code).map(|s| s.to_string())
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
        if let Some(help) = self.helps.get(code) {
            self.render(help, params)
        } else {
            String::new()
        }
    }
}
