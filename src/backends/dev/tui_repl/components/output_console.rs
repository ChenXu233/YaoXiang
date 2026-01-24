/// 输出控制台组件
///
/// 显示程序输出和错误信息
use ratatui::{
    layout::{Rect, Margin},
    style::{Color, Style},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

/// 输出条目
#[derive(Debug, Clone)]
pub enum OutputEntry {
    /// 正常输出
    Output(String),
    /// 错误信息
    Error(String),
    /// 警告信息
    Warning(String),
    /// 信息
    Info(String),
}

/// 输出控制台
pub struct OutputConsole {
    /// 输出条目列表
    entries: Vec<OutputEntry>,
    /// 最大条目数
    max_entries: usize,
    /// 当前滚动位置
    scroll: u16,
}

impl OutputConsole {
    /// 创建新的输出控制台
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            max_entries: 1000,
            scroll: 0,
        }
    }

    /// 添加输出
    pub fn add_output(
        &mut self,
        output: String,
    ) {
        self.entries.push(OutputEntry::Output(output));
        self.trim_entries();
    }

    /// 添加错误
    pub fn add_error(
        &mut self,
        error: String,
    ) {
        self.entries.push(OutputEntry::Error(error));
        self.trim_entries();
    }

    /// 添加警告
    pub fn add_warning(
        &mut self,
        warning: String,
    ) {
        self.entries.push(OutputEntry::Warning(warning));
        self.trim_entries();
    }

    /// 添加信息
    pub fn add_info(
        &mut self,
        info: String,
    ) {
        self.entries.push(OutputEntry::Info(info));
        self.trim_entries();
    }

    /// 清空输出
    pub fn clear(&mut self) {
        self.entries.clear();
        self.scroll = 0;
    }

    /// 向上滚动
    pub fn scroll_up(&mut self) {
        if self.scroll > 0 {
            self.scroll -= 1;
        }
    }

    /// 向下滚动
    pub fn scroll_down(&mut self) {
        if self.scroll < self.entries.len().saturating_sub(1) as u16 {
            self.scroll += 1;
        }
    }

    /// 重置滚动位置
    pub fn reset_scroll(&mut self) {
        self.scroll = 0;
    }

    /// 修剪条目
    fn trim_entries(&mut self) {
        if self.entries.len() > self.max_entries {
            self.entries.drain(0..self.entries.len() - self.max_entries);
        }
    }

    /// 渲染输出控制台
    pub fn render(
        &self,
        f: &mut Frame<'_>,
        area: Rect,
    ) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Plain)
            .title(" Output ");

        f.render_widget(block, area);

        let inner_area = area.inner(&Margin::default());

        if self.entries.is_empty() {
            return;
        }

        // 创建输出内容
        let content = self
            .entries
            .iter()
            .map(|entry| match entry {
                OutputEntry::Output(text) => text.to_string(),
                OutputEntry::Error(text) => {
                    format!("ERROR: {}", text)
                }
                OutputEntry::Warning(text) => {
                    format!("WARNING: {}", text)
                }
                OutputEntry::Info(text) => {
                    format!("INFO: {}", text)
                }
            })
            .collect::<Vec<_>>()
            .join("\n");

        let paragraph = Paragraph::new(content)
            .style(Style::default().fg(Color::White))
            .scroll((self.scroll, 0));

        f.render_widget(paragraph, inner_area);
    }

    /// 获取条目数量
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 检查是否为空
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Default for OutputConsole {
    fn default() -> Self {
        Self::new()
    }
}
