//! 主屏幕
//!
//! REPL 的主输入和输出界面

use crossterm::event::KeyCode;

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
        key_code: KeyCode,
        input_buffer: &mut String,
    ) {
        match key_code {
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
                // 光标向左移动（简化处理）
                if !input_buffer.is_empty() {
                    input_buffer.remove(input_buffer.len().saturating_sub(1));
                }
            }
            KeyCode::Right => {
                // 光标向右移动（暂时忽略）
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
