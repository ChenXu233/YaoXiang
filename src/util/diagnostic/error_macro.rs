//! 错误报告宏
//!
//! # 示例
//!
//! ```ignore
//! use yaoxiang::util::diagnostic::{error, diagnostic};
//!
//! // 使用错误码创建错误
//! return Err(error!(E1001, name = "x"));
//!
//! // 使用错误码创建带位置的错误
//! return Err(error!(E1001, name = "x", span = my_span));
//!
//! // 使用错误码创建带相关错误的错误
//! return Err(error!(E1001, name = "x", related = vec![other_diag]));
//! ```

/// 使用错误码创建诊断（编译期验证）
///
/// # 使用方式
///
/// ```ignore
/// use yaoxiang::util::diagnostic::error;
///
/// // 基本用法
/// return Err(error!(E1001, name = "x"));
///
/// // 带位置
/// return Err(error!(E1001, name = "x", span = my_span));
///
/// // 带相关错误
/// return Err(error!(E1001, name = "x", related = vec![other]));
/// ```
///
/// # 参数
///
/// - `code`: 错误码（如 `E1001`）
/// - `name = "value"`: 模板参数
/// - `span = span_expr`: 可选，源码位置
/// - `related = vec![...]`: 可选，相关诊断列表
/// - `lang = "zh"`: 可选，语言（默认英文）
#[macro_export]
macro_rules! error {
    ($code:ident, $($arg:tt)*) => {
        $crate::util::diagnostic::error_internal!($code, (), $($arg)*)
    };
}

/// 内部宏：处理 span 参数
#[macro_export]
macro_rules! error_internal {
    ($code:ident, (), $($arg:tt)*) => {
        $crate::__error_impl!($code, (), (), $($arg)*)
    };
}

/// 错误实现宏
#[macro_export]
macro_rules! __error_impl {
    // 匹配 span = expr 参数（必须在通用匹配之前）
    ($code:ident, ($($params:tt)*), (), span = $span:expr, $($rest:tt)*) => {
        $crate::__error_impl!(
            $code,
            ($($params)*),
            (span = $span),
            $($rest)*
        )
    };

    // 匹配 related = vec![] 参数（必须在通用匹配之前）
    ($code:ident, ($($params:tt)*), (), related = $related:expr, $($rest:tt)*) => {
        $crate::__error_impl!(
            $code,
            ($($params)*),
            (related = $related),
            $($rest)*
        )
    };

    // 匹配 lang = "zh" 参数（必须在通用匹配之前）
    ($code:ident, ($($params:tt)*), (), lang = $lang:expr, $($rest:tt)*) => {
        $crate::__error_impl!(
            $code,
            ($($params)*),
            (lang = $lang),
            $($rest)*
        )
    };

    // 通用参数匹配 - 匹配任意 identifier = expr
    ($code:ident, ($($params:tt)*), (), $param:ident = $value:expr, $($rest:tt)*) => {
        $crate::__error_impl!(
            $code,
            ($($params)* (stringify!($param), $value)),
            (),
            $($rest)*
        )
    };

    // 没有更多参数，生成构建器
    ($code:ident, ($($params:tt)*), (span = $span:expr), lang = $lang:expr) => {
        {
            let code_def = $crate::util::diagnostic::ErrorCodeDefinition::find(stringify!($code))
                .unwrap_or_else(|| panic!("Unknown error code: {}", stringify!($code)));
            let i18n = $crate::util::diagnostic::I18nRegistry::new($lang);
            let mut builder = code_def.builder();
            $(
                builder = builder.param($params.0, $params.1);
            )*
            builder.at($span).build(&i18n)
        }
    };

    ($code:ident, ($($params:tt)*), (span = $span:expr)) => {
        {
            let code_def = $crate::util::diagnostic::ErrorCodeDefinition::find(stringify!($code))
                .unwrap_or_else(|| panic!("Unknown error code: {}", stringify!($code)));
            let i18n = $crate::util::diagnostic::I18nRegistry::en();
            let mut builder = code_def.builder();
            $(
                builder = builder.param($params.0, $params.1);
            )*
            builder.at($span).build(&i18n)
        }
    };

    ($code:ident, ($($params:tt)*), (related = $related:expr), lang = $lang:expr) => {
        {
            let code_def = $crate::util::diagnostic::ErrorCodeDefinition::find(stringify!($code))
                .unwrap_or_else(|| panic!("Unknown error code: {}", stringify!($code)));
            let i18n = $crate::util::diagnostic::I18nRegistry::new($lang);
            let mut builder = code_def.builder();
            $(
                builder = builder.param($params.0, $params.1);
            )*
            builder.with_related($related).build(&i18n)
        }
    };

    ($code:ident, ($($params:tt)*), (related = $related:expr)) => {
        {
            let code_def = $crate::util::diagnostic::ErrorCodeDefinition::find(stringify!($code))
                .unwrap_or_else(|| panic!("Unknown error code: {}", stringify!($code)));
            let i18n = $crate::util::diagnostic::I18nRegistry::en();
            let mut builder = code_def.builder();
            $(
                builder = builder.param($params.0, $params.1);
            )*
            builder.with_related($related).build(&i18n)
        }
    };

    ($code:ident, ($($params:tt)*), (span = $span:expr, related = $related:expr), lang = $lang:expr) => {
        {
            let code_def = $crate::util::diagnostic::ErrorCodeDefinition::find(stringify!($code))
                .unwrap_or_else(|| panic!("Unknown error code: {}", stringify!($code)));
            let i18n = $crate::util::diagnostic::I18nRegistry::new($lang);
            let mut builder = code_def.builder();
            $(
                builder = builder.param($params.0, $params.1);
            )*
            builder.at($span).with_related($related).build(&i18n)
        }
    };

    ($code:ident, ($($params:tt)*), (span = $span:expr, related = $related:expr)) => {
        {
            let code_def = $crate::util::diagnostic::ErrorCodeDefinition::find(stringify!($code))
                .unwrap_or_else(|| panic!("Unknown error code: {}", stringify!($code)));
            let i18n = $crate::util::diagnostic::I18nRegistry::en();
            let mut builder = code_def.builder();
            $(
                builder = builder.param($params.0, $params.1);
            )*
            builder.at($span).with_related($related).build(&i18n)
        }
    };

    // 只有 lang
    ($code:ident, ($($params:tt)*), (), lang = $lang:expr) => {
        {
            let code_def = $crate::util::diagnostic::ErrorCodeDefinition::find(stringify!($code))
                .unwrap_or_else(|| panic!("Unknown error code: {}", stringify!($code)));
            let i18n = $crate::util::diagnostic::I18nRegistry::new($lang);
            let mut builder = code_def.builder();
            $(
                builder = builder.param($params.0, $params.1);
            )*
            builder.build(&i18n)
        }
    };

    // 没有任何额外参数
    ($code:ident, ($($params:tt)*), ()) => {
        {
            let code_def = $crate::util::diagnostic::ErrorCodeDefinition::find(stringify!($code))
                .unwrap_or_else(|| panic!("Unknown error code: {}", stringify!($code)));
            let i18n = $crate::util::diagnostic::I18nRegistry::en();
            let mut builder = code_def.builder();
            $(
                builder = builder.param($params.0, $params.1);
            )*
            builder.build(&i18n)
        }
    };
}
