//! REPL Completer
//!
//! Provides completion candidates for rustyline based on REPL context.

use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use rustyline::completion::{Completer, Pair};
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;

use super::backend::REPLBackend;
use super::eval::Evaluator;

/// REPL Completer
///
/// Completes identifiers based on defined symbols in the REPL context.
pub struct ReplCompleter {
    /// Evaluator to get symbols from
    evaluator: Rc<RefCell<Evaluator>>,
    /// Keywords for YaoXiang
    keywords: Vec<&'static str>,
}

impl fmt::Debug for ReplCompleter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReplCompleter")
            .field("keywords", &self.keywords)
            .finish()
    }
}

impl ReplCompleter {
    /// Create a new completer
    pub fn new(evaluator: Rc<RefCell<Evaluator>>) -> Self {
        Self {
            evaluator,
            keywords: Self::yaoxiang_keywords(),
        }
    }

    /// Get YaoXiang keywords
    fn yaoxiang_keywords() -> Vec<&'static str> {
        vec![
            "let", "fn", "if", "else", "match", "for", "while", "return", "struct", "enum",
            "trait", "impl", "use", "mod", "pub", "true", "false", "nil", "break", "continue",
        ]
    }
}

impl Completer for ReplCompleter {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &rustyline::Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Self::Candidate>)> {
        // Get the word being completed
        let (start, word) =
            if let Some(i) = line[..pos].rfind(|c: char| !c.is_alphanumeric() && c != '_') {
                (i + 1, &line[i + 1..pos])
            } else {
                (0, &line[..pos])
            };

        if word.is_empty() {
            return Ok((start, Vec::new()));
        }

        let mut candidates = Vec::new();
        let evaluator = self.evaluator.borrow();

        // Add symbol completions from evaluator
        for sym in evaluator.get_symbols() {
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

impl Hinter for ReplCompleter {
    type Hint = String;
}

impl Highlighter for ReplCompleter {}

impl Validator for ReplCompleter {}

impl rustyline::Helper for ReplCompleter {}
