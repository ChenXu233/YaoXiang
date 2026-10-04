//! 字节码序列化单元测试
//!
//! 测试 DebugSection 的序列化和反序列化（round-trip）功能。
//!
//! 规范来源：
//! - RFC-034 §「IR 调试元数据」（调试段格式）：v1 仅 ip→span；v2 增每函数
//!   局部变量名表；v3 增全局槽位名表。读取端接受 v1/v2/v3，旧产物补空表不 panic。

use crate::frontend::core::typecheck::MonoType;
use crate::middle::passes::codegen::bytecode::{
    BytecodeFile, BytecodeInstruction, CodeSection, DebugSection, FileHeader, FunctionCode,
};
use crate::backends::common::opcode;
use crate::util::span::{DebugSpan, Position, SourceMap, Span};
use std::collections::HashMap;
use std::io;

// ── 辅助函数（规则 5.3：集中在文件顶部）──────────────

/// Helper: 构造含单个 main 函数的字节码文件，其 ip→span 调试映射为 `debug_span`。
fn bytecode_file_with_debug_map(
    sources: SourceMap,
    debug_span: DebugSpan,
) -> BytecodeFile {
    let function = FunctionCode {
        name: "main".to_string(),
        params: Vec::new(),
        return_type: MonoType::Void,
        instructions: vec![BytecodeInstruction::new(opcode::NOP, vec![])],
        local_count: 0,
        local_names: HashMap::new(),
        debug_map: HashMap::from([(0usize, debug_span)]),
    };

    let code_section = CodeSection {
        functions: vec![function],
    };

    let debug_section = DebugSection::from_sources_and_functions(sources, &code_section.functions);

    BytecodeFile {
        header: FileHeader::default(),
        type_table: Vec::new(),
        const_pool: Vec::new(),
        code_section,
        vtables: Vec::new(),
        debug_section: Some(debug_section),
    }
}

/// Helper: 序列化字节码文件后从尾部读回调试段。
fn roundtrip_debug_section(file: &BytecodeFile) -> DebugSection {
    let mut bytes = Vec::new();
    file.write_to(&mut bytes).expect("write bytecode");

    let mut cursor = io::Cursor::new(bytes);
    DebugSection::read_from_end(&mut cursor)
        .expect("read debug section")
        .expect("debug section should exist")
}
#[test]
fn test_debug_section_round_trip() {
    // Arrange
    let mut sources = SourceMap::new();
    let file_id = sources.add_file("main.yx".to_string(), "main = () => { 1 / 0 }".to_string());

    let span = Span::new(
        Position::with_offset(1, 1, 0),
        Position::with_offset(1, 5, 4),
    );
    let debug_span = DebugSpan::new(file_id, span);
    let file = bytecode_file_with_debug_map(sources.clone(), debug_span);

    // Act
    let decoded = roundtrip_debug_section(&file);

    assert_eq!(decoded.sources.files().len(), 1);
    assert_eq!(decoded.sources.files()[0].name, "main.yx");
    assert_eq!(decoded.sources.files()[0].content, "main = () => { 1 / 0 }");
    assert_eq!(decoded.function_debug_maps.len(), 1);
    assert_eq!(
        decoded.function_debug_maps[0].get(&0).copied(),
        Some(debug_span)
    );
}

#[test]
fn test_debug_section_round_trip_global_names() {
    // #368：全局槽位名表（v3）——顶层绑定的名字不在任何函数局部名表里，
    // 顶层 `a[i]` 的越界诊断要靠它才能报出 `i`。往返必须保真。
    let mut sources = SourceMap::new();
    sources.add_file("script.yx".to_string(), "a = [1]\ni = 5".to_string());

    let code_section = CodeSection { functions: vec![] };
    let debug_section = DebugSection::with_global_names(
        sources,
        &code_section.functions,
        HashMap::from([(0usize, "a".to_string()), (1usize, "i".to_string())]),
    );
    let file = BytecodeFile {
        header: FileHeader::default(),
        type_table: Vec::new(),
        const_pool: Vec::new(),
        code_section,
        vtables: Vec::new(),
        debug_section: Some(debug_section),
    };

    let mut bytes = Vec::new();
    file.write_to(&mut bytes).expect("write bytecode");

    let mut cursor = io::Cursor::new(bytes);
    let decoded = DebugSection::read_from_end(&mut cursor)
        .expect("read debug section")
        .expect("debug section should exist");

    assert_eq!(decoded.global_names.get(&0).map(String::as_str), Some("a"));
    assert_eq!(decoded.global_names.get(&1).map(String::as_str), Some("i"));
}
