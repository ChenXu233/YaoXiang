//! 帮助屏幕
//!
//! 显示帮助文档和快捷键参考

use crossterm::event::{KeyCode, KeyEvent};

/// 帮助屏幕
#[derive(Debug)]
pub struct HelpScreen {
    /// 当前页面
    current_page: HelpPage,
}

impl HelpScreen {
    /// 创建新的帮助屏幕
    pub fn new() -> Self {
        Self {
            current_page: HelpPage::Overview,
        }
    }

    /// 处理按键事件
    pub fn handle_key_event(
        &mut self,
        key: KeyEvent,
        _input_buffer: &mut String,
    ) {
        match key.code {
            KeyCode::Char('1') => self.current_page = HelpPage::Overview,
            KeyCode::Char('2') => self.current_page = HelpPage::Shortcuts,
            KeyCode::Char('3') => self.current_page = HelpPage::Commands,
            KeyCode::Char('4') => self.current_page = HelpPage::Examples,
            _ => {}
        }
    }

    /// 获取帮助内容
    pub fn get_help_content(&self) -> &'static str {
        match self.current_page {
            HelpPage::Overview => HELP_OVERVIEW,
            HelpPage::Shortcuts => HELP_SHORTCUTS,
            HelpPage::Commands => HELP_COMMANDS,
            HelpPage::Examples => HELP_EXAMPLES,
        }
    }

    /// 获取当前页面
    pub fn get_current_page(&self) -> HelpPage {
        self.current_page
    }
}

impl Default for HelpScreen {
    fn default() -> Self {
        Self::new()
    }
}

/// 帮助页面
#[derive(Debug, Clone, Copy)]
pub enum HelpPage {
    Overview,
    Shortcuts,
    Commands,
    Examples,
}

/// 帮助内容
const HELP_OVERVIEW: &str = r#"YaoXiang REPL v0.3.6

Welcome to the YaoXiang interactive programming environment!

This REPL provides:
• Interactive code execution
• Multi-line input support
• Command history
• Debug information
• Performance profiling

Quick Start:
1. Type YaoXiang code at the >> prompt
2. Press Enter to execute
3. Use F3 to browse history
4. Use F4 to view debug info

Use F1 for help, F2 to clear, F3 for history, F4 for debug.
"#;

const HELP_SHORTCUTS: &str = r#"Keyboard Shortcuts

Function Keys:
F1       Show help
F2       Clear input
F3       History browser
F4       Debug panel

Editing:
Tab      Auto-complete
Enter    Execute code
Esc      Cancel multi-line input
Backspace Delete character

Navigation:
Ctrl+A   Move to line start
Ctrl+E   Move to line end
Ctrl+U   Clear line

History:
↑        Previous command
↓        Next command
Ctrl+R   Search history

Debug:
F4       Toggle debug panel
Tab      Switch debug tab (in debug mode)
"#;

const HELP_COMMANDS: &str = r#"Special Commands

:quit, :q          Exit the REPL
:clear, :c         Clear input buffer
:help, :h          Show this help
:history, :hist    Show command history
:reset             Reset compiler state

Debug Commands:
:break <line>      Set breakpoint
:step             Step execution
:continue         Continue execution
:print <var>      Print variable value
:inspect <expr>   Inspect expression

Performance:
:profile          Show performance profile
:time <expr>      Measure execution time
:memory           Show memory usage

Advanced:
:save <path>      Save session to file
:load <path>      Load session from file
:import <module>  Import module
:explain <expr>   Explain expression
"#;

const HELP_EXAMPLES: &str = r#"Code Examples

Variable Declaration:
let x = 42
let name = "YaoXiang"
let list = [1, 2, 3, 4, 5]

Function Definition:
fn greet(name) {
    print("Hello, " + name + "!")
}

Control Flow:
if x > 10 {
    print("x is greater than 10")
} else {
    print("x is not greater than 10")
}

Loops:
for i in 0..10 {
    print(i)
}

Types:
struct Point {
    x: Int,
    y: Int,
}

let p = Point { x: 10, y: 20 }
print(p.x)
"#;
