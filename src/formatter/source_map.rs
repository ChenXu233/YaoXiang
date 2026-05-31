//! 源映射系统
//!
//! 记录源代码中的注释、空白行和 Token 位置信息，
//! 为格式化工具提供完整的源代码元信息。

use crate::util::span::Span;

/// 注释样式
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommentStyle {
    /// 单行注释 `// ...`
    SingleLine,
    /// 多行注释 `/* ... */`
    MultiLine,
    /// 文档注释 `/// ...`
    Doc,
}

/// 注释
#[derive(Debug, Clone)]
pub struct Comment {
    /// 注释内容（包含 `//` 或 `/* */` 标记）
    pub content: String,
    /// 注释在源代码中的位置
    pub span: Span,
    /// 注释样式
    pub style: CommentStyle,
}

/// 源映射
///
/// 记录源代码中所有注释和空白行信息，
/// 用于在格式化输出中正确保留注释。
#[derive(Debug, Clone)]
pub struct SourceMap {
    /// 原始源代码
    pub source: String,
    /// 注释列表（按位置排序）
    pub comments: Vec<Comment>,
    /// 每行起始字节偏移（用于偏移到行列转换）
    pub line_offsets: Vec<usize>,
    /// 空白行号列表（1-indexed）
    pub blank_lines: Vec<usize>,
}

impl SourceMap {
    /// 从源代码构建源映射
    pub fn build(source: &str) -> Self {
        let line_offsets = Self::compute_line_offsets(source);
        let blank_lines = Self::compute_blank_lines(source, &line_offsets);
        let comments = Self::scan_comments(source);

        Self {
            source: source.to_string(),
            comments,
            line_offsets,
            blank_lines,
        }
    }

    /// 计算每行起始偏移
    fn compute_line_offsets(source: &str) -> Vec<usize> {
        let mut offsets = vec![0]; // 第一行从偏移0开始
        for (i, ch) in source.char_indices() {
            if ch == '\n' {
                offsets.push(i + 1);
            }
        }
        offsets
    }

