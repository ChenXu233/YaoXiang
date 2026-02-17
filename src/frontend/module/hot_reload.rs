//! 热重载机制
//!
//! 在开发模式下监听 `.yx` 文件变化，触发增量重编译。
//!
//! # 架构
//!
//! ```text
//! FileWatcher → Debouncer → 变更分类 → ModuleCache::invalidate → 重编译
//! ```
//!
//! # 工作流程
//!
//! 1. 监听项目 `src/` 和 `.yaoxiang/vendor/` 目录
//! 2. 文件变化事件经过防抖处理（默认 300ms）
//! 3. 根据变化类型分类：
//!    - `.yx` 文件 → 失效对应模块缓存 → 触发重编译回调
//!    - `yaoxiang.toml` → 重新发现 vendor 依赖
//!    - `yaoxiang.lock` → 检查依赖版本变化
//! 4. 通过回调通知上层（编译器/IDE）

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use parking_lot::Mutex;
use tokio::sync::mpsc;

use super::cache::ModuleCache;

/// 文件变更类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileChange {
    /// 源文件（.yx）被修改
    SourceModified(PathBuf),
    /// 源文件被创建
    SourceCreated(PathBuf),
    /// 源文件被删除
    SourceDeleted(PathBuf),
    /// yaoxiang.toml 变更
    ManifestChanged,
    /// yaoxiang.lock 变更
    LockfileChanged,
}

/// 重载事件（经过防抖处理和分类后的事件）
#[derive(Debug, Clone)]
pub struct ReloadEvent {
    /// 变更列表
    pub changes: Vec<FileChange>,
    /// 需要重编译的模块路径
    pub affected_modules: Vec<String>,
}

/// 热重载配置
#[derive(Debug, Clone)]
pub struct HotReloadConfig {
    /// 防抖时间（毫秒），防止快速连续修改触发多次重编译
    pub debounce_ms: u64,
    /// 监听的目录列表
    pub watch_dirs: Vec<PathBuf>,
    /// 是否监听 vendor 目录
    pub watch_vendor: bool,
}

impl Default for HotReloadConfig {
    fn default() -> Self {
        Self {
            debounce_ms: 300,
            watch_dirs: Vec::new(),
            watch_vendor: false,
        }
    }
}

impl HotReloadConfig {
    /// 从项目根目录创建默认配置
    pub fn from_project(project_root: &Path) -> Self {
        let mut watch_dirs = vec![project_root.join("src")];

        let vendor_dir = project_root.join(".yaoxiang").join("vendor");
        let watch_vendor = vendor_dir.exists();
        if watch_vendor {
            watch_dirs.push(vendor_dir);
        }

        // 监听项目根目录的 toml/lock 文件
        watch_dirs.push(project_root.to_path_buf());

        Self {
            debounce_ms: 300,
            watch_dirs,
            watch_vendor,
        }
    }
}

/// 热重载器
///
/// 监听文件变化并通过 channel 发送 `ReloadEvent`。
/// 集成 `ModuleCache` 自动失效脏缓存。
pub struct HotReloader {
    /// 项目根目录
    project_root: PathBuf,
    /// 配置
    config: HotReloadConfig,
    /// 模块缓存（共享引用）
    cache: Arc<ModuleCache>,
    /// 文件监听器
    _watcher: Option<RecommendedWatcher>,
    /// 是否正在运行
    running: Arc<Mutex<bool>>,
}

impl std::fmt::Debug for HotReloader {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        f.debug_struct("HotReloader")
            .field("project_root", &self.project_root)
            .field("config", &self.config)
            .field("running", &*self.running.lock())
            .finish()
    }
}

impl HotReloader {
    /// 创建新的热重载器（不启动监听）
    pub fn new(
        project_root: PathBuf,
        config: HotReloadConfig,
        cache: Arc<ModuleCache>,
    ) -> Self {
        Self {
            project_root,
            config,
            cache,
            _watcher: None,
            running: Arc::new(Mutex::new(false)),
        }
    }

