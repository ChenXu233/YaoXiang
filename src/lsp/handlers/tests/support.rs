//! LSP 处理器测试共享助手
//!
//! 生产路径等价构造（#433/RFC-039 D55 静默通道判定）：语义数据一律由真实
//! tokenize → parse → TypeChecker::check_module_collect_all 管线产出、经
//! World::update_semantic_db 按文件 upsert，与 `server.rs` 的
//! `update_semantic_db` 完全同款。**禁止在测试里手工构造
//! definitions/references**——消费端测试手工喂登记数据即静默通道
//! （coding-rules 第六部分）。

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;
use crate::frontend::core::typecheck::TypeChecker;
use crate::lsp::session::Session;
use crate::lsp::world::World;

/// 测试样例的虚拟文档 URI
pub(crate) const SAMPLE_URI: &str = "file:///test/main.yx";

/// 以生产同款管线把 `content` 的语义数据灌入 `world`（按 `uri` upsert）
pub(crate) fn check_semantic_into_world(
    world: &mut World,
    uri: &str,
    content: &str,
) {
    let tokens = tokenize(content).expect("测试源码必须可词法分析");
    let parsed = parse(&tokens);
    let mut checker = TypeChecker::new(uri);
    let result = checker.check_module_collect_all(&parsed.module);
    world.update_semantic_db(result.semantic_db);
}

/// 打开文档（didOpen 形态）并灌入语义数据，返回 (Session, World)
pub(crate) fn open_and_check(content: &str) -> (Session, World) {
    let mut session = Session::new();
    let mut world = World::new();
    session
        .document_store_mut()
        .open(SAMPLE_URI.to_string(), content.to_string(), 1);
    check_semantic_into_world(&mut world, SAMPLE_URI, content);
    (session, world)
}
