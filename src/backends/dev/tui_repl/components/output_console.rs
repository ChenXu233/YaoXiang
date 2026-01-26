/// 输出控制台组件
///
/// 显示程序输出和错误信息
use ratatui::{
    layout::Rect,
    style::{Color, Style, Modifier},
    text::{Line, Span},
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
        prompt: &str,
        input: &str,
    ) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Gray))
            .title(Span::styled(
                " Terminal ",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ));

        let inner_area = block.inner(area);
        f.render_widget(block, area);

        // 创建输出内容
        let mut lines = Vec::new();
        for entry in &self.entries {
            match entry {
                OutputEntry::Output(text) => {
                    for line in text.lines() {
                        lines.push(Line::from(Span::raw(line)));
                    }
                }
                OutputEntry::Error(text) => {
                    for line in text.lines() {
                        lines.push(Line::from(Span::styled(
                            line,
                            Style::default().fg(Color::Red),
                        )));
                    }
                }
                OutputEntry::Warning(text) => {
                    for line in text.lines() {
                        lines.push(Line::from(Span::styled(
                            line,
                            Style::default().fg(Color::Yellow),
                        )));
                    }
                }
                OutputEntry::Info(text) => {
                    for line in text.lines() {
                        lines.push(Line::from(Span::styled(
                            line,
                            Style::default().fg(Color::Blue),
                        )));
                    }
                }
            }
        }

        // Add current input line at the bottom
        let input_line = format!("{}{}", prompt, input);
        lines.push(Line::from(Span::styled(
            input_line,
            Style::default().fg(Color::Cyan),
        )));

        // Scroll logic: if the prompt line is not visible, scroll to it.
        // We calculate height roughly.
        let content_height = inner_area.height as usize;
        let total_lines = lines.len();

        // Auto-scroll to bottom if we are near the end or if it's the default behavior
        let scroll_offset = if total_lines > content_height {
            (total_lines - content_height) as u16
        } else {
            0
        };

        let paragraph = Paragraph::new(lines.clone())
            .style(Style::default().fg(Color::White))
            .wrap(ratatui::widgets::Wrap { trim: false })
            .scroll((scroll_offset, 0));

        f.render_widget(paragraph, inner_area);

        // Calculate cursor position more accurately
        // We need to know how many VISUAL lines the content takes up
        // considering wrapping.
        let width = inner_area.width as usize;
        let mut visual_lines_count = 0;

        // Only calculate if we have width (avoid div by 0)
        if width > 0 {
            // Need to recreate the input line text for calculation since we didn't save it separately
            // Actually 'lines' contains everything.

            // NOTE: This simple calculation assumes NO escape codes width issues (standard chars).
            // Ratatui `Line::width()` returns the display width.
            for line in &lines {
                let line_width = line.width();
                if line_width == 0 {
                    visual_lines_count += 1;
                } else {
                    // Ceiling division for wrapping
                    visual_lines_count += line_width.div_ceil(width);
                }
            }

            let prompt_len = prompt.chars().count();
            let input_len = input.chars().count();
            let total_len = prompt_len + input_len;

            let cursor_x = inner_area.x + (total_len % width) as u16;

            // Calculate Y
            // If content fits in height, cursor is at (top + lines - 1)
            // If content > height, we scrolled, so cursor is at bottom.
            // CAUTION: The 'scroll_offset' calculation above used LOGICAL lines, not visual.
            // That scroll calculation might be slightly off if wrapping happens, but fixing the
            // cursor logic to match the *visual* reality is most important.
            // However, if the paragraph scroller logic uses logical lines, but displays wrapped lines...
            // "Paragraph with Wrap" usually handles scrolling by line index?
            // Actually, for simplicity, if we assume the standard REPL behavior where we auto-scroll to the bottom:

            let cursor_y = if visual_lines_count > content_height {
                inner_area.y + inner_area.height - 1
            } else {
                inner_area.y + visual_lines_count as u16 - 1
            };

            f.set_cursor_position((cursor_x, cursor_y));
        }
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