    /// 计算空白行
    fn compute_blank_lines(
        source: &str,
        line_offsets: &[usize],
    ) -> Vec<usize> {
        let mut blank_lines = Vec::new();
        let lines: Vec<&str> = source.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            if line.trim().is_empty() {
                blank_lines.push(i + 1); // 1-indexed
            }
        }
        let _ = line_offsets; // used for validation if needed
        blank_lines
    }

    /// 扫描源代码中的注释
    fn scan_comments(source: &str) -> Vec<Comment> {
        let mut comments = Vec::new();
        let chars: Vec<char> = source.chars().collect();
        let len = chars.len();
        let mut i = 0;
        let mut line: usize = 1;
        let mut column: usize = 1;
        let mut offset: usize = 0;

        while i < len {
            match chars[i] {
                '"' => {
                    // 跳过字符串字面量
                    i += 1;
                    offset += chars
                        .get(i.wrapping_sub(1))
                        .map(|c| c.len_utf8())
                        .unwrap_or(1);
                    column += 1;
                    while i < len && chars[i] != '"' {
                        if chars[i] == '\\' && i + 1 < len {
                            i += 1;
                            offset += chars[i - 1].len_utf8();
                            column += 1;
                        }
                        if chars[i] == '\n' {
                            line += 1;
                            column = 1;
                        } else {
                            column += 1;
                        }
                        offset += chars[i].len_utf8();
                        i += 1;
                    }
                    if i < len {
                        offset += chars[i].len_utf8();
                        column += 1;
                        i += 1;
                    }
                }
                '\'' => {
                    // 跳过字符字面量
                    i += 1;
                    offset += 1;
                    column += 1;
                    while i < len && chars[i] != '\'' {
                        if chars[i] == '\\' && i + 1 < len {
                            i += 1;
                            offset += 1;
                            column += 1;
                        }
                        offset += chars[i].len_utf8();
                        column += 1;
                        i += 1;
                    }
                    if i < len {
                        offset += chars[i].len_utf8();
                        column += 1;
                        i += 1;
                    }
                }
                '/' if i + 1 < len && chars[i + 1] == '/' => {
                    // 单行注释
                    let start_line = line;
                    let start_col = column;
                    let start_offset = offset;
                    let mut content = String::new();
                    // 检查是否是文档注释 ///
                    let is_doc = i + 2 < len && chars[i + 2] == '/';
                    while i < len && chars[i] != '\n' {
                        content.push(chars[i]);
                        offset += chars[i].len_utf8();
                        column += 1;
                        i += 1;
                    }
                    let style = if is_doc {
                        CommentStyle::Doc
                    } else {
                        CommentStyle::SingleLine
                    };
                    comments.push(Comment {
                        content,
                        span: Span::new(
                            crate::util::span::Position::with_offset(
                                start_line,
                                start_col,
                                start_offset,
                            ),
                            crate::util::span::Position::with_offset(line, column, offset),
                        ),
                        style,
                    });
                }
                '/' if i + 1 < len && chars[i + 1] == '*' => {
                    // 多行注释
                    let start_line = line;
                    let start_col = column;
                    let start_offset = offset;
                    let mut content = String::new();
                    let mut depth = 1;
                    content.push(chars[i]);
                    offset += chars[i].len_utf8();
                    column += 1;
                    i += 1;
                    content.push(chars[i]);
                    offset += chars[i].len_utf8();
                    column += 1;
                    i += 1;
                    while i < len && depth > 0 {
                        if chars[i] == '/' && i + 1 < len && chars[i + 1] == '*' {
                            depth += 1;
                            content.push(chars[i]);
                            offset += chars[i].len_utf8();
                            column += 1;
                            i += 1;
                        } else if chars[i] == '*' && i + 1 < len && chars[i + 1] == '/' {
                            depth -= 1;
                            content.push(chars[i]);
                            offset += chars[i].len_utf8();
                            column += 1;
                            i += 1;
                            if depth == 0 {
                                // Skip the closing '/'
                                content.push(chars[i]);
                                offset += chars[i].len_utf8();
                                column += 1;
                                i += 1;
                                break;
                            }
                        }
                        if chars[i] == '\n' {
                            line += 1;
                            column = 1;
                        } else {
                            column += 1;
                        }
                        content.push(chars[i]);
                        offset += chars[i].len_utf8();
                        i += 1;
                    }
                    comments.push(Comment {
                        content,
                        span: Span::new(
                            crate::util::span::Position::with_offset(
                                start_line,
                                start_col,
                                start_offset,
                            ),
                            crate::util::span::Position::with_offset(line, column, offset),
                        ),
                        style: CommentStyle::MultiLine,
                    });
                }
                '\n' => {
                    line += 1;
                    column = 1;
                    offset += 1;
                    i += 1;
                }
                c => {
                    offset += c.len_utf8();
                    column += 1;
                    i += 1;
                }
            }
        }

        comments
    }

    /// 根据字节偏移获取行号（1-indexed）
    pub fn offset_to_line(
        &self,
        offset: usize,
    ) -> usize {
        match self.line_offsets.binary_search(&offset) {
            Ok(idx) => idx + 1,
            Err(idx) => idx,
        }
    }

    /// 根据字节偏移获取列号（1-indexed）
    pub fn offset_to_column(
        &self,
        offset: usize,
    ) -> usize {
        let line = self.offset_to_line(offset);
        if line > 0 && line <= self.line_offsets.len() {
            offset - self.line_offsets[line - 1] + 1
        } else {
            1
        }
    }

    /// 获取在给定 span 之前的注释
    pub fn comments_before(
        &self,
        span: &Span,
    ) -> Vec<&Comment> {
        self.comments
            .iter()
            .filter(|c| c.span.end.offset <= span.start.offset)
            .collect()
    }

    /// 获取在给定行范围之间的注释
    pub fn comments_between_lines(
        &self,
        start_line: usize,
        end_line: usize,
    ) -> Vec<&Comment> {
        self.comments
            .iter()
            .filter(|c| c.span.start.line >= start_line && c.span.end.line <= end_line)
            .collect()
    }

    /// 获取行末注释（与代码在同一行）
    pub fn trailing_comment_on_line(
        &self,
        line: usize,
    ) -> Option<&Comment> {
        self.comments.iter().find(|c| {
            c.span.start.line == line
                && matches!(c.style, CommentStyle::SingleLine | CommentStyle::Doc)
        })
    }

    /// 重建导入语句的注释顺序
    ///
    /// old_order: 旧的导入索引顺序（相对于 use_indices）
    /// new_order: 新的导入索引顺序（相对于 use_indices）
    /// import_stmts: 所有导入语句（用于获取 span）
    pub fn rebuild_comments_for_imports(
        &mut self,
        old_order: &[usize],
        new_order: &[usize],
        import_stmts: &[&crate::frontend::core::parser::ast::Stmt],
    ) {
        // 1. 为每个旧导入索引关联其注释
        let mut comments_per_import: Vec<Vec<Comment>> = Vec::new();

        for (i, &old_idx) in old_order.iter().enumerate() {
            let stmt = import_stmts[old_idx];
            let comment_start = if i > 0 {
                import_stmts[old_order[i - 1]].span.end.line + 1
            } else {
                1
            };
            let comment_end = stmt.span.start.line;

            let comments: Vec<Comment> = self
                .comments
                .iter()
                .filter(|c| c.span.start.line >= comment_start && c.span.end.line < comment_end)
                .cloned()
                .collect();
            comments_per_import.push(comments);
        }

        // 2. 按新顺序重建注释列表
        let mut new_comments: Vec<Comment> = Vec::new();
        for &new_idx in new_order {
            new_comments.extend(comments_per_import[new_idx].iter().cloned());
        }

        // 3. 添加非导入区域的注释
        if !old_order.is_empty() {
            let first_import_line = import_stmts[old_order[0]].span.start.line;
            let last_import_line = import_stmts[old_order[old_order.len() - 1]].span.end.line;

            let non_import_comments: Vec<Comment> = self
                .comments
                .iter()
                .filter(|c| {
                    c.span.start.line < first_import_line || c.span.start.line > last_import_line
                })
                .cloned()
                .collect();

            // 头部注释 + 导入注释 + 尾部注释
            let mut final_comments: Vec<Comment> = non_import_comments
                .iter()
                .filter(|c| c.span.start.line < first_import_line)
                .cloned()
                .collect();
            final_comments.extend(new_comments);
            final_comments.extend(
                non_import_comments
                    .iter()
                    .filter(|c| c.span.start.line > last_import_line)
                    .cloned(),
            );
            self.comments = final_comments;
        } else {
            self.comments = new_comments;
        }
    }
}
