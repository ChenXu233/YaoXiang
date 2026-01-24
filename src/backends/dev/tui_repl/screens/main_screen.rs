//! 主屏幕
//!
//! REPL 的主输入和输出界面

use crossterm::event::{KeyCode, KeyEvent};

/// 主屏幕
#[derive(Debug)]
pub struct MainScreen {
    /// 是否正在输入多行代码
    multi_line_mode: bool,
}

impl MainScreen {
    /// 创建新的主屏幕
    pub fn new() -> Self {
        Self {
            multi_line_mode: false,
        }
    }

    /// 处理按键事件
    pub fn handle_key_event(
        &mut self,
        key: KeyEvent,
        input_buffer: &mut String,
    ) {
        match key.code {
            // 功能键
            KeyCode::Tab => {
                // 自动补全
            }
            KeyCode::Char('\\') => {
                // 多行模式切换
                self.multi_line_mode = !self.multi_line_mode;
            }
            KeyCode::Backspace => {
                // 删除字符
                input_buffer.pop();
            }
            KeyCode::Left => {
                // 光标向左移动 - 由于 input_window 渲染时使用的是 input_buffer
                // 且没有 synchronized cursor pos, 这里暂时不处理或者只做删除是不对的
                // 为了避免误操作，这里暂时不做任何事情，或者需要实现真正的光标移动
            }
            KeyCode::Right => {
                // 光标向右移动
            }

            // 处理普通字符输入
            KeyCode::Char(ch) => {
                input_buffer.push(ch);
            }

            _ => {}
        }
    }
}

impl Default for MainScreen {
    fn default() -> Self {
        Self::new()
    }
}
