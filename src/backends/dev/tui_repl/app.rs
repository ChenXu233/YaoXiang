//! 应用程序状态管理
//!
//! 管理 TUI REPL 的整体状态、屏幕切换和事件处理

use std::collections::HashMap;
use std::sync::Arc;

use crossterm::event::KeyCode;
use ratatui::{
    layout::{Constraint, Direction, Layout, Margin},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::backends::dev::tui_repl::components::{HistoryPanel, InputWindow, DebugPanel, OutputConsole};
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
    /// 是否显示调试信息
    show_debug: bool,
    /// 历史面板
    history_panel: HistoryPanel,
    /// 输入窗口
    input_window: InputWindow,
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
            show_debug: true,
            history_panel: HistoryPanel::new(),
            input_window: InputWindow::new(),
            debug_panel: DebugPanel::new(),
            output_console: OutputConsole::new(),
        }
    }

    /// 处理按键事件
    pub fn handle_key_event(
        &mut self,
        key_code: KeyCode,
    ) -> Option<Action> {
        match key_code {
            // 功能键
            KeyCode::F(1) => Some(Action::SwitchScreen(ScreenId::Help)),
            KeyCode::F(2) => Some(Action::Clear),
            KeyCode::F(3) => Some(Action::SwitchScreen(ScreenId::History)),
            KeyCode::F(4) => Some(Action::SwitchScreen(ScreenId::Debug)),

            // 通用快捷键
            KeyCode::Char('q') => Some(Action::Quit),
            KeyCode::Char('c') if self.current_screen == ScreenId::Main => {
                self.input_buffer.clear();
                Some(Action::Clear)
            }
            KeyCode::Enter => Some(Action::Execute),
            KeyCode::Esc => {
                self.current_screen = ScreenId::Main;
                Some(Action::None)
            }

            // 当前屏幕的特殊处理
            _ => {
                if let Some(screen) = self.screens.get_mut(&self.current_screen) {
                    screen.handle_key_event(key_code, &mut self.input_buffer);
                }
                Some(Action::None)
            }
        }
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
                Constraint::Length(3), // 输入区域
            ])
            .split(area);

        // 渲染标题栏
        self.render_title_bar(f, chunks[0]);

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

            // 渲染主内容
            self.render_main_content(f, main_chunks[1], compiler);

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
            self.render_main_content(f, main_chunks[1], compiler);
        }

        // 渲染输入区域
        self.input_window.render(f, chunks[2], &self.input_buffer);
    }

    /// 渲染标题栏
    fn render_title_bar(
        &self,
        f: &mut Frame<'_>,
        area: ratatui::layout::Rect,
    ) {
        let title = " YaoXiang REPL v0.3.6 - [F1:Help F2:Clear F3:History F4:Debug] ";

        f.render_widget(
            Paragraph::new(title)
                .style(Style::default().bg(Color::Blue).fg(Color::White))
                .alignment(ratatui::layout::Alignment::Center),
            area,
        );
    }

    /// 渲染主内容区域
    fn render_main_content(
        &mut self,
        f: &mut Frame<'_>,
        area: ratatui::layout::Rect,
        _compiler: &Arc<IncrementalCompiler>,
    ) {
        // 清除区域
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Input & Output ");

        f.render_widget(block, area);

        // 内容区域
        let inner_area = area.inner(&Margin::default());
        let content_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(70), // 输出区域
                Constraint::Percentage(30), // 输入预览
            ])
            .split(inner_area);

        // 渲染输出控制台
        self.output_console.render(f, content_chunks[0]);

        // 渲染输入预览
        f.render_widget(
            Paragraph::new(format!("Input: {}", self.input_buffer)).scroll((0, 0)),
            content_chunks[1],
        );
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
        key_code: KeyCode,
        input_buffer: &mut String,
    ) {
        match self {
            Screen::Main(screen) => screen.handle_key_event(key_code, input_buffer),
            Screen::History(screen) => screen.handle_key_event(key_code, input_buffer),
            Screen::Debug(screen) => screen.handle_key_event(key_code, input_buffer),
            Screen::Help(screen) => screen.handle_key_event(key_code, input_buffer),
        }
    }
}
