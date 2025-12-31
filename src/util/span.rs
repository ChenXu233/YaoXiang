//! Source location tracking

use std::fmt;

/// Source position (line, column, and byte offset)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Position {
    /// Line number (1-indexed)
    pub line: usize,
    /// Column number (1-indexed)
    pub column: usize,
    /// Byte offset from start of file
    pub offset: usize,
}

impl Position {
    /// Create a new position
    #[inline]
    pub fn new(line: usize, column: usize) -> Self {
        Self { line, column, offset: 0 }
    }

    /// Create a new position with offset
    #[inline]
    pub fn with_offset(line: usize, column: usize, offset: usize) -> Self {
        Self { line, column, offset }
    }

    /// Create a dummy position
    #[inline]
    pub fn dummy() -> Self {
        Self { line: 0, column: 0, offset: 0 }
    }
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

/// Source span (start position to end position)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Span {
    /// Start position (inclusive)
    pub start: Position,
    /// End position (exclusive)
    pub end: Position,
}

impl Span {
    /// Create a new span
    #[inline]
    pub fn new(start: Position, end: Position) -> Self {
        Self { start, end }
    }

    /// Create a dummy span
    #[inline]
    pub fn dummy() -> Self {
        Self {
            start: Position::dummy(),
            end: Position::dummy(),
        }
    }

    /// Check if this is a dummy span
    #[inline]
    pub fn is_dummy(&self) -> bool {
        self.start.line == 0
    }

    /// Get the source text length
    #[inline]
    pub fn len(&self) -> usize {
        self.end.offset.saturating_sub(self.start.offset)
    }

    /// Check if span is empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.start.offset == self.end.offset
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{} - {}]", self.start, self.end)
    }
}

/// Source file information
#[derive(Debug, Clone)]
pub struct SourceFile {
    /// File name
    pub name: String,
    /// File content
    pub content: String,
    /// Line offsets for fast line lookup
    line_offsets: Vec<usize>,
}

impl SourceFile {
    /// Create a new source file
    pub fn new(name: String, content: String) -> Self {
        let mut line_offsets = vec![0];
        for (i, _) in content.char_indices() {
            if content[i..].starts_with('\n') {
                line_offsets.push(i + 1);
            }
        }
        line_offsets.push(content.len());

        Self {
            name,
            content,
            line_offsets,
        }
    }

    /// Get position from byte offset
    pub fn position_from_offset(&self, offset: usize) -> Position {
        let line = self.line_offsets.partition_point(|&o| o <= offset);
        let column = offset.saturating_sub(self.line_offsets[line.saturating_sub(1)]);
        Position::with_offset(line, column + 1, offset)
    }

    /// Get span from byte range
    pub fn span_from_range(&self, start: usize, end: usize) -> Span {
        Span {
            start: self.position_from_offset(start),
            end: self.position_from_offset(end),
        }
    }

    /// Get source text for a span
    pub fn source_text(&self, span: Span) -> Option<&str> {
        let start = self.line_offsets[span.start.line.saturating_sub(1)] + (span.start.column - 1).min(self.line_offsets[span.start.line.saturating_sub(1) + 1].saturating_sub(self.line_offsets[span.start.line.saturating_sub(1)]) - 1);
        let end = self.line_offsets[span.end.line.saturating_sub(1)] + (span.end.column - 1).min(self.line_offsets[span.end.line.saturating_sub(1) + 1].saturating_sub(self.line_offsets[span.end.line.saturating_sub(1)]) - 1);
        self.content.get(start..end)
    }
}

impl fmt::Display for SourceFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}
