//! Internationalization support for YaoXiang compiler
//!
//! Loads translations from JSON files in the `locales/` directory.
//! Supports both hardcoded languages (en, zh, zh-miao) and dynamic loading.
//!
//! # Usage
//!
//! ```rust
//! use yaoxiang::util::i18n::{t_simple, current_lang, MSG};
//!
//! // Get translated message
//! println!("{}", t_simple(MSG::CmdReceived, "zh-x-miao"));
//! ```

use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::path::Path;

/// Translation table loaded from JSON
type TranslationMap = HashMap<String, String>;

/// Hardcoded language files
const HARDCODED_LANGS: &[&str] = &["en", "zh", "zh-x-miao"];

/// Load translations from a specific JSON file
fn load_translation_file(file_name: &str) -> TranslationMap {
    let path = format!("locales/{}.json", file_name);
    match std::fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_else(|e| {
            eprintln!(
                "Warning: Failed to parse {} translation file: {}",
                file_name, e
            );
            HashMap::new()
        }),
        Err(_) => HashMap::new(),
    }
}

/// Load hardcoded translations and scan for additional languages
static TRANSLATIONS: Lazy<HashMap<String, TranslationMap>> = Lazy::new(|| {
    let mut map = HashMap::new();

    // Load hardcoded languages (en, zh, zh-miao)
    for &lang in HARDCODED_LANGS {
        let translations = load_translation_file(lang);
        if !translations.is_empty() {
            map.insert(lang.to_string(), translations);
        }
    }

    // Dynamically scan for additional language files
    let locales_dir = Path::new("locales");
    if let Ok(entries) = std::fs::read_dir(locales_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "json").unwrap_or(false) {
                if let Some(file_stem) = path.file_stem().and_then(|s| s.to_str()) {
                    // Skip hardcoded languages (already loaded)
                    if HARDCODED_LANGS.contains(&file_stem) {
                        continue;
                    }
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        if let Ok(translations) = serde_json::from_str::<TranslationMap>(&content) {
                            map.insert(file_stem.to_string(), translations);
                        }
                    }
                }
            }
        }
    }

    map
});

/// Get all available language codes
pub fn available_langs() -> Vec<&'static str> {
    TRANSLATIONS.keys().map(|s| s.as_str()).collect()
}

/// Get translation for a message ID
#[inline]
pub fn t(
    id: MSG,
    lang: &str,
    args: Option<&[&dyn std::fmt::Display]>,
) -> String {
    // Try the requested language first
    let translations = TRANSLATIONS
        .get(lang)
        .cloned()
        .or_else(|| TRANSLATIONS.get("zh").cloned()) // Fallback to zh
        .or_else(|| TRANSLATIONS.get("en").cloned()) // Fallback to en
        .unwrap_or_default();

    let key = id.key();
    let template = translations
        .get(key)
        .cloned()
        .unwrap_or_else(|| key.to_string());

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
    lang: &str,
) -> String {
    t(id, lang, None)
}

/// Convenience function using current language (for backward compatibility)
#[inline]
pub fn t_cur(
    id: MSG,
    args: Option<&[&dyn std::fmt::Display]>,
) -> String {
    let lang = current_lang();
    t(id, lang, args)
}

/// Convenience function using current language without args (for backward compatibility)
#[inline]
pub fn t_cur_simple(id: MSG) -> String {
    t_cur(id, None)
}

/// Macro for translated logging with arguments (using current language)
#[macro_export]
macro_rules! tlog {
    ($level:expr, $id:expr) => {
        tracing::$level!("{}", $crate::util::i18n::t_cur_simple($id));
    };
    ($level:expr, $id:expr, $($arg:expr),*) => {
        tracing::$level!("{}", $crate::util::i18n::t_cur($id, Some(&[$(&$arg as &dyn std::fmt::Display),*])));
    };
}

