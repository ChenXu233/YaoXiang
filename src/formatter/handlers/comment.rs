//! 注释格式化处理器

use super::super::source_map::SourceMap;

/// 格式化**严格位于 `before_offset` 之前**、且起始行不早于 `start_line` 的注释。
///
/// 上界用字节偏移而非行号闭区间：与语句同行的行末注释属于该语句（由
/// `append_trailing_comment` 输出），若这里按行号把它也当语句前注释输出一次，
/// 每次格式化都会多复制一份（幂等性缺陷）。
pub fn format_comments_before(
    source_map: &SourceMap,
    start_line: usize,
    before_offset: usize,
) -> String {
    let comments =
        source_map.claim_unemitted(source_map.comments_before_offset(start_line, before_offset));
    let mut result = String::new();

    for comment in comments {
        result.push_str(&comment.content);
        if !comment.content.ends_with('\n') {
            result.push('\n');
        }
    }

    result
}
