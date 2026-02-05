//! REPL Completer
//!
//! Provides completion candidates for rustyline based on REPL context.

use std::borrow::Cow;

use rustyline::completion::{Completer, Pair};

use crate::backends::dev::repl::backend_trait::REPLBackend;

/// REPL Completer
///
/// Completes identifiers based on defined symbols in the REPL context.
pub struct REPLCompleter<B: REPLBackend> {
    /// Backend to get symbols from
    backend: B,
    /// Keywords for YaoXiang
    keywords: Vec<&'static str>,
}

impl<B: REPLBackend> REPLCompleter<B> {
    /// Create a new completer
    pub fn new(backend: B) -> Self {
        Self {
            backend,
            keywords: Self::yaoxiang_keywords(),
        }
    }

    /// Get YaoXiang keywords
    fn yaoxiang_keywords() -> Vec<&'static str> {
        vec![
            "let", "fn", "if", "else", "match", "for", "while", "return",
            "struct", "enum", "trait", "impl", "use", "mod", "pub",
            "true", "false", "nil", "break", "continue",
        ]
    }
}

impl<B: REPLBackend + 'static> Completer for REPLCompleter<B> {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &rustyline::Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Self::Candidate>)> {
        // Get the word being completed
        let (start, word) = if let Some(i) = line[..pos].rfind(|c: char| !c.is_alphanumeric() && c != '_') {
            (i + 1, &line[i + 1..pos])
        } else {
            (0, &line[..pos])
        };

        if word.is_empty() {
            return Ok((start, Vec::new()));
        }

        let mut candidates = Vec::new();

        // Add symbol completions from backend
        for sym in self.backend.get_symbols() {
            if sym.name.starts_with(word) {
                candidates.push(Pair {
                    display: format!("{}: {}", sym.name, sym.type_signature),
                    replacement: sym.name.clone(),
                });
            }
        }

        // Add keyword completions
        for kw in &self.keywords {
            if kw.starts_with(word) {
                candidates.push(Pair {
                    display: kw.to_string(),
                    replacement: kw.to_string(),
                });
            }
        }

        // Add builtin functions
        let builtins = ["print", "len", "range", "typeof", "assert"];
        for builtin in &builtins {
            if builtin.starts_with(word) {
                candidates.push(Pair {
                    display: format!("builtin {}", builtin),
                    replacement: builtin.to_string(),
                });
            }
        }

        // Sort and deduplicate
        candidates.sort_by(|a, b| a.replacement.cmp(&b.replacement));
        candidates.dedup_by(|a, b| a.replacement == b.replacement);

        Ok((start, candidates))
    }
}
