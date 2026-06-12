//! 字节码序列化单元测试
//!
//! 测试 DebugSection 的序列化和反序列化（round-trip）功能。

use crate::frontend::core::typecheck::MonoType;
use crate::middle::passes::codegen::bytecode::{
    BytecodeFile, BytecodeInstruction, CodeSection, DebugSection, FileHeader, FunctionCode,
};
use crate::backends::common::Opcode;
use crate::util::span::{DebugSpan, Position, SourceMap, Span};
use std::collections::HashMap;
use std::io;

#[test]
fn test_debug_section_round_trip() {
    let mut sources = SourceMap::new();
    let file_id = sources.add_file("main.yx".to_string(), "main = () => { 1 / 0 }".to_string());

    let span = Span::new(
        Position::with_offset(1, 1, 0),
        Position::with_offset(1, 5, 4),
    );
    let debug_span = DebugSpan::new(file_id, span);

    let function = FunctionCode {
        name: "main".to_string(),
        params: Vec::new(),
        return_type: MonoType::Void,
        instructions: vec![BytecodeInstruction::new(Opcode::Nop, vec![])],
        local_count: 0,
        debug_map: HashMap::from([(0usize, debug_span)]),
    };

    let code_section = CodeSection {
        functions: vec![function],
    };

    let debug_section =
        DebugSection::from_sources_and_functions(sources.clone(), &code_section.functions);
    let file = BytecodeFile {
        header: FileHeader::default(),
        type_table: Vec::new(),
        const_pool: Vec::new(),
        code_section,
        debug_section: Some(debug_section),
    };

    let mut bytes = Vec::new();
    file.write_to(&mut bytes).expect("write bytecode");

    let mut cursor = io::Cursor::new(bytes);
    let decoded = DebugSection::read_from_end(&mut cursor)
        .expect("read debug section")
        .expect("debug section should exist");

    assert_eq!(decoded.sources.files().len(), 1);
    assert_eq!(decoded.sources.files()[0].name, "main.yx");
    assert_eq!(decoded.sources.files()[0].content, "main = () => { 1 / 0 }");
    assert_eq!(decoded.function_debug_maps.len(), 1);
    assert_eq!(
        decoded.function_debug_maps[0].get(&0).copied(),
        Some(debug_span)
    );
}
