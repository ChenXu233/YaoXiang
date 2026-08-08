//! Internationalization support for YaoXiang compiler
//!
//! Loads translations from JSON files in the `locales/` directory.
//! Auto-discovers all `.json` files in `locales/` and registers them as languages.
//!
//! # Configuration
//!
//! Configuration priority (high → low):
//! 1. CLI arguments (--lang)
//! 2. Environment variable (YAOXIANG_LANG)
//! 3. Project-level config (yaoxiang.toml i18n section)
//! 4. User-level config (~/.config/yaoxiang/config.toml i18n section)
//! 5. Default values
//!
//! # Usage
//!
//! ```rust
//! use yaoxiang::util::i18n::{t_simple, current_lang, MSG};
//!
//! // Get translated message
//! println!("{}", t_simple(MSG::CmdReceived, "zh-x-miao"));
//! ```

use std::sync::OnceLock;

use std::sync::LazyLock;
use std::collections::HashMap;

pub use crate::util::config::{I18nConfig as ConfigI18n};

/// Cache for merged i18n config to avoid repeated file reads
static MERGED_CONFIG: OnceLock<ConfigI18n> = OnceLock::new();

/// Load and merge i18n config from all sources
/// Priority: CLI > env > project > user > default
fn load_merged_config() -> ConfigI18n {
    // 1. Start with user-level config (default)
    let user_config = crate::util::config::load_user_config()
        .unwrap_or_else(|_| crate::util::config::UserConfig::default())
        .i18n;

    // 2. Try to merge project-level config if in a project
    if let Ok(project_dir) = std::env::current_dir() {
        #[cfg(feature = "cli")]
        if let Ok(manifest) = crate::package::manifest::PackageManifest::load(&project_dir) {
            if let Some(project_i18n) = manifest.i18n {
                // Project-level overrides user-level
                return ConfigI18n {
                    lang: project_i18n.lang,
                    fallback: project_i18n.fallback,
                    error_lang: project_i18n.error_lang,
                    local_lang: project_i18n.local_lang,
                };
            }
        }
    }

    // Return user-level config (or default if failed)
    user_config
}

/// Reload merged config (useful for testing)
#[cfg(test)]
pub fn reload_config() {
    // Reset the OnceLock to force reload on next access
    let _ = MERGED_CONFIG.set(load_merged_config());
}

/// Get the merged i18n config
pub fn get_i18n_config() -> &'static ConfigI18n {
    MERGED_CONFIG.get_or_init(load_merged_config)
}

/// Translation table loaded from JSON
type TranslationMap = HashMap<String, String>;

/// Load translations from a JSON string (used for compile-time embedded locales)
/// 从 JSON 字符串加载翻译（用于编译期嵌入的 locale）
#[allow(clippy::collapsible_match)]
fn load_translation_file_from_str(content: &str) -> TranslationMap {
    if let Ok(raw) = serde_json::from_str::<serde_json::Value>(content) {
        if let serde_json::Value::Object(map) = raw {
            map.into_iter()
                .filter_map(|(k, v)| {
                    if let serde_json::Value::String(s) = v {
                        Some((k, s))
                    } else {
                        None
                    }
                })
                .collect()
        } else {
            HashMap::new()
        }
    } else {
        HashMap::new()
    }
}

/// 编译期嵌入所有 locale 文件，避免运行时相对路径依赖
const LOCALE_FILES: &[(&str, &str)] = &[
    ("en", include_str!("../../../locales/en.json")),
    ("zh", include_str!("../../../locales/zh.json")),
    ("ja", include_str!("../../../locales/ja.json")),
    ("ru", include_str!("../../../locales/ru.json")),
    (
        "zh-classical",
        include_str!("../../../locales/zh-classical.json"),
    ),
    ("zh-x-miao", include_str!("../../../locales/zh-x-miao.json")),
];

