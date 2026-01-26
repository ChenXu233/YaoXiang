//! TUI REPL 主框架
//!
//! 使用 ratatui 实现的现代化终端用户界面 REPL

use std::io;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use crate::backends::dev::tui_repl::app::{App, Action};
use crate::backends::dev::tui_repl::engine::IncrementalCompiler;
use crate::Result;

/// TUI REPL 主结构
pub struct TuiREPL {
    /// 终端实例
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    /// 应用程序状态
    app: App,
    /// 增量编译器
    compiler: Arc<IncrementalCompiler>,
}

impl TuiREPL {
    /// 创建新的 TUI REPL
    pub fn new() -> Result<Self> {
        // 设置终端
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        // 创建增量编译器
        let compiler = Arc::new(IncrementalCompiler::new()?);

        // 创建应用
        let app = App::new();

        Ok(Self {
            terminal,
            app,
            compiler,
        })
    }

    /// 运行 TUI REPL
    pub fn run(&mut self) -> Result<()> {
        let tick_rate = Duration::from_millis(250);
        let mut last_tick = Instant::now();

        loop {
            // 计算时间差
            let delta = last_tick.elapsed();

            // 绘制界面
            self.terminal.draw(|f| {
                self.app.render(f, f.area(), &self.compiler);
            })?;

            // 检查是否超时
            if delta >= tick_rate {
                last_tick = Instant::now();
            }

            // 处理事件 - 计算等待时间，避免负数
            let wait_time = if delta >= tick_rate {
                Duration::from_millis(0)
            } else {
                tick_rate - delta
            };

            if event::poll(wait_time)? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        // 处理按键事件
                        match self.app.handle_key_event(key) {
                            Some(Action::Quit) => return self.quit(),
                            Some(Action::Execute) => self.app.execute_input(&self.compiler),
                            Some(Action::SwitchScreen(id)) => self.app.switch_screen(id),
                            Some(Action::Clear) => self.app.clear_output(),
                            Some(Action::ToggleDebug) => self.app.toggle_debug(),
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    /// 退出 REPL
    fn quit(&mut self) -> Result<()> {
        disable_raw_mode()?;
        execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        self.terminal.show_cursor()?;
        Ok(())
    }
}

impl Drop for TuiREPL {
    fn drop(&mut self) {
        let _ = self.quit();
    }
}

impl Default for TuiREPL {
    fn default() -> Self {
        Self::new().expect("Failed to create TUI REPL")
    }
}
