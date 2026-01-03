//! Span 单元测试
//!
//! 测试源位置跟踪的 Position、Span 和 SourceFile

use crate::util::span::{Position, SourceFile, Span};

#[cfg(test)]
mod position_tests {
    use super::*;

    #[test]
    fn test_position_creation() {
        let pos = Position::new(1, 5);
        assert_eq!(pos.line, 1);
        assert_eq!(pos.column, 5);
        assert_eq!(pos.offset, 0);
    }

    #[test]
    fn test_position_with_offset() {
        let pos = Position::with_offset(1, 5, 100);
        assert_eq!(pos.line, 1);
        assert_eq!(pos.column, 5);
        assert_eq!(pos.offset, 100);
    }

    #[test]
    fn test_position_dummy() {
        let pos = Position::dummy();
        assert_eq!(pos.line, 0);
        assert_eq!(pos.column, 0);
        assert_eq!(pos.offset, 0);
    }

    #[test]
    fn test_position_display() {
        let pos = Position::new(10, 20);
        let display = format!("{}", pos);
        assert_eq!(display, "10:20");
    }

    #[test]
    fn test_position_partial_eq() {
        assert_eq!(Position::new(1, 5), Position::new(1, 5));
        assert_ne!(Position::new(1, 5), Position::new(1, 6));
    }

    #[test]
    fn test_position_clone() {
        let pos = Position::new(5, 10);
        let cloned = pos.clone();
        assert_eq!(pos, cloned);
    }

    #[test]
    fn test_position_debug() {
        let pos = Position::new(1, 1);
        let debug = format!("{:?}", pos);
        assert!(debug.contains("Position"));
    }
}

#[cfg(test)]
mod span_tests {
    use super::*;

    #[test]
    fn test_span_creation() {
        let start = Position::new(1, 1);
        let end = Position::new(1, 10);
        let span = Span::new(start, end);
        assert_eq!(span.start, start);
        assert_eq!(span.end, end);
    }

    #[test]
    fn test_span_dummy() {
        let span = Span::dummy();
        assert!(span.is_dummy());
    }

    #[test]
    fn test_span_is_dummy() {
        let dummy = Span::dummy();
        assert!(dummy.is_dummy());

        let real = Span::new(Position::new(1, 1), Position::new(1, 10));
        assert!(!real.is_dummy());
    }

    #[test]
    fn test_span_is_empty() {
        let start = Position::with_offset(1, 1, 0);
        let end = Position::with_offset(1, 1, 0);
        let empty_span = Span::new(start, end);
        assert!(empty_span.is_empty());

        let start = Position::with_offset(1, 1, 0);
        let end = Position::with_offset(1, 2, 10);
        let non_empty_span = Span::new(start, end);
        assert!(!non_empty_span.is_empty());
    }

    #[test]
    fn test_span_len() {
        let start = Position::with_offset(1, 1, 0);
        let end = Position::with_offset(1, 5, 100);
        let span = Span::new(start, end);
        assert_eq!(span.len(), 100);
    }

    #[test]
    fn test_span_len_empty() {
        let start = Position::with_offset(1, 1, 50);
        let end = Position::with_offset(1, 1, 50);
        let span = Span::new(start, end);
        assert_eq!(span.len(), 0);
    }

    #[test]
    fn test_span_display() {
        let start = Position::new(1, 1);
        let end = Position::new(1, 10);
        let span = Span::new(start, end);
        let display = format!("{}", span);
        assert!(display.contains("1:1"));
        assert!(display.contains("1:10"));
    }

    #[test]
    fn test_span_partial_eq() {
        let pos1 = Position::new(1, 1);
        let pos2 = Position::new(1, 10);
        let span1 = Span::new(pos1, pos2);
        let span2 = Span::new(pos1, pos2);
        assert_eq!(span1, span2);
    }

    #[test]
    fn test_span_clone() {
        let span = Span::new(Position::new(1, 1), Position::new(1, 10));
        let cloned = span.clone();
        assert_eq!(span, cloned);
    }

