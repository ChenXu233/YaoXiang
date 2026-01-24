//! 历史屏幕
//!
//! 浏览和管理历史记录

use crossterm::event::KeyCode;

/// 历史屏幕
#[derive(Debug)]
pub struct HistoryScreen {
    /// 搜索查询
    search_query: String,
    /// 筛选模式
    filter_mode: bool,
}

impl HistoryScreen {
    /// 创建新的历史屏幕
    pub fn new() -> Self {
        Self {
            search_query: String::new(),
            filter_mode: false,
        }
    }

    /// 处理按键事件
    pub fn handle_key_event(
        &mut self,
        key_code: KeyCode,
        _input_buffer: &mut String,
    ) {
        match key_code {
            KeyCode::Char('/') => {
                self.filter_mode = !self.filter_mode;
            }
            KeyCode::Backspace if self.filter_mode => {
                self.search_query.pop();
            }
            KeyCode::Char(c) if self.filter_mode => {
                self.search_query.push(c);
            }
            KeyCode::Esc => {
                self.filter_mode = false;
                self.search_query.clear();
            }
            _ => {}
        }
    }

    /// 获取搜索查询
    pub fn get_search_query(&self) -> &str {
        &self.search_query
    }

    /// 是否在筛选模式
    pub fn is_filtering(&self) -> bool {
        self.filter_mode
    }
}

impl Default for HistoryScreen {
    fn default() -> Self {
        Self::new()
    }
}
