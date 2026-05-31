//! 注释格式化处理器

use super::super::source_map::SourceMap;

/// 格式化两个行号之间的注释
pub fn format_comments_between(
    source_map: &SourceMap,
    start_line: usize,
    end_line: usize,
) -> String {
    let comments = source_map.comments_between_lines(start_line, end_line);
    let mut result = String::new();

    for comment in comments {
        result.push_str(&comment.content);
        if !comment.content.ends_with('\n') {
            result.push('\n');
        }
    }

    result
}