    /// 启动文件监听
    ///
    /// 返回一个接收 `ReloadEvent` 的 channel receiver。
    /// 调用者应在异步上下文中循环接收事件并触发重编译。
    pub fn start(&mut self) -> Result<mpsc::UnboundedReceiver<ReloadEvent>, HotReloadError> {
        if *self.running.lock() {
            return Err(HotReloadError::AlreadyRunning);
        }

        let (raw_tx, mut raw_rx) = mpsc::unbounded_channel::<Event>();
        let (event_tx, event_rx) = mpsc::unbounded_channel::<ReloadEvent>();

        // 创建 notify watcher
        let tx_clone = raw_tx.clone();
        let watcher = notify::recommended_watcher(move |result: Result<Event, notify::Error>| {
            if let Ok(event) = result {
                let _ = tx_clone.send(event);
            }
        })
        .map_err(|e| HotReloadError::WatcherInit(e.to_string()))?;

        self._watcher = Some(watcher);

        // 设置监听目录
        if let Some(ref mut watcher) = self._watcher {
            for dir in &self.config.watch_dirs {
                if dir.exists() {
                    // 项目根目录只监听顶层文件（yaoxiang.toml 等）
                    let mode = if dir == &self.project_root {
                        RecursiveMode::NonRecursive
                    } else {
                        RecursiveMode::Recursive
                    };
                    watcher
                        .watch(dir, mode)
                        .map_err(|e| HotReloadError::WatcherInit(e.to_string()))?;
                }
            }
        }

        *self.running.lock() = true;

        // 启动防抖处理任务
        let debounce_ms = self.config.debounce_ms;
        let project_root = self.project_root.clone();
        let cache = self.cache.clone();
        let running = self.running.clone();

        tokio::spawn(async move {
            let mut pending_events: Vec<Event> = Vec::new();

            loop {
                if !*running.lock() {
                    break;
                }

                // 收集事件，带超时
                let timeout = tokio::time::sleep(Duration::from_millis(debounce_ms));
                tokio::pin!(timeout);

                tokio::select! {
                    event = raw_rx.recv() => {
                        match event {
                            Some(e) => {
                                pending_events.push(e);
                                // 继续收集更多事件（防抖窗口内）
                                continue;
                            }
                            None => break, // channel 关闭
                        }
                    }
                    _ = &mut timeout => {
                        if pending_events.is_empty() {
                            continue;
                        }

                        // 防抖窗口结束，处理累积的事件
                        let changes = classify_events(&pending_events, &project_root);
                        pending_events.clear();

                        if changes.is_empty() {
                            continue;
                        }

                        // 失效相关缓存
                        let affected = invalidate_cache(&changes, &cache);

                        let reload_event = ReloadEvent {
                            changes,
                            affected_modules: affected,
                        };

                        if event_tx.send(reload_event).is_err() {
                            break; // receiver 已断开
                        }
                    }
                }
            }
        });

        Ok(event_rx)
    }

    /// 停止文件监听
    pub fn stop(&mut self) {
        *self.running.lock() = false;
        self._watcher = None;
    }

    /// 是否正在运行
    pub fn is_running(&self) -> bool {
        *self.running.lock()
    }

    /// 获取项目根目录
    pub fn project_root(&self) -> &Path {
        &self.project_root
    }
}

/// 热重载错误
#[derive(Debug, Clone, thiserror::Error)]
pub enum HotReloadError {
    /// Watcher 初始化失败
    #[error("failed to initialize file watcher: {0}")]
    WatcherInit(String),
    /// 已在运行
    #[error("hot reloader is already running")]
    AlreadyRunning,
}

/// 将原始文件系统事件分类为 FileChange
fn classify_events(
    events: &[Event],
    project_root: &Path,
) -> Vec<FileChange> {
    let mut changes = Vec::new();
    let mut seen_paths = std::collections::HashSet::new();

    for event in events {
        for path in &event.paths {
            // 去重
            if !seen_paths.insert(path.clone()) {
                continue;
            }

            let file_name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            // yaoxiang.toml 变更
            if file_name == "yaoxiang.toml" {
                changes.push(FileChange::ManifestChanged);
                continue;
            }

            // yaoxiang.lock 变更
            if file_name == "yaoxiang.lock" {
                changes.push(FileChange::LockfileChanged);
                continue;
            }

            // 只处理 .yx 文件
            if !file_name.ends_with(".yx") {
                continue;
            }

            match event.kind {
                EventKind::Create(_) => {
                    changes.push(FileChange::SourceCreated(path.clone()));
                }
                EventKind::Remove(_) => {
                    changes.push(FileChange::SourceDeleted(path.clone()));
                }
                EventKind::Modify(_) | EventKind::Any => {
                    changes.push(FileChange::SourceModified(path.clone()));
                }
                _ => {}
            }
        }
    }

    // 去除重复的 ManifestChanged/LockfileChanged
    changes.dedup();
    let _ = project_root; // 预留：将来用于计算相对路径

    changes
}

/// 根据文件变更失效缓存，返回受影响的模块路径
fn invalidate_cache(
    changes: &[FileChange],
    cache: &ModuleCache,
) -> Vec<String> {
    let mut affected = Vec::new();

    for change in changes {
        match change {
            FileChange::SourceModified(path) | FileChange::SourceDeleted(path) => {
                // 通过文件路径失效缓存
                cache.invalidate_by_file(path);
                // 从文件路径推导模块名（粗略）
                if let Some(module_name) = path_to_module_name(path) {
                    affected.push(module_name);
                }
            }
            FileChange::SourceCreated(_) => {
                // 新文件不需要失效缓存，但需要通知上层
            }
            FileChange::ManifestChanged | FileChange::LockfileChanged => {
                // 清空所有缓存（依赖可能变化）
                cache.clear();
                affected.push("*".to_string()); // 标记全部受影响
            }
        }
    }

    affected
}