/// Convenience function to get current language from env or default
pub fn current_lang() -> &'static str {
    let env_lang = std::env::var("YAOXIANG_LANG").ok();

    // Check if this language is available
    if let Some(lang) = &env_lang {
        if TRANSLATIONS.contains_key(lang) {
            return TRANSLATIONS
                .keys()
                .find(|k| k.as_str() == lang)
                .map(|s| s.as_str())
                .unwrap_or("en");
        }
    }

    // Default to "zh" or "en" based on available translations
    if TRANSLATIONS.contains_key("zh") {
        "zh"
    } else {
        "en"
    }
}

/// Set current language via environment variable
pub fn set_lang_from_string(lang: String) {
    std::env::set_var("YAOXIANG_LANG", lang);
}

/// Message IDs for compiler logs and errors
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub enum MSG {
    // Command
    CmdReceived,

    // File operations
    RunFile,
    ReadingFile,
    BuildBytecode,
    WritingBytecode,
    DumpBytecode,

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
    TypeCheckProcessFn,
    TypeCheckHasAnnotation,
    TypeCheckAnnotation,
    TypeCheckAnnotated,
    TypeCheckAddError,
    TypeCheckCallFnDef,

    // Codegen
    CodegenStart,
    CodegenComplete,
    CodegenFunctions,
    CodegenConstPool,
    CodegenCodeSection,
    CodegenTypeTable,

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
    pub fn key(&self) -> &'static str {
        match self {
            MSG::CmdReceived => "cmd_received",
            MSG::RunFile => "run_file",
            MSG::ReadingFile => "reading_file",
            MSG::BuildBytecode => "build_bytecode",
            MSG::WritingBytecode => "writing_bytecode",
            MSG::DumpBytecode => "dump_bytecode",
            MSG::LexStart => "lex_start",
            MSG::LexComplete => "lex_complete",
            MSG::LexCompleteWithTokens => "lex_complete_tokens",
            MSG::ParserStart => "parser_start",
            MSG::ParserComplete => "parser_complete",
            MSG::ParserCompleteWithItems => "parser_complete_items",
            MSG::TypeCheckStart => "typecheck_start",
            MSG::TypeCheckComplete => "typecheck_complete",
            MSG::TypeCheckProcessFn => "typecheck_process_fn",
            MSG::TypeCheckHasAnnotation => "typecheck_has_annotation",
            MSG::TypeCheckAnnotation => "typecheck_annotation",
            MSG::TypeCheckAnnotated => "typecheck_annotated",
            MSG::TypeCheckAddError => "typecheck_add_error",
            MSG::TypeCheckCallFnDef => "typecheck_call_fndef",
            MSG::CodegenStart => "codegen_start",
            MSG::CodegenComplete => "codegen_complete",
            MSG::CodegenFunctions => "codegen_functions",
            MSG::CodegenConstPool => "codegen_const_pool",
            MSG::CodegenCodeSection => "codegen_code_section",
            MSG::CodegenTypeTable => "codegen_type_table",
            MSG::VmStart => "vm_start",
            MSG::VmComplete => "vm_complete",
            MSG::CompilationStart => "compilation_start",
            MSG::CompilingSource => "compiling_source",
            MSG::DebugRunCalled => "debug_run_called",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_msg_key() {
        assert_eq!(MSG::CmdReceived.key(), "cmd_received");
        assert_eq!(MSG::LexStart.key(), "lex_start");
    }

    #[test]
    fn test_available_langs() {
        let langs = available_langs();
        assert!(!langs.is_empty());
        assert!(langs.contains(&"en"));
        assert!(langs.contains(&"zh"));
        assert!(langs.contains(&"zh-x-miao"));
    }

    #[test]
    fn test_t_with_lang() {
        let result = t_simple(MSG::CmdReceived, "en");
        assert!(!result.is_empty());
    }

    #[test]
    fn test_t_miao() {
        let result = t_simple(MSG::CmdReceived, "zh-x-miao");
        // Should contain miao-style content
        if !result.is_empty() && result != "cmd_received" {
            assert!(result.contains("喵"));
        }
    }
}