/// 编译期嵌入所有 locale 文件，避免运行时相对路径依赖
static TRANSLATIONS: LazyLock<HashMap<String, TranslationMap>> = LazyLock::new(|| {
    let mut map = HashMap::new();
    for (lang, content) in LOCALE_FILES {
        let translations = load_translation_file_from_str(content);
        if !translations.is_empty() {
            map.insert(lang.to_string(), translations);
        }
    }
    map
});

/// 错误码展示条目（与 MSG 翻译同源，从同一 locales JSON 的对象值解析）
#[derive(Debug, Clone, Default)]
pub struct ErrorEntry {
    pub title: String,
    pub template: Option<String>,
    pub help: String,
    pub example: Option<String>,
    pub error_output: Option<String>,
    pub zen_message: Option<String>,
}

/// 从 locale JSON 提取错误码条目（对象值，如 `"E0001": {title, template, ...}`）
fn load_error_entries(content: &str) -> HashMap<String, ErrorEntry> {
    serde_json::from_str::<serde_json::Value>(content)
        .ok()
        .and_then(|v| v.as_object().cloned())
        .map(|map| {
            map.into_iter()
                .filter_map(|(k, v)| {
                    let obj = v.as_object()?;
                    let title = obj.get("title")?.as_str()?.to_string();
                    Some((
                        k,
                        ErrorEntry {
                            title,
                            template: obj
                                .get("template")
                                .and_then(|x| x.as_str())
                                .map(String::from),
                            help: obj
                                .get("help")
                                .and_then(|x| x.as_str())
                                .unwrap_or_default()
                                .to_string(),
                            example: obj
                                .get("example")
                                .and_then(|x| x.as_str())
                                .map(String::from),
                            error_output: obj
                                .get("error_output")
                                .and_then(|x| x.as_str())
                                .map(String::from),
                            zen_message: obj
                                .get("zen_message")
                                .and_then(|x| x.as_str())
                                .map(String::from),
                        },
                    ))
                })
                .collect()
        })
        .unwrap_or_default()
}

/// 错误码条目表（每语言一个，与 TRANSLATIONS 同一份 locale 文件加载）
static ERROR_ENTRIES: LazyLock<HashMap<String, HashMap<String, ErrorEntry>>> =
    LazyLock::new(|| {
        let mut map = HashMap::new();
        for (lang, content) in LOCALE_FILES {
            let entries = load_error_entries(content);
            if !entries.is_empty() {
                map.insert(lang.to_string(), entries);
            }
        }
        map
    });

/// 获取某语言的错误码条目表
pub fn error_entries(lang: &str) -> Option<&'static HashMap<String, ErrorEntry>> {
    ERROR_ENTRIES.get(lang)
}

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
    ($level:ident, $id:expr $(, $arg:expr)*) => {
        tracing::$level!("{}", $crate::util::i18n::t_cur($id, Some(&[$($arg),*])));
    };
}

/// Convenience function to get current language
/// Priority: YAOXIANG_LANG env > config file > fallback > default
/// Get current language for src/util/i18n messages
/// Priority: YAOXIANG_LANG env > local-lang > lang > fallback
pub fn current_lang() -> &'static str {
    // 1. Check YAOXIANG_LANG environment variable (highest priority)
    if let Ok(env_lang) = std::env::var("YAOXIANG_LANG") {
        if TRANSLATIONS.contains_key(&env_lang) {
            return TRANSLATIONS
                .keys()
                .find(|k| k.as_str() == env_lang)
                .map(|s| s.as_str())
                .unwrap_or("en");
        }
    }

    let config = get_i18n_config();

    // 2. Use explicit local-lang if set
    if let Some(ref local_lang) = config.local_lang {
        if TRANSLATIONS.contains_key(local_lang) {
            return local_lang;
        }
    }

    // 3. Fall back to lang
    if TRANSLATIONS.contains_key(&config.lang) {
        return &config.lang;
    }

    // 4. Use fallback language (英文兜底)
    fallback_lang()
}

/// Get the fallback language (英文兜底)
pub fn fallback_lang() -> &'static str {
    let config = get_i18n_config();

    // Use config fallback if available
    if TRANSLATIONS.contains_key(&config.fallback) {
        return &config.fallback;
    }

    // Default to English
    "en"
}