/// 从文件路径推导模块名
///
/// 例如：`src/utils/math.yx` → `utils.math`
fn path_to_module_name(path: &Path) -> Option<String> {
    let stem = path.file_stem()?.to_str()?;

    // 如果是 mod.yx 或 index.yx，使用父目录名
    if stem == "mod" || stem == "index" {
        let parent = path.parent()?;
        return parent.file_name()?.to_str().map(String::from);
    }

    Some(stem.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::module::cache::CacheMode;

    #[test]
    fn test_classify_yx_modify() {
        let event = Event {
            kind: EventKind::Modify(notify::event::ModifyKind::Data(
                notify::event::DataChange::Content,
            )),
            paths: vec![PathBuf::from("/project/src/main.yx")],
            attrs: Default::default(),
        };

        let changes = classify_events(&[event], Path::new("/project"));
        assert_eq!(changes.len(), 1);
        assert!(matches!(changes[0], FileChange::SourceModified(_)));
    }

    #[test]
    fn test_classify_manifest() {
        let event = Event {
            kind: EventKind::Modify(notify::event::ModifyKind::Data(
                notify::event::DataChange::Content,
            )),
            paths: vec![PathBuf::from("/project/yaoxiang.toml")],
            attrs: Default::default(),
        };

        let changes = classify_events(&[event], Path::new("/project"));
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0], FileChange::ManifestChanged);
    }

    #[test]
    fn test_classify_non_yx_ignored() {
        let event = Event {
            kind: EventKind::Modify(notify::event::ModifyKind::Data(
                notify::event::DataChange::Content,
            )),
            paths: vec![PathBuf::from("/project/README.md")],
            attrs: Default::default(),
        };

        let changes = classify_events(&[event], Path::new("/project"));
        assert!(changes.is_empty());
    }

    #[test]
    fn test_classify_create_delete() {
        let events = vec![
            Event {
                kind: EventKind::Create(notify::event::CreateKind::File),
                paths: vec![PathBuf::from("/project/src/new.yx")],
                attrs: Default::default(),
            },
            Event {
                kind: EventKind::Remove(notify::event::RemoveKind::File),
                paths: vec![PathBuf::from("/project/src/old.yx")],
                attrs: Default::default(),
            },
        ];

        let changes = classify_events(&events, Path::new("/project"));
        assert_eq!(changes.len(), 2);
        assert!(matches!(changes[0], FileChange::SourceCreated(_)));
        assert!(matches!(changes[1], FileChange::SourceDeleted(_)));
    }

    #[test]
    fn test_invalidate_cache_source() {
        let cache = ModuleCache::new(CacheMode::Compile);
        let path = PathBuf::from("/project/src/utils.yx");
        let module = crate::frontend::module::ModuleInfo::new(
            "utils".to_string(),
            crate::frontend::module::ModuleSource::User,
        );
        cache.put("utils", module, Some(&path));

        let changes = vec![FileChange::SourceModified(path)];
        let affected = invalidate_cache(&changes, &cache);

        assert!(affected.contains(&"utils".to_string()));
        assert!(cache.get("utils", None).is_none());
    }

    #[test]
    fn test_invalidate_cache_manifest() {
        let cache = ModuleCache::new(CacheMode::Compile);
        cache.put(
            "mod_a",
            crate::frontend::module::ModuleInfo::new(
                "mod_a".to_string(),
                crate::frontend::module::ModuleSource::User,
            ),
            None,
        );

        let changes = vec![FileChange::ManifestChanged];
        let affected = invalidate_cache(&changes, &cache);

        assert!(affected.contains(&"*".to_string()));
        assert!(cache.get("mod_a", None).is_none());
    }

    #[test]
    fn test_path_to_module_name() {
        assert_eq!(
            path_to_module_name(Path::new("/src/utils.yx")),
            Some("utils".to_string())
        );
        assert_eq!(
            path_to_module_name(Path::new("/src/math/mod.yx")),
            Some("math".to_string())
        );
        assert_eq!(
            path_to_module_name(Path::new("/src/math/index.yx")),
            Some("math".to_string())
        );
    }

    #[test]
    fn test_hot_reload_config_default() {
        let config = HotReloadConfig::default();
        assert_eq!(config.debounce_ms, 300);
        assert!(config.watch_dirs.is_empty());
        assert!(!config.watch_vendor);
    }

    #[test]
    fn test_hot_reload_config_from_project() {
        let dir = tempfile::TempDir::new().unwrap();
        let src_dir = dir.path().join("src");
        std::fs::create_dir_all(&src_dir).unwrap();

        let config = HotReloadConfig::from_project(dir.path());
        assert_eq!(config.debounce_ms, 300);
        assert!(!config.watch_dirs.is_empty());
        assert!(config.watch_dirs.contains(&src_dir));
    }

    #[test]
    fn test_dedup_events() {
        let events = vec![
            Event {
                kind: EventKind::Modify(notify::event::ModifyKind::Data(
                    notify::event::DataChange::Content,
                )),
                paths: vec![PathBuf::from("/project/src/main.yx")],
                attrs: Default::default(),
            },
            Event {
                kind: EventKind::Modify(notify::event::ModifyKind::Data(
                    notify::event::DataChange::Content,
                )),
                paths: vec![PathBuf::from("/project/src/main.yx")],
                attrs: Default::default(),
            },
        ];

        let changes = classify_events(&events, Path::new("/project"));
        // 同一文件应该被去重
        assert_eq!(changes.len(), 1);
    }
}
