//! 应用程序状态管理
//!
//! 管理 TUI REPL 的整体状态、屏幕切换和事件处理

use std::collections::HashMap;
use std::sync::Arc;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style, Modifier},
    text::Span,
    widgets::{Paragraph, Clear, Block, Borders},
    Frame,
};

use crate::backends::dev::tui_repl::components::{HistoryPanel, DebugPanel, OutputConsole};
use crate::backends::dev::tui_repl::engine::IncrementalCompiler;
use crate::backends::dev::tui_repl::screens::{MainScreen, HistoryScreen, DebugScreen, HelpScreen};

/// 屏幕标识符
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScreenId {
    Main,
    History,
    Debug,
    Help,
}

/// 用户动作
#[derive(Debug, Clone)]
pub enum Action {
    None,
    Quit,
    SwitchScreen(ScreenId),
    Execute,
    Clear,
    ToggleDebug,
}

/// 应用程序状态
pub struct App {
    /// 当前屏幕
    current_screen: ScreenId,
    /// 屏幕映射
    screens: HashMap<ScreenId, Screen>,
    /// 输入缓冲区
    input_buffer: String,
    /// 是否在多行输入模式
    continuation_mode: bool,
    /// 是否显示调试信息
    show_debug: bool,
    /// 历史面板
    history_panel: HistoryPanel,
    /// 调试面板
    debug_panel: DebugPanel,
    /// 输出控制台
    output_console: OutputConsole,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    /// 创建新的应用
    pub fn new() -> Self {
        let mut screens = HashMap::new();
        screens.insert(ScreenId::Main, Screen::Main(MainScreen::new()));
        screens.insert(ScreenId::History, Screen::History(HistoryScreen::new()));
        screens.insert(ScreenId::Debug, Screen::Debug(DebugScreen::new()));
        screens.insert(ScreenId::Help, Screen::Help(HelpScreen::new()));

        Self {
            current_screen: ScreenId::Main,
            screens,
            input_buffer: String::new(),
            continuation_mode: false,
            show_debug: true,
            history_panel: HistoryPanel::new(),
            debug_panel: DebugPanel::new(),
            output_console: OutputConsole::new(),
        }
    }

