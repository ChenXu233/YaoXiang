//! 调试屏幕
//!
//! 显示详细的调试信息

use crossterm::event::{KeyCode, KeyEvent};

/// 调试屏幕
#[derive(Debug)]
pub struct DebugScreen {
    /// 选中的标签页
    selected_tab: DebugTab,
}

impl DebugScreen {
    /// 创建新的调试屏幕
    pub fn new() -> Self {
        Self {
            selected_tab: DebugTab::CallStack,
        }
    }

    /// 处理按键事件
    pub fn handle_key_event(
        &mut self,
        key: KeyEvent,
        _input_buffer: &mut String,
    ) {
        match key.code {
            KeyCode::Tab | KeyCode::Right => {
                self.next_tab();
            }
            KeyCode::BackTab | KeyCode::Left => {
                self.prev_tab();
            }
            _ => {}
        }
    }

    /// 切换到下一个标签页
    pub fn next_tab(&mut self) {
        self.selected_tab = match self.selected_tab {
            DebugTab::CallStack => DebugTab::Variables,
            DebugTab::Variables => DebugTab::Performance,
            DebugTab::Performance => DebugTab::Memory,
            DebugTab::Memory => DebugTab::CallStack,
        };
    }

    /// 切换到上一个标签页
    pub fn prev_tab(&mut self) {
        self.selected_tab = match self.selected_tab {
            DebugTab::CallStack => DebugTab::Memory,
            DebugTab::Variables => DebugTab::CallStack,
            DebugTab::Performance => DebugTab::Variables,
            DebugTab::Memory => DebugTab::Performance,
        };
    }

    /// 获取选中的标签页
    pub fn get_selected_tab(&self) -> DebugTab {
        self.selected_tab
    }
}

impl Default for DebugScreen {
    fn default() -> Self {
        Self::new()
    }
}

/// 调试标签页
#[derive(Debug, Clone, Copy)]
pub enum DebugTab {
    CallStack,
    Variables,
    Performance,
    Memory,
}
