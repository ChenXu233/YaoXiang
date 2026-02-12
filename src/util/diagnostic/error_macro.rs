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
    // 匹配 name = "value" 参数
    ($code:ident, ($($params:tt)*), (), name = $value:expr, $($rest:tt)*) => {
        $crate::__error_impl!(
            $code,
            ($($params)* ("name", $value)),
            (),
            $($rest)*
        )
    };

    // 匹配 type = "value" 参数
    ($code:ident, ($($params:tt)*), (), type = $value:expr, $($rest:tt)*) => {
        $crate::__error_impl!(
            $code,
            ($($params)* ("type", $value)),
            (),
            $($rest)*
        )
    };

    // 匹配 found = "value" 参数
    ($code:ident, ($($params:tt)*), (), found = $value:expr, $($rest:tt)*) => {
        $crate::__error_impl!(
            $code,
            ($($params)* ("found", $value)),
            (),
            $($rest)*
        )
    };

    // 匹配 expected = "value" 参数
    ($code:ident, ($($params:tt)*), (), expected = $value:expr, $($rest:tt)*) => {
        $crate::__error_impl!(
            $code,
            ($($params)* ("expected", $value)),
            (),
            $($rest)*
        )
    };

    // 匹配 left = "value" 参数
    ($code:ident, ($($params:tt)*), (), left = $value:expr, $($rest:tt)*) => {
        $crate::__error_impl!(
            $code,
            ($($params)* ("left", $value)),
            (),
            $($rest)*
        )
    };

    // 匹配 right = "value" 参数
    ($code:ident, ($($params:tt)*), (), right = $value:expr, $($rest:tt)*) => {
        $crate::__error_impl!(
            $code,
            ($($params)* ("right", $value)),
            (),
            $($rest)*
        )
    };

    // 匹配 label = "value" 参数
    ($code:ident, ($($params:tt)*), (), label = $value:expr, $($rest:tt)*) => {
        $crate::__error_impl!(
            $code,
            ($($params)* ("label", $value)),
            (),
            $($rest)*
        )
    };

    // 匹配 char = "value" 参数
    ($code:ident, ($($params:tt)*), (), char = $value:expr, $($rest:tt)*) => {
        $crate::__error_impl!(
            $code,
            ($($params)* ("char", $value)),
            (),
            $($rest)*
        )
    };

    // 匹配 literal = "value" 参数
    ($code:ident, ($($params:tt)*), (), literal = $value:expr, $($rest:tt)*) => {
        $crate::__error_impl!(
            $code,
            ($($params)* ("literal", $value)),
            (),
            $($rest)*
        )
    };

    // 匹配 func = "value" 参数
    ($code:ident, ($($params:tt)*), (), func = $value:expr, $($rest:tt)*) => {
        $crate::__error_impl!(
            $code,
            ($($params)* ("func", $value)),
            (),
            $($rest)*
        )
    };

    // 匹配 op = "value" 参数
    ($code:ident, ($($params:tt)*), (), op = $value:expr, $($rest:tt)*) => {
        $crate::__error_impl!(
            $code,
            ($($params)* ("op", $value)),
            (),
            $($rest)*
        )
    };

    // 匹配 field = "value" 参数
    ($code:ident, ($($params:tt)*), (), field = $value:expr, $($rest:tt)*) => {
        $crate::__error_impl!(
            $code,
            ($($params)* ("field", $value)),
            (),
            $($rest)*
        )
    };

    // 匹配 reason = "value" 参数
    ($code:ident, ($($params:tt)*), (), reason = $value:expr, $($rest:tt)*) => {
        $crate::__error_impl!(
            $code,
            ($($params)* ("reason", $value)),
            (),
            $($rest)*
        )
    };

    // 匹配 message = "value" 参数
    ($code:ident, ($($params:tt)*), (), message = $value:expr, $($rest:tt)*) => {
        $crate::__error_impl!(
            $code,
            ($($params)* ("message", $value)),
            (),
            $($rest)*
        )
    };

    // 匹配 module = "value" 参数
    ($code:ident, ($($params:tt)*), (), module = $value:expr, $($rest:tt)*) => {
        $crate::__error_impl!(
            $code,
            ($($params)* ("module", $value)),
            (),
            $($rest)*
        )
    };

    // 匹配 path = "value" 参数
    ($code:ident, ($($params:tt)*), (), path = $value:expr, $($rest:tt)*) => {
        $crate::__error_impl!(
            $code,
            ($($params)* ("path", $value)),
            (),
            $($rest)*
        )
    };

    // 匹配 expr = "value" 参数
    ($code:ident, ($($params:tt)*), (), expr = $value:expr, $($rest:tt)*) => {
        $crate::__error_impl!(
            $code,
            ($($params)* ("expr", $value)),
            (),
            $($rest)*
        )
    };

    // 匹配 pattern = "value" 参数
    ($code:ident, ($($params:tt)*), (), pattern = $value:expr, $($rest:tt)*) => {
        $crate::__error_impl!(
            $code,
            ($($params)* ("pattern", $value)),
            (),
            $($rest)*
        )
    };

    // 匹配 patterns = "value" 参数
    ($code:ident, ($($params:tt)*), (), patterns = $value:expr, $($rest:tt)*) => {
        $crate::__error_impl!(
            $code,
            ($($params)* ("patterns", $value)),
            (),
            $($rest)*
        )
    };

    // 匹配 span = expr 参数
    ($code:ident, ($($params:tt)*), (), span = $span:expr, $($rest:tt)*) => {
        $crate::__error_impl!(
            $code,
            ($($params)*),
            (span = $span),
            $($rest)*
        )
    };

    // 匹配 related = vec![] 参数
    ($code:ident, ($($params:tt)*), (), related = $related:expr, $($rest:tt)*) => {
        $crate::__error_impl!(
            $code,
            ($($params)*),
            (related = $related),
            $($rest)*
        )
    };

    // 匹配 lang = "zh" 参数
    ($code:ident, ($($params:tt)*), (), lang = $lang:expr, $($rest:tt)*) => {
        $crate::__error_impl!(
            $code,
            ($($params)*),
            (lang = $lang),
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
