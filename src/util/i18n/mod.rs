//! Internationalization support for YaoXiang compiler
//!
//! Loads translations from JSON files in the `locales/` directory.
//! Supports both hardcoded languages (en, zh, zh-miao) and dynamic loading.
//!
//! # Configuration
//!
//! Configuration priority (high → low):
//! 1. CLI arguments (--lang)
//! 2. Environment variable (YAOXIANG_LANG)
//! 3. Project-level config (yaoxiang.toml [i18n])
//! 4. User-level config (~/.config/yaoxiang/config.toml [i18n])
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

use once_cell::sync::Lazy;
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
    let locales_dir = std::path::Path::new("locales");
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
    (debug, $id:expr) => {
        tracing::debug!("{}", $crate::util::i18n::t_cur_simple($id));
    };
    (info, $id:expr) => {
        tracing::info!("{}", $crate::util::i18n::t_cur_simple($id));
    };
    (warn, $id:expr) => {
        tracing::warn!("{}", $crate::util::i18n::t_cur_simple($id));
    };
    (error, $id:expr) => {
        tracing::error!("{}", $crate::util::i18n::t_cur_simple($id));
    };
    (debug, $id:expr, $arg1:expr) => {
        tracing::debug!("{}", $crate::util::i18n::t_cur($id, Some(&[$arg1])));
    };
    (info, $id:expr, $arg1:expr) => {
        tracing::info!("{}", $crate::util::i18n::t_cur($id, Some(&[$arg1])));
    };
    (warn, $id:expr, $arg1:expr) => {
        tracing::warn!("{}", $crate::util::i18n::t_cur($id, Some(&[$arg1])));
    };
    (error, $id:expr, $arg1:expr) => {
        tracing::error!("{}", $crate::util::i18n::t_cur($id, Some(&[$arg1])));
    };
    (debug, $id:expr, $arg1:expr, $arg2:expr) => {
        tracing::debug!("{}", $crate::util::i18n::t_cur($id, Some(&[$arg1, $arg2])));
    };
    (info, $id:expr, $arg1:expr, $arg2:expr) => {
        tracing::info!("{}", $crate::util::i18n::t_cur($id, Some(&[$arg1, $arg2])));
    };
    (warn, $id:expr, $arg1:expr, $arg2:expr) => {
        tracing::warn!("{}", $crate::util::i18n::t_cur($id, Some(&[$arg1, $arg2])));
    };
    (error, $id:expr, $arg1:expr, $arg2:expr) => {
        tracing::error!("{}", $crate::util::i18n::t_cur($id, Some(&[$arg1, $arg2])));
    };
    (debug, $id:expr, $arg1:expr, $arg2:expr, $arg3:expr) => {
        tracing::debug!(
            "{}",
            $crate::util::i18n::t_cur($id, Some(&[$arg1, $arg2, $arg3]))
        );
    };
    (info, $id:expr, $arg1:expr, $arg2:expr, $arg3:expr) => {
        tracing::info!(
            "{}",
            $crate::util::i18n::t_cur($id, Some(&[$arg1, $arg2, $arg3]))
        );
    };
    (warn, $id:expr, $arg1:expr, $arg2:expr, $arg3:expr) => {
        tracing::warn!(
            "{}",
            $crate::util::i18n::t_cur($id, Some(&[$arg1, $arg2, $arg3]))
        );
    };
    (error, $id:expr, $arg1:expr, $arg2:expr, $arg3:expr) => {
        tracing::error!(
            "{}",
            $crate::util::i18n::t_cur($id, Some(&[$arg1, $arg2, $arg3]))
        );
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
    LexComplete,
    LexCompleteWithTokens,
    LexTokenIdentifier,
    LexTokenKeyword,
    LexTokenNumber,
    LexTokenString,
    LexTokenChar,
    LexTokenOperator,
    LexTokenPunctuation,

    // Parser
    ParserStart,
    ParserComplete,
    ParserCompleteWithItems,
    ParserParseStmt,
    ParserParseExpr,
    ParserParseFnDef,
    ParserParseLet,
    ParserParseReturn,
    ParserParseIf,
    ParserParseLoop,
    ParserParseBlock,

    // TypeCheck
    TypeCheckStart,
    TypeCheckComplete,
    TypeCheckProcessFn,
    TypeCheckHasAnnotation,
    TypeCheckAnnotation,
    TypeCheckAnnotated,
    TypeCheckAddError,
    TypeCheckCallFnDef,
    TypeCheckInferExpr,
    TypeCheckInferFn,
    TypeCheckAddConstraint,
    TypeCheckSolveConstraints,
    TypeCheckVarBinding,

    // Codegen
    CodegenStart,
    CodegenComplete,
    CodegenFunctions,
    CodegenConstPool,
    CodegenCodeSection,
    CodegenTypeTable,
    CodegenGenFn,
    CodegenGenBlock,
    CodegenGenInstr,
    CodegenRegAlloc,
    CodegenAddConst,

    // VM
    VmStart,
    VmComplete,

    // Bytecode
    BytecodeDecodeI64Add,
    BytecodeDecodeI64AddTooShort,

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

    // REPL
    ReplWelcome,
    ReplHelp,
    ReplError,
    ReplUnknownCommand,
    ReplAvailableCommands,
    ReplExitCommand,
    ReplHelpCommand,
    ReplHistoryCommand,
    ReplClearCommand,

    // Shell
    ShellWelcome,
    ShellHelp,
    ShellExiting,
    ShellError,
    ShellAvailableCommands,
    ShellExitCommand,
    ShellClearCommand,
    ShellCdCommand,
    ShellPwdCommand,
    ShellLsCommand,
    ShellCodeCommands,
    ShellRunCommand,
    ShellLoadCommand,
    ShellDebugCommand,
    ShellBreakCommand,
    ShellReplCommand,
    ShellOtherInput,

    // Debugger
    DebuggerAtLocation,
    DebuggerLocals,
    DebuggerCallStack,

    // Parser Tests
    ParserTestParsedParams,
    ParserTestParsedReturnType,
    ParserTestParsedAsVar,
    ParserTestName,
    ParserTestAnnotation,

    // REPL Additional
    ReplValue,
    ReplPrompt,
    ReplHistoryEntry,

    // Shell Additional
    ShellExecTime,
    ShellLoaded,
    ShellDebugStart,
    ShellDebugCmd,

    // VM Additional
    VmExecuteFn,
    VmExecInstruction,
    VmCallStack,
    VmPushFrame,
    VmPopFrame,
    VmLoadLocal,
    VmStoreLocal,
    VmLoadArg,
    VmRegRead,
    VmRegWrite,
    VmPushStack,
    VmPopStack,
    VmCallFunc,
    VmReturnFunc,
    VmBinaryOp,
    VmI64Add,
    VmExecutingFunction,
    VmFunctionReturned,
    VmStoringResult,
    VmRegistersAfter,

    // General
    CompilationStart,
    CompilingSource,
    DebugRunCalled,

    // Debug logging
    DebugCheckingStmt,
    DebugStmtExpr,
    DebugStmtFn,
    DebugCheckingType,
    DebugStructType,
    DebugNonStructType,
    DebugLoadingFunction,
    DebugTotalFunctions,
    DebugAvailableFunctions,
    DebugFunctionLookup,
    DebugFunctionFound,
    DebugFunctionCall,
    DebugFunctionReturn,
    DebugExecBinaryOp,
    DebugAddingNumbers,
    DebugStructTypeConstructorCall,
    DebugTranslatingInstr,
    DebugGeneratingIRBinOp,

    // Error messages
    ErrorUnknownVariable,
    ErrorUnknownType,
    ErrorTypeMismatch,
    ErrorArityMismatch,
    ErrorIndexOutOfBounds,
    ErrorUnknownField,
    ErrorRecursiveType,
    ErrorUnsupportedOp,
    ErrorNonExhaustivePatterns,
    ErrorImportError,
    ErrorInferenceFailed,
    ErrorCannotInferParamType,
    HelpDidYouMean,
    HelpSimilarVariables,
    HelpInScope,

    // Bytecode dump messages
    BytecodeDumpHeader,
    BytecodeDumpTypeTable,
    BytecodeDumpConstants,
    BytecodeDumpFunctions,
    BytecodeFileHeader,
    BytecodeMagic,
    BytecodeVersion,
    BytecodeFlags,
    BytecodeEntryPoint,
    BytecodeSectionCount,
    BytecodeFileSize,
    BytecodeTypeCount,
    BytecodeConstCount,
    BytecodeFuncCount,
    BytecodeFuncName,
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
    DebugMatch,

    // Other messages
    FormatterNotImplemented,

    // Package manager - errors
    PackageErrorAlreadyExists,
    PackageErrorNotProject,
    PackageErrorDepNotFound,
    PackageErrorDepAlreadyExists,
    PackageErrorInvalidManifest,
    PackageErrorIoError,
    PackageErrorTomlParseError,

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

    // Package manager - lock file
    PackageLockGenerated,

    // Package manager - source resolver
    PackageInvalidVersion,
    PackageInvalidMajorVersion,

    // Package manager - update messages
    PackageUpdateFailed,
    PackageAlreadyUpToDate,
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
            MSG::LexComplete => "lex_complete",
            MSG::LexCompleteWithTokens => "lex_complete_tokens",
            MSG::LexTokenIdentifier => "lex_token_identifier",
            MSG::LexTokenKeyword => "lex_token_keyword",
            MSG::LexTokenNumber => "lex_token_number",
            MSG::LexTokenString => "lex_token_string",
            MSG::LexTokenChar => "lex_token_char",
            MSG::LexTokenOperator => "lex_token_operator",
            MSG::LexTokenPunctuation => "lex_token_punctuation",
            MSG::ParserStart => "parser_start",
            MSG::ParserComplete => "parser_complete",
            MSG::ParserCompleteWithItems => "parser_complete_items",
            MSG::ParserParseStmt => "parser_parse_stmt",
            MSG::ParserParseExpr => "parser_parse_expr",
            MSG::ParserParseFnDef => "parser_parse_fn_def",
            MSG::ParserParseLet => "parser_parse_let",
            MSG::ParserParseReturn => "parser_parse_return",
            MSG::ParserParseIf => "parser_parse_if",
            MSG::ParserParseLoop => "parser_parse_loop",
            MSG::ParserParseBlock => "parser_parse_block",
            MSG::TypeCheckStart => "typecheck_start",
            MSG::TypeCheckComplete => "typecheck_complete",
            MSG::TypeCheckProcessFn => "typecheck_process_fn",
            MSG::TypeCheckHasAnnotation => "typecheck_has_annotation",
            MSG::TypeCheckAnnotation => "typecheck_annotation",
            MSG::TypeCheckAnnotated => "typecheck_annotated",
            MSG::TypeCheckAddError => "typecheck_add_error",
            MSG::TypeCheckCallFnDef => "typecheck_call_fndef",
            MSG::TypeCheckInferExpr => "typecheck_infer_expr",
            MSG::TypeCheckInferFn => "typecheck_infer_fn",
            MSG::TypeCheckAddConstraint => "typecheck_add_constraint",
            MSG::TypeCheckSolveConstraints => "typecheck_solve_constraints",
            MSG::TypeCheckVarBinding => "typecheck_var_binding",
            MSG::CodegenStart => "codegen_start",
            MSG::CodegenComplete => "codegen_complete",
            MSG::CodegenFunctions => "codegen_functions",
            MSG::CodegenConstPool => "codegen_const_pool",
            MSG::CodegenCodeSection => "codegen_code_section",
            MSG::CodegenTypeTable => "codegen_type_table",
            MSG::CodegenGenFn => "codegen_gen_fn",
            MSG::CodegenGenBlock => "codegen_gen_block",
            MSG::CodegenGenInstr => "codegen_gen_instr",
            MSG::CodegenRegAlloc => "codegen_reg_alloc",
            MSG::CodegenAddConst => "codegen_add_const",
            MSG::VmStart => "vm_start",
            MSG::VmComplete => "vm_complete",
            MSG::VmExecuteFn => "vm_execute_fn",
            MSG::VmExecInstruction => "vm_exec_instruction",
            MSG::VmCallStack => "vm_call_stack",
            MSG::VmPushFrame => "vm_push_frame",
            MSG::VmPopFrame => "vm_pop_frame",
            MSG::VmLoadLocal => "vm_load_local",
            MSG::VmStoreLocal => "vm_store_local",
            MSG::VmLoadArg => "vm_load_arg",
            MSG::VmRegRead => "vm_reg_read",
            MSG::VmRegWrite => "vm_reg_write",
            MSG::VmPushStack => "vm_push_stack",
            MSG::VmPopStack => "vm_pop_stack",
            MSG::VmCallFunc => "vm_call_func",
            MSG::VmReturnFunc => "vm_return_func",
            MSG::VmBinaryOp => "vm_binary_op",
            MSG::VmI64Add => "vm_i64_add",
            MSG::VmExecutingFunction => "vm_executing_function",
            MSG::VmFunctionReturned => "vm_function_returned",
            MSG::VmStoringResult => "vm_storing_result",
            MSG::VmRegistersAfter => "vm_registers_after",
            MSG::CompilationStart => "compilation_start",
            MSG::CompilingSource => "compiling_source",
            MSG::DebugRunCalled => "debug_run_called",

            // Debug logging
            MSG::DebugCheckingStmt => "debug_checking_stmt",
            MSG::DebugStmtExpr => "debug_stmt_expr",
            MSG::DebugStmtFn => "debug_stmt_fn",
            MSG::DebugCheckingType => "debug_checking_type",
            MSG::DebugStructType => "debug_struct_type",
            MSG::DebugNonStructType => "debug_non_struct_type",
            MSG::DebugLoadingFunction => "debug_loading_function",
            MSG::DebugTotalFunctions => "debug_total_functions",
            MSG::DebugAvailableFunctions => "debug_available_functions",
            MSG::DebugFunctionLookup => "debug_function_lookup",
            MSG::DebugFunctionFound => "debug_function_found",
            MSG::DebugFunctionCall => "debug_function_call",
            MSG::DebugFunctionReturn => "debug_function_return",
            MSG::DebugExecBinaryOp => "debug_exec_binary_op",
            MSG::DebugAddingNumbers => "debug_adding_numbers",
            MSG::DebugStructTypeConstructorCall => "debug_struct_type_constructor_call",
            MSG::DebugTranslatingInstr => "debug_translating_instr",
            MSG::DebugGeneratingIRBinOp => "debug_generating_ir_binop",

            // Error messages
            MSG::ErrorUnknownVariable => "error_unknown_variable",
            MSG::ErrorUnknownType => "error_unknown_type",
            MSG::ErrorTypeMismatch => "error_type_mismatch",
            MSG::ErrorArityMismatch => "error_arity_mismatch",
            MSG::ErrorIndexOutOfBounds => "error_index_out_of_bounds",
            MSG::ErrorUnknownField => "error_unknown_field",
            MSG::ErrorRecursiveType => "error_recursive_type",
            MSG::ErrorUnsupportedOp => "error_unsupported_op",
            MSG::ErrorNonExhaustivePatterns => "error_non_exhaustive_patterns",
            MSG::ErrorImportError => "error_import_error",
            MSG::ErrorInferenceFailed => "error_inference_failed",
            MSG::ErrorCannotInferParamType => "error_cannot_infer_param_type",
            MSG::HelpDidYouMean => "help_did_you_mean",
            MSG::HelpSimilarVariables => "help_similar_variables",
            MSG::HelpInScope => "help_in_scope",

            // Bytecode dump messages
            MSG::BytecodeDumpHeader => "bytecode_dump_header",
            MSG::BytecodeDumpTypeTable => "bytecode_dump_type_table",
            MSG::BytecodeDumpConstants => "bytecode_dump_constants",
            MSG::BytecodeDumpFunctions => "bytecode_dump_functions",
            MSG::BytecodeFileHeader => "bytecode_file_header",
            MSG::BytecodeMagic => "bytecode_magic",
            MSG::BytecodeVersion => "bytecode_version",
            MSG::BytecodeFlags => "bytecode_flags",
            MSG::BytecodeEntryPoint => "bytecode_entry_point",
            MSG::BytecodeSectionCount => "bytecode_section_count",
            MSG::BytecodeFileSize => "bytecode_file_size",
            MSG::BytecodeTypeCount => "bytecode_type_count",
            MSG::BytecodeConstCount => "bytecode_const_count",
            MSG::BytecodeFuncCount => "bytecode_func_count",
            MSG::BytecodeFuncName => "bytecode_func_name",
            MSG::BytecodeFuncParams => "bytecode_func_params",
            MSG::BytecodeFuncReturnType => "bytecode_func_return_type",
            MSG::BytecodeFuncLocalCount => "bytecode_func_local_count",
            MSG::BytecodeFuncInstrCount => "bytecode_func_instr_count",
            MSG::BytecodeFuncCode => "bytecode_func_code",
            MSG::BytecodeInstrIndex => "bytecode_instr_index",
            MSG::BytecodeUnknownOpcode => "bytecode_unknown_opcode",

            // REPL and Shell messages
            MSG::ShellExecTime => "shell_exec_time",

            // Debugger messages
            MSG::DebuggerAtLocation => "debugger_at_location",
            MSG::DebuggerLocals => "debugger_locals",
            MSG::DebuggerCallStack => "debugger_call_stack",

            // REPL messages
            MSG::ReplWelcome => "repl_welcome",
            MSG::ReplHelp => "repl_help",
            MSG::ReplError => "repl_error",
            MSG::ReplUnknownCommand => "repl_unknown_command",
            MSG::ReplAvailableCommands => "repl_available_commands",
            MSG::ReplExitCommand => "repl_exit_command",
            MSG::ReplHelpCommand => "repl_help_command",
            MSG::ReplHistoryCommand => "repl_history_command",
            MSG::ReplClearCommand => "repl_clear_command",
            MSG::ReplValue => "repl_value",
            MSG::ReplPrompt => "repl_prompt",
            MSG::ReplHistoryEntry => "repl_history_entry",

            // Shell messages
            MSG::ShellWelcome => "shell_welcome",
            MSG::ShellHelp => "shell_help",
            MSG::ShellExiting => "shell_exiting",
            MSG::ShellError => "shell_error",
            MSG::ShellAvailableCommands => "shell_available_commands",
            MSG::ShellExitCommand => "shell_exit_command",
            MSG::ShellClearCommand => "shell_clear_command",
            MSG::ShellCdCommand => "shell_cd_command",
            MSG::ShellPwdCommand => "shell_pwd_command",
            MSG::ShellLsCommand => "shell_ls_command",
            MSG::ShellCodeCommands => "shell_code_commands",
            MSG::ShellRunCommand => "shell_run_command",
            MSG::ShellLoadCommand => "shell_load_command",
            MSG::ShellDebugCommand => "shell_debug_command",
            MSG::ShellBreakCommand => "shell_break_command",
            MSG::ShellReplCommand => "shell_repl_command",
            MSG::ShellOtherInput => "shell_other_input",
            MSG::ShellDebugStart => "shell_debug_start",
            MSG::ShellDebugCmd => "shell_debug_cmd",

            // Debug messages
            MSG::DebugBinaryOp => "debug_binary_op",
            MSG::DebugRegisters => "debug_registers",
            MSG::DebugMatch => "debug_match",

            // Other messages
            MSG::FormatterNotImplemented => "formatter_not_implemented",

            // Package manager - errors
            MSG::PackageErrorAlreadyExists => "package_error_already_exists",
            MSG::PackageErrorNotProject => "package_error_not_project",
            MSG::PackageErrorDepNotFound => "package_error_dep_not_found",
            MSG::PackageErrorDepAlreadyExists => "package_error_dep_already_exists",
            MSG::PackageErrorInvalidManifest => "package_error_invalid_manifest",
            MSG::PackageErrorIoError => "package_error_io_error",
            MSG::PackageErrorTomlParseError => "package_error_toml_parse_error",

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

            // Package manager - lock file
            MSG::PackageLockGenerated => "package_lock_generated",

            // Package manager - source resolver
            MSG::PackageInvalidVersion => "package_invalid_version",
            MSG::PackageInvalidMajorVersion => "package_invalid_major_version",

            // Package manager - update messages
            MSG::PackageUpdateFailed => "package_update_failed",
            MSG::PackageAlreadyUpToDate => "package_already_up_to_date",

            _ => "unknown_message",
        }
    }
}

#[cfg(test)]
mod tests;
