//! REPL Command Handler
//!
//! Handles special commands starting with ':'.

use super::backend_trait::REPLBackend;

/// Command result
#[derive(Debug)]
pub enum CommandResult {
    /// Exit the REPL
    Exit,
    /// Continue to next input
    Continue,
    /// Output a message
    Output(String),
}

/// Command handler for REPL
pub struct CommandHandler<'a, B: REPLBackend> {
    backend: &'a mut B,
}

impl<'a, B: REPLBackend> CommandHandler<'a, B> {
    /// Create a new command handler
    pub fn new(backend: &'a mut B) -> Self {
        Self { backend }
    }

    /// Handle a command
    pub fn handle(
        &mut self,
        line: &str,
    ) -> Option<CommandResult> {
        let cmd = line.trim_start_matches(':').trim();
        let parts: Vec<&str> = cmd.split_whitespace().collect();

        match parts[0] {
            "quit" | "q" => Some(CommandResult::Exit),
            "help" | "h" => {
                self.print_help();
                Some(CommandResult::Continue)
            }
            "clear" | "c" => {
                self.backend.clear();
                println!("Context cleared");
                Some(CommandResult::Continue)
            }
            "type" | "t" => {
                if let Some(name) = parts.get(1) {
                    if let Some(ty) = self.backend.get_type(name) {
                        println!("{}: {}", name, ty);
                    } else {
                        println!("Unknown symbol: {}", name);
                    }
                } else {
                    println!("Usage: :type <name>");
                }
                Some(CommandResult::Continue)
            }
            "symbols" | "info" | "i" => {
                for sym in self.backend.get_symbols() {
                    println!("{}: {}", sym.name, sym.type_signature);
                }
                Some(CommandResult::Continue)
            }
            "history" | "hist" => {
                println!("History command not yet implemented");
                Some(CommandResult::Continue)
            }
            "stats" => {
                let stats = self.backend.stats();
                println!("Eval count: {}", stats.eval_count);
                println!("Total time: {:?}", stats.total_time);
                Some(CommandResult::Continue)
            }
            "" => Some(CommandResult::Continue),
            _ => {
                println!("Unknown command: {}", line);
                Some(CommandResult::Continue)
            }
        }
    }

    /// Print help message
    fn print_help(&self) {
        println!("Available commands:");
        println!("  :quit, :q      - Exit the REPL");
        println!("  :help, :h      - Show this help");
        println!("  :clear, :c     - Clear all state");
        println!("  :type, :t <n>  - Show type of symbol");
        println!("  :symbols, :i   - List all symbols");
        println!("  :history, :hist - Show command history");
        println!("  :stats         - Show execution statistics");
    }
}
