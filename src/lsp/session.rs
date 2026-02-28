//! LSP 会话管理
//!
//! 跟踪 LSP 会话的生命周期状态。

use crate::util::cache::DocumentStore;

/// LSP 会话状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    /// 未初始化（等待 initialize 请求）
    Uninitialized,
    /// 正在初始化（已收到 initialize，等待 initialized 通知）
    Initializing,
    /// 已初始化（正常工作状态）
    Running,
    /// 正在关闭（已收到 shutdown 请求）
    ShuttingDown,
}

/// LSP 会话
///
/// 管理单个客户端连接的状态和文档缓存。
#[derive(Debug)]
pub struct Session {
    /// 会话状态
    state: SessionState,
    /// 文档存储
    document_store: DocumentStore,
    /// 客户端根路径
    root_path: Option<String>,
}

impl Session {
    /// 创建新的会话
    pub fn new() -> Self {
        Self {
            state: SessionState::Uninitialized,
            document_store: DocumentStore::new(),
            root_path: None,
        }
    }

    /// 获取当前状态
    pub fn state(&self) -> SessionState {
        self.state
    }

    /// 设置状态
    pub fn set_state(
        &mut self,
        state: SessionState,
    ) {
        tracing::info!("LSP 会话状态变更: {:?} → {:?}", self.state, state);
        self.state = state;
    }

    /// 获取文档存储（不可变）
    pub fn document_store(&self) -> &DocumentStore {
        &self.document_store
    }

    /// 获取文档存储（可变）
    pub fn document_store_mut(&mut self) -> &mut DocumentStore {
        &mut self.document_store
    }

    /// 设置工作区根路径
    pub fn set_root_path(
        &mut self,
        path: Option<String>,
    ) {
        self.root_path = path;
    }

    /// 获取工作区根路径
    pub fn root_path(&self) -> Option<&str> {
        self.root_path.as_deref()
    }

    /// 检查会话是否已初始化（可正常处理请求）
    pub fn is_ready(&self) -> bool {
        self.state == SessionState::Running
    }

    /// 检查会话是否正在关闭
    pub fn is_shutting_down(&self) -> bool {
        self.state == SessionState::ShuttingDown
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn test_session_document_store() {
        let mut session = Session::new();
        let store = session.document_store_mut();
        store.open("test.yx".to_string(), "x = 42".to_string(), 1);
        assert_eq!(session.document_store().document_count(), 1);
    }
}
