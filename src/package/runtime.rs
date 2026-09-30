//! 包管理命令层的 async 驱动（RFC-014a Phase 4）
//!
//! Source 层自 Phase 3.5 起 async 化；GitHub 适配层引入 reqwest 后 future
//! 需要 tokio reactor（`futures::executor::block_on` 无法驱动 IO/定时器）。
//! 本模块提供包管理自有的惰性单例运行时：current_thread + IO/时间驱动，
//! 仅供 [`crate::package`] 命令层经 [`drive`] 使用，编译器其余部分保持同步。

use std::sync::OnceLock;

use tokio::runtime::Runtime;

static RUNTIME: OnceLock<Runtime> = OnceLock::new();

/// 在包管理自有的 tokio 运行时上驱动 future
pub(crate) fn drive<F: std::future::Future>(fut: F) -> F::Output {
    RUNTIME
        .get_or_init(|| {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("build tokio runtime for package manager")
        })
        .block_on(fut)
}