    #[test]
    fn test_span_debug() {
        let span = Span::new(Position::new(1, 1), Position::new(1, 10));
        let debug = format!("{:?}", span);
        assert!(debug.contains("Span"));
    }
}

#[cfg(test)]
mod source_file_tests {
    use super::*;

    #[test]
    fn test_source_file_creation() {
        let file = SourceFile::new("test.yx".to_string(), "hello world".to_string());
        assert_eq!(file.name, "test.yx");
        assert_eq!(file.content, "hello world");
    }

    #[test]
    fn test_source_file_display() {
        let file = SourceFile::new("test.yx".to_string(), "".to_string());
        let display = format!("{}", file);
        assert_eq!(display, "test.yx");
    }

    #[test]
    fn test_source_file_position_from_offset() {
        let file = SourceFile::new("test.yx".to_string(), "line1\nline2".to_string());
        let pos = file.position_from_offset(0);
        assert_eq!(pos.line, 1);
        assert_eq!(pos.column, 1);
        assert_eq!(pos.offset, 0);

        let pos = file.position_from_offset(6);
        assert_eq!(pos.line, 2);
        assert_eq!(pos.column, 1);
    }

    #[test]
    fn test_source_file_position_from_offset_multiline() {
        let content = "line1\nline2\nline3";
        let file = SourceFile::new("test.yx".to_string(), content.to_string());
        
        let pos0 = file.position_from_offset(0);
        assert_eq!(pos0.line, 1);
        assert_eq!(pos0.column, 1);
        
        let pos5 = file.position_from_offset(5);
        assert_eq!(pos5.line, 1);
        assert_eq!(pos5.column, 6);
        
        let pos6 = file.position_from_offset(6);
        assert_eq!(pos6.line, 2);
        assert_eq!(pos6.column, 1);
        
        let pos12 = file.position_from_offset(12);
        assert_eq!(pos12.line, 3);
        assert_eq!(pos12.column, 1);
    }

    #[test]
    fn test_source_file_span_from_range() {
        let file = SourceFile::new("test.yx".to_string(), "hello world".to_string());
        let span = file.span_from_range(0, 5);
        assert_eq!(span.start.line, 1);
        assert_eq!(span.start.column, 1);
        assert_eq!(span.end.line, 1);
        assert_eq!(span.end.column, 6);
    }

    #[test]
    fn test_source_file_source_text() {
        let file = SourceFile::new("test.yx".to_string(), "hello world".to_string());
        let span = file.span_from_range(0, 5);
        let text = file.source_text(span);
        assert_eq!(text, Some("hello"));
    }

    #[test]
    fn test_source_file_clone() {
        let file = SourceFile::new("test.yx".to_string(), "content".to_string());
        let cloned = file.clone();
        assert_eq!(cloned.name, file.name);
        assert_eq!(cloned.content, file.content);
    }

    #[test]
    fn test_source_file_debug() {
        let file = SourceFile::new("test.yx".to_string(), "content".to_string());
        let debug = format!("{:?}", file);
        assert!(debug.contains("SourceFile"));
        assert!(debug.contains("test.yx"));
    }

    #[test]
    fn test_source_file_empty_content() {
        let file = SourceFile::new("empty.yx".to_string(), "".to_string());
        assert_eq!(file.content.len(), 0);
        
        let pos = file.position_from_offset(0);
        assert_eq!(pos.line, 1);
        assert_eq!(pos.column, 1);
    }

    #[test]
    fn test_source_file_single_line() {
        let file = SourceFile::new("single.yx".to_string(), "only one line".to_string());
        let pos = file.position_from_offset(0);
        assert_eq!(pos.line, 1);
        
        let pos = file.position_from_offset(14);
        assert_eq!(pos.line, 1);
        assert_eq!(pos.column, 15);
    }

    // 从span.rs迁移的测试
    #[test]
    fn test_source_file_source_text_none() {
        let file = SourceFile::new("test.yx".to_string(), "hello".to_string());
        let span = Span::new(Position::with_offset(1, 1, 10), Position::with_offset(1, 1, 15));
        let text = file.source_text(span);
        assert_eq!(text, None);
    }
}