    /// 处理按键事件
    pub fn handle_key_event(
        &mut self,
        key: KeyEvent,
    ) -> Option<Action> {
        match key.code {
            // 功能键
            KeyCode::F(1) => Some(Action::SwitchScreen(ScreenId::Help)),
            KeyCode::F(2) => Some(Action::Clear),
            KeyCode::F(3) => Some(Action::SwitchScreen(ScreenId::History)),
            KeyCode::F(4) => Some(Action::ToggleDebug),

            // 通用快捷键
            KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Some(Action::Quit)
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Ctrl+C clear or interrupt
                self.input_buffer.clear();
                Some(Action::Clear)
            }
            KeyCode::Enter => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    self.input_buffer.push('\n');
                    Some(Action::None)
                } else {
                    let input = self.input_buffer.trim();
                    match input {
                        ":q" | ":quit" | ":exit" | ":q!" | ":quit!" => Some(Action::Quit),
                        _ => Some(Action::Execute),
                    }
                }
            }
            KeyCode::Esc => {
                self.current_screen = ScreenId::Main;
                Some(Action::None)
            }

            // 当前屏幕的特殊处理
            _ => {
                if let Some(screen) = self.screens.get_mut(&self.current_screen) {
                    screen.handle_key_event(key, &mut self.input_buffer);
                }
                Some(Action::None)
            }
        }
    }

    /// 执行输入
    pub fn execute_input(
        &mut self,
        compiler: &Arc<IncrementalCompiler>,
    ) {
        let input = self.input_buffer.trim_end();
        if input.is_empty() && !self.continuation_mode {
            self.input_buffer.clear();
            return;
        }

        // 添加到历史
        let input_clone = input.to_string();

        // 显示当前行，如果是 continuation，就用 ... 否则用 >>>
        let prompt = if self.continuation_mode {
            "... "
        } else {
            ">>> "
        };
        self.output_console
            .add_info(format!("{}{}", prompt, input_clone));

        let mut entry_output = None;

        // 编译/执行
        match compiler.compile(input) {
            Ok(result) => {
                if result.need_more_input {
                    self.continuation_mode = true;
                } else if result.success {
                    self.continuation_mode = false;
                    // 获取实际输出
                    let output = compiler.read_stdout();
                    if !output.is_empty() {
                        self.output_console.add_output(output.clone());
                        entry_output = Some(output);
                    }
                } else {
                    self.continuation_mode = false;
                    if let Some(err) = result.error {
                        self.output_console.add_error(err);
                    }
                }
            }
            Err(e) => {
                self.continuation_mode = false;
                self.output_console.add_error(e.to_string());
            }
        }

        use crate::backends::dev::tui_repl::components::history_panel::HistoryEntry;
        self.history_panel.add_entry(HistoryEntry {
            input: input_clone,
            output: entry_output,
            timestamp: std::time::SystemTime::now(),
            duration: std::time::Duration::from_millis(0), // TODO: Measure time
        });

        // 清空输入
        self.input_buffer.clear();
    }

    /// 渲染应用
    pub fn render(
        &mut self,
        f: &mut Frame<'_>,
        area: ratatui::layout::Rect,
        compiler: &Arc<IncrementalCompiler>,
    ) {
        // 创建主布局
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // 标题栏
                Constraint::Min(0),    // 主内容区域
            ])
            .split(area);

        // 渲染标题栏
        self.render_title_bar(f, chunks[0]);

        // Handle Help Screen
        if self.current_screen == ScreenId::Help {
            let help_text = if let Some(Screen::Help(s)) = self.screens.get(&ScreenId::Help) {
                s.get_help_content()
            } else {
                "Help not available"
            };
            let block = Block::default()
                .title(" Help (Press ESC to return) ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan));
            let p = Paragraph::new(help_text).block(block);
            f.render_widget(Clear, chunks[1]);
            f.render_widget(p, chunks[1]);
            return;
        }

        // REPL prompt
        let prompt = if self.continuation_mode {
            "... "
        } else {
            ">>> "
        };

        if self.show_debug {
            // 三栏布局：历史 | 内容 | 调试
            let main_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(25), // 历史面板
                    Constraint::Percentage(50), // 主内容
                    Constraint::Percentage(25), // 调试面板
                ])
                .split(chunks[1]);

            // 渲染历史面板
            self.history_panel.render(f, main_chunks[0], compiler);

            // 渲染主内容(Integrated Terminal)
            self.output_console
                .render(f, main_chunks[1], prompt, &self.input_buffer);

            // 渲染调试面板
            self.debug_panel.render(f, main_chunks[2], compiler);
        } else {
            // 两栏布局：历史 | 内容
            let main_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(30), // 历史面板
                    Constraint::Percentage(70), // 主内容
                ])
                .split(chunks[1]);

            // 渲染历史面板
            self.history_panel.render(f, main_chunks[0], compiler);

            // 渲染主内容
            self.output_console
                .render(f, main_chunks[1], prompt, &self.input_buffer);
        }
    }

    /// 渲染标题栏
    fn render_title_bar(
        &self,
        f: &mut Frame<'_>,
        area: ratatui::layout::Rect,
    ) {
        let title = "  YaoXiang REPL v0.3.6 (Experimental)  ";
        let help = "  F1:Help | F2:Clear | F3:History | F4:Debug | Tab:Complete  ";

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(0), Constraint::Length(help.len() as u16)])
            .split(area);

        f.render_widget(
            Paragraph::new(Span::styled(
                title,
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ))
            .style(Style::default().bg(Color::Rgb(50, 50, 50)))
            .alignment(ratatui::layout::Alignment::Left),
            chunks[0],
        );

        f.render_widget(
            Paragraph::new(Span::styled(help, Style::default().fg(Color::Cyan)))
                .style(Style::default().bg(Color::Rgb(50, 50, 50)))
                .alignment(ratatui::layout::Alignment::Right),
            chunks[1],
        );
    }

    // render_main_content removed (replaced by direct output_console.render in render())

    /// Switch to a screen
    pub fn switch_screen(
        &mut self,
        screen_id: ScreenId,
    ) {
        self.current_screen = screen_id;
    }

    /// Clear output console
    pub fn clear_output(&mut self) {
        // We can expose a clear method on OutputConsole if needed,
        // or just recreate it. For now let's just create a new one to be safe/simple.
        self.output_console = OutputConsole::new();
        self.input_buffer.clear();
    }

    /// Toggle debug panel
    pub fn toggle_debug(&mut self) {
        self.show_debug = !self.show_debug;
    }
}

/// 屏幕枚举
#[derive(Debug)]
pub enum Screen {
    Main(MainScreen),
    History(HistoryScreen),
    Debug(DebugScreen),
    Help(HelpScreen),
}

impl Screen {
    pub fn handle_key_event(
        &mut self,
        key: KeyEvent,
        input_buffer: &mut String,
    ) {
        match self {
            Screen::Main(screen) => screen.handle_key_event(key, input_buffer),
            Screen::History(screen) => screen.handle_key_event(key, input_buffer),
            Screen::Debug(screen) => screen.handle_key_event(key, input_buffer),
            Screen::Help(screen) => screen.handle_key_event(key, input_buffer),
        }
    }
}
