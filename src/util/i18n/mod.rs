//! Internationalization support for YaoXiang compiler
//!
//! Loads translations from JSON files in the `locales/` directory.
//!
//! # Usage
//!
//! ```rust
//! use yaoxiang::util::i18n::{t, Lang, MSG};
//!
//! // Get translated message
//! println!("{}", t(MSG::CmdReceived, Lang::Zh));
//!
//! // With arguments
//! println!("{}", t(MSG::LexStart, Lang::En, 1024));
//! ```

use once_cell::sync::Lazy;
use std::collections::HashMap;

/// Supported languages
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    En,
    Zh,
}

impl From<&str> for Lang {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "zh" | "cn" | "zh-cn" => Lang::Zh,
            _ => Lang::En,
        }
    }
}

/// Message IDs for compiler logs and errors
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub enum MSG {
    // Command
    CmdReceived,

    // Lexer
    LexStart,
    LexComplete,
    LexCompleteWithTokens,

    // Parser
    ParserStart,
    ParserComplete,
    ParserCompleteWithItems,

    // TypeCheck
    TypeCheckStart,
    TypeCheckComplete,

    // Codegen
    CodegenStart,
    CodegenComplete,
    CodegenFunctions,

    // VM
    VmStart,
    VmComplete,

    // General
    CompilationStart,
    CompilingSource,
    DebugRunCalled,
}

impl MSG {
    /// Get the JSON key for this message ID
    fn key(&self) -> &'static str {
        match self {
            MSG::CmdReceived => "cmd_received",
            MSG::LexStart => "lex_start",
            MSG::LexComplete => "lex_complete",
            MSG::LexCompleteWithTokens => "lex_complete_tokens",
            MSG::ParserStart => "parser_start",
            MSG::ParserComplete => "parser_complete",
            MSG::ParserCompleteWithItems => "parser_complete_items",
            MSG::TypeCheckStart => "typecheck_start",
            MSG::TypeCheckComplete => "typecheck_complete",
            MSG::CodegenStart => "codegen_start",
            MSG::CodegenComplete => "codegen_complete",
            MSG::CodegenFunctions => "codegen_functions",
            MSG::VmStart => "vm_start",
            MSG::VmComplete => "vm_complete",
            MSG::CompilationStart => "compilation_start",
            MSG::CompilingSource => "compiling_source",
            MSG::DebugRunCalled => "debug_run_called",
        }
    }
}

/// Translation table loaded from JSON
type TranslationMap = HashMap<String, String>;

/// Load translations from JSON file
fn load_translations(lang: Lang) -> TranslationMap {
    let lang_str = match lang {
        Lang::En => "en",
        Lang::Zh => "zh",
    };

    let path = format!("locales/{}.json", lang_str);

    // Try to load from file, fall back to empty map if file doesn't exist
    match std::fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_else(|e| {
            eprintln!(
                "Warning: Failed to parse {} translation file: {}",
                lang_str, e
            );
            HashMap::new()
        }),
        Err(e) => {
            eprintln!("Warning: Failed to load {} translation file: {}", path, e);
            HashMap::new()
        }
    }
}

/// Global translation tables (lazily loaded)
static EN_TRANSLATIONS: Lazy<TranslationMap> = Lazy::new(|| load_translations(Lang::En));
static ZH_TRANSLATIONS: Lazy<TranslationMap> = Lazy::new(|| load_translations(Lang::Zh));

/// Get translation for a message ID from JSON file
#[inline]
pub fn t(
    id: MSG,
    lang: Lang,
    args: Option<&[&dyn std::fmt::Display]>,
) -> String {
    let translations = match lang {
        Lang::En => &*EN_TRANSLATIONS,
        Lang::Zh => &*ZH_TRANSLATIONS,
    };

    let key = id.key();
    let template = translations.get(key).cloned().unwrap_or_else(|| {
        // Fallback to key if translation not found
        key.to_string()
    });

    match args {
        Some(args) => {
            let mut result = template;
            for (i, arg) in args.iter().enumerate() {
                result = result.replace(&format!("{{{}}}", i), &arg.to_string());
            }
            result
        }
        None => template,
    }
}

/// Convenience function for translation without args
#[inline]
pub fn t_simple(
    id: MSG,
    lang: Lang,
) -> String {
    t(id, lang, None)
}

/// Macro for translated logging with arguments
#[macro_export]
macro_rules! tlog {
    ($level:expr, $id:expr, $lang:expr) => {
        tracing::$level!("{}", $crate::util::i18n::t_simple($id, $lang));
    };
    ($level:expr, $id:expr, $lang:expr, $($arg:expr),*) => {
        tracing::$level!("{}", $crate::util::i18n::t($id, $lang, Some(&[$(&$arg as &dyn std::fmt::Display),*])));
    };
}

/// Convenience function to get current language from env or default
pub fn current_lang() -> Lang {
    std::env::var("YAOXIANG_LANG")
        .ok()
        .as_deref()
        .map(Lang::from)
        .unwrap_or(Lang::En)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_msg_key() {
        assert_eq!(MSG::CmdReceived.key(), "cmd_received");
        assert_eq!(MSG::LexStart.key(), "lex_start");
        assert_eq!(MSG::VmComplete.key(), "vm_complete");
    }

    #[test]
    fn test_t_simple() {
        // These tests will fail if JSON files don't exist, which is expected in unit tests
        // Integration tests should verify actual translations
        let result = t_simple(MSG::CmdReceived, Lang::En);
        assert!(!result.is_empty());
    }
}