/// Get the language for diagnostic error messages
/// Priority: error-lang > lang > fallback
pub fn error_lang() -> &'static str {
    // 1. Check YAOXIANG_LANG environment variable first
    if let Ok(env_lang) = std::env::var("YAOXIANG_LANG") {
        if TRANSLATIONS.contains_key(&env_lang) {
            return TRANSLATIONS
                .keys()
                .find(|k| k.as_str() == env_lang)
                .map(|s| s.as_str())
                .unwrap_or("en");
        }
    }

    let config = get_i18n_config();

    // 2. Use explicit error-lang if set
    if let Some(ref error_lang) = config.error_lang {
        if TRANSLATIONS.contains_key(error_lang) {
            return error_lang;
        }
    }

    // 3. Fall back to lang
    if TRANSLATIONS.contains_key(&config.lang) {
        return &config.lang;
    }

    // 4. Fall back to fallback
    fallback_lang()
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

    // Lexer
    LexStart,
    LexCompleteWithTokens,
    LexTokenIdentifier,
    LexTokenKeyword,
    LexTokenNumber,
    LexTokenString,
    LexTokenChar,
    LexTokenOperator,
    LexTokenPunctuation,

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

    // Bytecode

    // IR Gen
    IrGenEnterScope,
    IrGenExitScope,
    IrGenRegisterLocal,
    IrGenLookupLocal,
    IrGenLookupLocalNotFound,
    IrGenBeforeProcessStmt,
    IrGenAfterProcessStmt,
    IrGenAboutToExitScope,
    IrGenAfterExitScope,

    VmI64Add,

    // General
    CompilationStart,
    CompilingSource,
    DebugRunCalled,

    DebugLoadingFunction,
    DebugTotalFunctions,
    DebugAvailableFunctions,
    DebugExecBinaryOp,
    DebugAddingNumbers,
    DebugGeneratingIRBinOp,

    // Bytecode dump messages
    BytecodeDumpHeader,
    BytecodeDumpTypeTable,
    BytecodeDumpConstants,
    BytecodeDumpFunctions,
    BytecodeMagic,
    BytecodeVersion,
    BytecodeFlags,
    BytecodeEntryPoint,
    BytecodeSectionCount,
    BytecodeFileSize,
    BytecodeFuncParams,
    BytecodeFuncReturnType,
    BytecodeFuncLocalCount,
    BytecodeFuncInstrCount,
    BytecodeFuncCode,
    BytecodeInstrIndex,
    BytecodeUnknownOpcode,

    // Debug messages
    DebugBinaryOp,
    DebugRegisters,

    // Package manager - commands
    PackageNoDepsToUpdate,
    PackageNoDepsToInstall,
    PackageDepsUpdated,
    PackageDepsResolved,
    PackageDepInstalled,
    PackageDepCached,
    PackageDepsInstallFailed,
    PackageLockUpdated,
    PackageNoDeps,
    PackageDevDepAdded,
    PackageDepAdded,
    PackageDevDepRemoved,
    PackageDepRemoved,
    PackageProjectCreated,
    PackageProjectCreatedLib,
    PackageInitHere,
    PackageFileSkipped,

    // Package manager - lock file
    PackageLockGenerated,

    // Package manager - update messages
    PackageUpdateFailed,
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
            MSG::LexStart => "lex_start",
            MSG::LexCompleteWithTokens => "lex_complete_tokens",
            MSG::LexTokenIdentifier => "lex_token_identifier",
            MSG::LexTokenKeyword => "lex_token_keyword",
            MSG::LexTokenNumber => "lex_token_number",
            MSG::LexTokenString => "lex_token_string",
            MSG::LexTokenChar => "lex_token_char",
            MSG::LexTokenOperator => "lex_token_operator",
            MSG::LexTokenPunctuation => "lex_token_punctuation",
            MSG::CodegenStart => "codegen_start",
            MSG::CodegenComplete => "codegen_complete",
            MSG::CodegenFunctions => "codegen_functions",
            MSG::CodegenConstPool => "codegen_const_pool",
            MSG::CodegenCodeSection => "codegen_code_section",
            MSG::CodegenTypeTable => "codegen_type_table",
            MSG::VmStart => "vm_start",
            MSG::VmComplete => "vm_complete",
            MSG::VmI64Add => "vm_i64_add",
            MSG::CompilationStart => "compilation_start",
            MSG::CompilingSource => "compiling_source",
            MSG::DebugRunCalled => "debug_run_called",

            // Debug logging
            MSG::DebugLoadingFunction => "debug_loading_function",
            MSG::DebugTotalFunctions => "debug_total_functions",
            MSG::DebugAvailableFunctions => "debug_available_functions",
            MSG::DebugExecBinaryOp => "debug_exec_binary_op",
            MSG::DebugAddingNumbers => "debug_adding_numbers",
            MSG::DebugGeneratingIRBinOp => "debug_generating_ir_binop",

            // Error messages

            // Bytecode dump messages
            MSG::BytecodeDumpHeader => "bytecode_dump_header",
            MSG::BytecodeDumpTypeTable => "bytecode_dump_type_table",
            MSG::BytecodeDumpConstants => "bytecode_dump_constants",
            MSG::BytecodeDumpFunctions => "bytecode_dump_functions",
            MSG::BytecodeMagic => "bytecode_magic",
            MSG::BytecodeVersion => "bytecode_version",
            MSG::BytecodeFlags => "bytecode_flags",
            MSG::BytecodeEntryPoint => "bytecode_entry_point",
            MSG::BytecodeSectionCount => "bytecode_section_count",
            MSG::BytecodeFileSize => "bytecode_file_size",
            MSG::BytecodeFuncParams => "bytecode_func_params",
            MSG::BytecodeFuncReturnType => "bytecode_func_return_type",
            MSG::BytecodeFuncLocalCount => "bytecode_func_local_count",
            MSG::BytecodeFuncInstrCount => "bytecode_func_instr_count",
            MSG::BytecodeFuncCode => "bytecode_func_code",
            MSG::BytecodeInstrIndex => "bytecode_instr_index",
            MSG::BytecodeUnknownOpcode => "bytecode_unknown_opcode",

            // REPL and Shell messages

            // Debugger messages

            // REPL messages

            // Shell messages

            // Debug messages
            MSG::DebugBinaryOp => "debug_binary_op",
            MSG::DebugRegisters => "debug_registers",

            // Other messages

            // Package manager - errors

            // Package manager - commands
            MSG::PackageNoDepsToUpdate => "package_no_deps_to_update",
            MSG::PackageNoDepsToInstall => "package_no_deps_to_install",
            MSG::PackageDepsUpdated => "package_deps_updated",
            MSG::PackageDepsResolved => "package_deps_resolved",
            MSG::PackageDepInstalled => "package_dep_installed",
            MSG::PackageDepCached => "package_dep_cached",
            MSG::PackageDepsInstallFailed => "package_deps_install_failed",
            MSG::PackageLockUpdated => "package_lock_updated",
            MSG::PackageNoDeps => "package_no_deps",
            MSG::PackageDevDepAdded => "package_dev_dep_added",
            MSG::PackageDepAdded => "package_dep_added",
            MSG::PackageDevDepRemoved => "package_dev_dep_removed",
            MSG::PackageDepRemoved => "package_dep_removed",
            MSG::PackageProjectCreated => "package_project_created",
            MSG::PackageProjectCreatedLib => "package_project_created_lib",
            MSG::PackageInitHere => "package_init_here",
            MSG::PackageFileSkipped => "package_file_skipped",

            // Package manager - lock file
            MSG::PackageLockGenerated => "package_lock_generated",

            // Package manager - source resolver

            // Package manager - update messages
            MSG::PackageUpdateFailed => "package_update_failed",

            _ => "unknown_message",
        }
    }
}

#[cfg(test)]
mod tests;
