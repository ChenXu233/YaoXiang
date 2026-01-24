/// 输入窗口组件
///
/// 提供智能输入功能，包括语法高亮和自动补全
use ratatui::{
    layout::{Rect, Margin},
    style::{Color, Style},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

/// 输入窗口
pub struct InputWindow {
    /// 输入缓冲区
    buffer: String,
    /// 光标位置
    cursor_pos: usize,
    /// 多行模式
    is_multi_line: bool,
    /// 括号深度
    paren_depth: usize,
}

impl InputWindow {
    /// 创建新的输入窗口
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            cursor_pos: 0,
            is_multi_line: false,
            paren_depth: 0,
        }
    }

    /// 获取输入缓冲区
    pub fn buffer(&self) -> &str {
        &self.buffer
    }

    /// 设置输入缓冲区
    pub fn set_buffer(
        &mut self,
        buffer: String,
    ) {
        self.buffer = buffer;
        self.cursor_pos = self.buffer.len();
    }

    /// 清空缓冲区
    pub fn clear(&mut self) {
        self.buffer.clear();
        self.cursor_pos = 0;
        self.is_multi_line = false;
        self.paren_depth = 0;
    }

    /// 插入字符
    pub fn insert_char(
        &mut self,
        ch: char,
    ) {
        self.buffer.insert(self.cursor_pos, ch);
        self.cursor_pos += 1;
    }

    /// 删除字符（光标前）
    pub fn delete_char(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
            self.buffer.remove(self.cursor_pos);
        }
    }

    /// 向前移动光标
    pub fn move_cursor_forward(&mut self) {
        if self.cursor_pos < self.buffer.len() {
            self.cursor_pos += 1;
        }
    }

    /// 向后移动光标
    pub fn move_cursor_back(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
        }
    }

    /// 移动到行首
    pub fn move_to_line_start(&mut self) {
        self.cursor_pos = 0;
    }

    /// 移动到行尾
    pub fn move_to_line_end(&mut self) {
        self.cursor_pos = self.buffer.len();
    }

    /// 检查输入是否完整
    pub fn is_complete(&self) -> bool {
        self.paren_depth == 0 && !self.is_multi_line
    }

    /// 获取提示符
    pub fn get_prompt(&self) -> &str {
        if self.is_multi_line || self.paren_depth > 0 {
            "   "
        } else {
            ">> "
        }
    }

    /// 渲染输入窗口
    pub fn render(
        &self,
        f: &mut Frame<'_>,
        area: Rect,
        buffer: &str,
    ) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Plain)
            .title(" Input ");

        f.render_widget(block, area);

        let inner_area = area.inner(&Margin::default());

        // 渲染提示符和输入
        let prompt = self.get_prompt();
        let content = format!("{}{}", prompt, buffer);

        let paragraph = Paragraph::new(content)
            .style(Style::default().fg(Color::White))
            .scroll((0, 0));

        f.render_widget(paragraph, inner_area);

        // 渲染光标（简化版）
        // 注意：ratatui 中直接渲染光标比较复杂，这里只是占位符
    }
}

impl Default for InputWindow {
    fn default() -> Self {
        Self::new()
    }
}
