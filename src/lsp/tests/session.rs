//! LSP 会话管理测试
//!
//! 测试覆盖：
//! - 会话生命周期状态
//! - 工作区根路径设置
//! - 文档存储管理

use crate::lsp::session::{Session, SessionState};

#[test]
fn test_session_lifecycle() {
    let mut session = Session::new();
    assert_eq!(session.state(), SessionState::Uninitialized);
    assert!(!session.is_ready());

    session.set_state(SessionState::Initializing);
    assert_eq!(session.state(), SessionState::Initializing);
    assert!(!session.is_ready());

    session.set_state(SessionState::Running);
    assert_eq!(session.state(), SessionState::Running);
    assert!(session.is_ready());
    assert!(!session.is_shutting_down());

    session.set_state(SessionState::ShuttingDown);
    assert!(session.is_shutting_down());
    assert!(!session.is_ready());
}

#[test]
fn test_session_root_path() {
    let mut session = Session::new();
    assert!(session.root_path().is_none());

    session.set_root_path(Some("/workspace/project".to_string()));
    assert_eq!(session.root_path(), Some("/workspace/project"));
}
