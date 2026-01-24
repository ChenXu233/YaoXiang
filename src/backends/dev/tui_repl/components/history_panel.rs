use std::sync::Arc;
/// 历史面板组件
///
/// 显示和管理 REPL 历史记录
use ratatui::{
    layout::{Rect, Margin},
    style::{Color, Style},
    widgets::{Block, BorderType, Borders, List, ListItem},
    Frame,
};

use crate::backends::dev::tui_repl::engine::IncrementalCompiler;

/// 历史条目
#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub input: String,
    pub output: Option<String>,
    pub timestamp: std::time::SystemTime,
    pub duration: std::time::Duration,
}

/// 历史面板
pub struct HistoryPanel {
    /// 历史条目列表
    entries: Vec<HistoryEntry>,
    /// 当前选中的索引
    selected_idx: usize,
    /// 搜索过滤器
    filter: Option<String>,
}

impl HistoryPanel {
    /// 创建新的历史面板
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            selected_idx: 0,
            filter: None,
        }
    }

    /// 添加历史条目
    pub fn add_entry(
        &mut self,
        entry: HistoryEntry,
    ) {
        self.entries.push(entry);
    }

    /// 设置搜索过滤器
    pub fn set_filter(
        &mut self,
        filter: Option<String>,
    ) {
        self.filter = filter;
    }

    /// 清空历史
    pub fn clear(&mut self) {
        self.entries.clear();
        self.selected_idx = 0;
    }

    /// 获取当前选中的条目
    pub fn get_selected(&self) -> Option<&HistoryEntry> {
        self.entries.get(self.selected_idx)
    }

    /// 向上移动选择
    pub fn move_up(&mut self) {
        if self.selected_idx > 0 {
            self.selected_idx -= 1;
        }
    }

    /// 向下移动选择
    pub fn move_down(&mut self) {
        if self.selected_idx < self.entries.len().saturating_sub(1) {
            self.selected_idx += 1;
        }
    }

    /// 渲染历史面板
    pub fn render(
        &self,
        f: &mut Frame<'_>,
        area: Rect,
        _compiler: &Arc<IncrementalCompiler>,
    ) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Plain)
            .title(" History ");

        f.render_widget(block, area);

        let inner_area = area.inner(&Margin::default());

        if self.entries.is_empty() {
            return;
        }

        // 创建历史条目列表
        let items: Vec<ListItem<'_>> = self
            .entries
            .iter()
            .enumerate()
            .map(|(i, entry)| {
                let style = if i == self.selected_idx {
                    Style::default().bg(Color::Blue).fg(Color::White)
                } else {
                    Style::default()
                };

                let content = if let Some(output) = &entry.output {
                    format!("> {}  =>  {}", entry.input, output)
                } else {
                    format!("> {}", entry.input)
                };

                ListItem::new(content).style(style)
            })
            .collect();

        let list = List::new(items).block(Block::default().borders(Borders::BOTTOM));

        f.render_widget(list, inner_area);
    }

    /// 获取历史条目数量
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 检查是否为空
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Default for HistoryPanel {
    fn default() -> Self {
        Self::new()
    }
}
