//! Standard File System library (YaoXiang)
//!
//! 路径级文件系统操作（#104）：文件读写、目录遍历、元数据、临时文件与路径操作。
//!
//! 职责分界（对齐 Rust `std::fs` 与 `std::os` 的分工）：
//! - `std.fs` —— 路径级操作（整文件读写、目录、元数据、路径运算）
//! - `std.os` —— 句柄级操作（open/read/seek 流式访问）与进程环境（env/args/cwd）
//! - `std.io` —— 控制台（print/println/read_line）
//!
//! 本模块在 `wasm32` 目标上不注册（无文件系统语义）；`mkdtemp`/`tmpfile`
//! 依赖 tempfile crate，同样仅原生目标可用。

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::UNIX_EPOCH;

use crate::backends::common::{RuntimeValue, HeapValue};
use crate::backends::ExecutorError;
use crate::std::{NativeContext, NativeExport, StdModule};

// FsModule - StdModule Implementation

/// FS module implementation.
#[derive(Default)]
pub struct FsModule;

impl StdModule for FsModule {
    fn module_path(&self) -> &str {
        "std.fs"
    }

    /// 按职责分段的导出表：分段顺序即导出顺序（见各 `exports_*` 的文档注释）。
    fn exports(&self) -> Vec<NativeExport> {
        let mut all = exports_file_io();
        all.extend(exports_path_predicates());
        all.extend(exports_dir_ops());
        all.extend(exports_file_entry_ops());
        all.extend(exports_traversal());
        all.extend(exports_temp_ops());
        // `// ---- 路径操作` 段首项：单项不值得单独成组，就地入表。
        all.push(export!(
            "path_join",
            "std.fs.path_join",
            "(base: &String, rel: &String) -> String",
            native_path_join
        ));
        all.extend(exports_path_parts());
        all
    }
}

// `std.fs` 导出表按职责分段（原 `exports()` 内 5 条 `// ----` 分段；
// 单段过长者按成员语义再细分，顺序不变）。

/// `std.fs` 导出：文件整体读写（自 `std.io` 迁入，#104）。
fn exports_file_io() -> Vec<NativeExport> {
    vec![
        export!(
            "read_file",
            "std.fs.read_file",
            "(path: &String) -> String",
            native_read_file
        ),
        export!(
            "write_file",
            "std.fs.write_file",
            "(path: &String, content: &String) -> Bool",
            native_write_file
        ),
        export!(
            "append_file",
            "std.fs.append_file",
            "(path: &String, content: &String) -> Bool",
            native_append_file
        ),
    ]
}

/// `std.fs` 导出：存在性与类型查询（`// ---- 目录与文件管理` 段之一）。
fn exports_path_predicates() -> Vec<NativeExport> {
    vec![
        export!(
            "exists",
            "std.fs.exists",
            "(path: &String) -> Bool",
            native_exists
        ),
        export!(
            "is_file",
            "std.fs.is_file",
            "(path: &String) -> Bool",
            native_is_file
        ),
        export!(
            "is_dir",
            "std.fs.is_dir",
            "(path: &String) -> Bool",
            native_is_dir
        ),
    ]
}

/// `std.fs` 导出：目录的创建与删除（`// ---- 目录与文件管理` 段之二）。
fn exports_dir_ops() -> Vec<NativeExport> {
    vec![
        export!(
            "mkdir",
            "std.fs.mkdir",
            "(path: &String) -> Bool",
            native_mkdir
        ),
        export!(
            "mkdir_all",
            "std.fs.mkdir_all",
            "(path: &String) -> Bool",
            native_mkdir_all
        ),
        export!(
            "rmdir",
            "std.fs.rmdir",
            "(path: &String) -> Bool",
            native_rmdir
        ),
    ]
}

/// `std.fs` 导出：条目的删除、复制与改名（`// ---- 目录与文件管理` 段之三）。
fn exports_file_entry_ops() -> Vec<NativeExport> {
    vec![
        export!(
            "remove",
            "std.fs.remove",
            "(path: &String) -> Bool",
            native_remove
        ),
        export!(
            "copy",
            "std.fs.copy",
            "(src: &String, dst: &String) -> Bool",
            native_copy
        ),
        export!(
            "rename",
            "std.fs.rename",
            "(src: &String, dst: &String) -> Bool",
            native_rename
        ),
    ]
}

/// `std.fs` 导出：遍历与元数据（#104 新增）。
fn exports_traversal() -> Vec<NativeExport> {
    vec![
        export!(
            "read_dir",
            "std.fs.read_dir",
            "(path: &String) -> Vec(String)",
            native_read_dir
        ),
        export!(
            "walk",
            "std.fs.walk",
            "(path: &String) -> Vec(String)",
            native_walk
        ),
        export!(
            "stat",
            "std.fs.stat",
            "(path: &String) -> Dict(String, Any)",
            native_stat
        ),
    ]
}

/// `std.fs` 导出：临时文件与目录（#104 新增；`tempfile` 仅原生目标编译）。
fn exports_temp_ops() -> Vec<NativeExport> {
    vec![
        #[cfg(not(target_arch = "wasm32"))]
        export!(
            "temp_dir",
            "std.fs.temp_dir",
            "() -> String",
            native_temp_dir
        ),
        #[cfg(not(target_arch = "wasm32"))]
        export!(
            "mkdtemp",
            "std.fs.mkdtemp",
            "(prefix: &String) -> String",
            native_mkdtemp
        ),
        #[cfg(not(target_arch = "wasm32"))]
        export!(
            "tmpfile",
            "std.fs.tmpfile",
            "(prefix: &String) -> String",
            native_tmpfile
        ),
    ]
}

/// `std.fs` 导出：路径分量拆解（`// ---- 路径操作` 段；段首项 `path_join` 在 `exports()` 内就地入表）。
fn exports_path_parts() -> Vec<NativeExport> {
    vec![
        export!(
            "path_basename",
            "std.fs.path_basename",
            "(path: &String) -> String",
            native_path_basename
        ),
        export!(
            "path_dirname",
            "std.fs.path_dirname",
            "(path: &String) -> String",
            native_path_dirname
        ),
        export!(
            "path_extension",
            "std.fs.path_extension",
            "(path: &String) -> String",
            native_path_extension
        ),
    ]
}

/// Singleton instance for std.fs module.
pub const FS_MODULE: FsModule = FsModule;

// Shared Helpers

/// 提取 String 位置参数（统一报错措辞）。
fn expect_path(
    args: &[RuntimeValue],
    index: usize,
    fname: &str,
    param: &str,
) -> Result<String, ExecutorError> {
    match args.get(index) {
        Some(RuntimeValue::String(s)) => Ok(s.to_string()),
        Some(other) => Err(ExecutorError::type_only(format!(
            "{} expects {} as String, got {:?}",
            fname,
            param,
            other.value_type(None)
        ))),
        None => Err(ExecutorError::runtime_only(format!(
            "{} expects {} argument (path: String)",
            fname, param
        ))),
    }
}

// File Read/Write Functions (moved from std.io, #104)

/// Native implementation: read_file
fn native_read_file(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let path = expect_path(args, 0, "read_file", "path")?;
    match fs::read_to_string(&path) {
        Ok(content) => Ok(RuntimeValue::String(content.into())),
        Err(e) => Err(ExecutorError::runtime_only(format!(
            "Failed to read file '{}': {}",
            path, e
        ))),
    }
}

/// Native implementation: write_file
fn native_write_file(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let path = expect_path(args, 0, "write_file", "path")?;
    let content = expect_path(args, 1, "write_file", "content")?;
    match fs::write(&path, &content) {
        Ok(()) => Ok(RuntimeValue::Bool(true)),
        Err(e) => Err(ExecutorError::runtime_only(format!(
            "Failed to write file '{}': {}",
            path, e
        ))),
    }
}

/// Native implementation: append_file
fn native_append_file(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    use std::io::Write;

    let path = expect_path(args, 0, "append_file", "path")?;
    let content = expect_path(args, 1, "append_file", "content")?;
    match fs::OpenOptions::new().append(true).create(true).open(&path) {
        Ok(mut file) => match file.write_all(content.as_bytes()) {
            Ok(()) => Ok(RuntimeValue::Bool(true)),
            Err(e) => Err(ExecutorError::runtime_only(format!(
                "Failed to append to file '{}': {}",
                path, e
            ))),
        },
        Err(e) => Err(ExecutorError::runtime_only(format!(
            "Failed to open file '{}' for appending: {}",
            path, e
        ))),
    }
}

// Directory & File Management (moved from std.os, #104)

/// Native implementation: exists
fn native_exists(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let path = expect_path(args, 0, "exists", "path")?;
    Ok(RuntimeValue::Bool(Path::new(&path).exists()))
}

/// Native implementation: is_file
fn native_is_file(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let path = expect_path(args, 0, "is_file", "path")?;
    Ok(RuntimeValue::Bool(Path::new(&path).is_file()))
}

/// Native implementation: is_dir
fn native_is_dir(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let path = expect_path(args, 0, "is_dir", "path")?;
    Ok(RuntimeValue::Bool(Path::new(&path).is_dir()))
}

/// Native implementation: mkdir（单层，父目录必须已存在）
fn native_mkdir(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let path = expect_path(args, 0, "mkdir", "path")?;
    match fs::create_dir(&path) {
        Ok(()) => Ok(RuntimeValue::Bool(true)),
        Err(e) => Err(ExecutorError::runtime_only(format!(
            "Failed to create directory '{}': {}",
            path, e
        ))),
    }
}

/// Native implementation: mkdir_all（递归创建缺失的父目录）
fn native_mkdir_all(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let path = expect_path(args, 0, "mkdir_all", "path")?;
    match fs::create_dir_all(&path) {
        Ok(()) => Ok(RuntimeValue::Bool(true)),
        Err(e) => Err(ExecutorError::runtime_only(format!(
            "Failed to create directory tree '{}': {}",
            path, e
        ))),
    }
}

/// Native implementation: rmdir（仅空目录）
fn native_rmdir(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let path = expect_path(args, 0, "rmdir", "path")?;
    match fs::remove_dir(&path) {
        Ok(()) => Ok(RuntimeValue::Bool(true)),
        Err(e) => Err(ExecutorError::runtime_only(format!(
            "Failed to remove directory '{}': {}",
            path, e
        ))),
    }
}

/// Native implementation: remove（删除文件）
fn native_remove(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let path = expect_path(args, 0, "remove", "path")?;
    match fs::remove_file(&path) {
        Ok(()) => Ok(RuntimeValue::Bool(true)),
        Err(e) => Err(ExecutorError::runtime_only(format!(
            "Failed to remove file '{}': {}",
            path, e
        ))),
    }
}

/// Native implementation: copy
fn native_copy(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let src = expect_path(args, 0, "copy", "src")?;
    let dst = expect_path(args, 1, "copy", "dst")?;
    match fs::copy(&src, &dst) {
        Ok(_) => Ok(RuntimeValue::Bool(true)),
        Err(e) => Err(ExecutorError::runtime_only(format!(
            "Failed to copy '{}' to '{}': {}",
            src, dst, e
        ))),
    }
}

/// Native implementation: rename
fn native_rename(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let src = expect_path(args, 0, "rename", "src")?;
    let dst = expect_path(args, 1, "rename", "dst")?;
    match fs::rename(&src, &dst) {
        Ok(()) => Ok(RuntimeValue::Bool(true)),
        Err(e) => Err(ExecutorError::runtime_only(format!(
            "Failed to rename '{}' to '{}': {}",
            src, dst, e
        ))),
    }
}

// Traversal & Metadata (#104)

/// Native implementation: read_dir —— 返回目录项名称 List（按名称排序）。
///
/// 注意与 std.os.read_dir 的历史差异：os 版本返回 "\n" 连接的 String，
/// 迁入 std.fs 时升级为类型正确的 List<String>。
pub(crate) fn native_read_dir(
    args: &[RuntimeValue],
    ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let path = expect_path(args, 0, "read_dir", "path")?;
    match fs::read_dir(&path) {
        Ok(entries) => {
            let mut names: Vec<String> = entries
                .filter_map(|entry| {
                    entry
                        .ok()
                        .and_then(|e| e.file_name().to_str().map(|s| s.to_string()))
                })
                .collect();
            names.sort();
            let values = names
                .into_iter()
                .map(|s| RuntimeValue::String(s.into()))
                .collect();
            Ok(RuntimeValue::List(
                ctx.heap.allocate(HeapValue::List(values)),
            ))
        }
        Err(e) => Err(ExecutorError::runtime_only(format!(
            "Failed to read directory '{}': {}",
            path, e
        ))),
    }
}

/// Native implementation: walk —— 递归遍历，返回全部条目的完整路径 List。
///
/// 每层按名称排序（确定性输出）；符号链接本身作为条目列出但不深入
/// （与 walkdir 默认一致，避免 Windows 目录联接导致的环）。
pub(crate) fn native_walk(
    args: &[RuntimeValue],
    ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let path = expect_path(args, 0, "walk", "path")?;
    let root = Path::new(&path);
    if !root.is_dir() {
        return Err(ExecutorError::runtime_only(format!(
            "walk expects a directory, '{}' is not a directory",
            path
        )));
    }

    let mut paths = Vec::new();
    walk_recursive(root, &mut paths)
        .map_err(|e| ExecutorError::runtime_only(format!("Failed to walk '{}': {}", path, e)))?;
    let values = paths
        .into_iter()
        .map(|s| RuntimeValue::String(s.into()))
        .collect();
    Ok(RuntimeValue::List(
        ctx.heap.allocate(HeapValue::List(values)),
    ))
}

fn walk_recursive(
    dir: &Path,
    out: &mut Vec<String>,
) -> std::io::Result<()> {
    let mut entries: Vec<std::fs::DirEntry> = fs::read_dir(dir)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
        out.push(entry.path().to_string_lossy().to_string());
        if is_dir {
            walk_recursive(&entry.path(), out)?;
        }
    }
    Ok(())
}

/// Native implementation: stat —— 文件元数据字典。
///
/// 键集合：`size`（字节，Int）、`is_dir`、`is_file`、`readonly`（Bool）、
/// `mtime`（Unix 秒，Int；早于纪元/不可得时为 0）。
pub(crate) fn native_stat(
    args: &[RuntimeValue],
    ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let path = expect_path(args, 0, "stat", "path")?;
    match fs::metadata(&path) {
        Ok(meta) => {
            let map = metadata_to_dict(&meta);
            Ok(RuntimeValue::Dict(ctx.heap.allocate(HeapValue::Dict(map))))
        }
        Err(e) => Err(ExecutorError::runtime_only(format!(
            "Failed to stat '{}': {}",
            path, e
        ))),
    }
}

/// `stat` 的 `mtime` 取值：Unix 秒；早于纪元或不可得时为 0。
fn metadata_mtime_secs(meta: &fs::Metadata) -> i64 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// 把 `fs::Metadata` 映射为 `stat` 的键值字典（键集合见 `native_stat` 文档注释）。
fn metadata_to_dict(meta: &fs::Metadata) -> HashMap<RuntimeValue, RuntimeValue> {
    let mut map: HashMap<RuntimeValue, RuntimeValue> = HashMap::new();
    map.insert(
        RuntimeValue::String("size".into()),
        RuntimeValue::Int(meta.len() as i64),
    );
    map.insert(
        RuntimeValue::String("is_dir".into()),
        RuntimeValue::Bool(meta.is_dir()),
    );
    map.insert(
        RuntimeValue::String("is_file".into()),
        RuntimeValue::Bool(meta.is_file()),
    );
    map.insert(
        RuntimeValue::String("readonly".into()),
        RuntimeValue::Bool(meta.permissions().readonly()),
    );
    map.insert(
        RuntimeValue::String("mtime".into()),
        RuntimeValue::Int(metadata_mtime_secs(meta)),
    );
    map
}

// Temporary Files & Directories (#104, native targets only)

/// Native implementation: temp_dir —— 系统临时目录路径。
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn native_temp_dir(
    _args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    Ok(RuntimeValue::String(
        std::env::temp_dir().to_string_lossy().to_string().into(),
    ))
}

/// Native implementation: mkdtemp —— 创建临时目录并返回其路径。
///
/// 目录**不会**被自动回收（调用方用完自行 rmdir）——脚本语义下
/// 显式生命周期比 drop 钩子可预测。
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn native_mkdtemp(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let prefix = expect_path(args, 0, "mkdtemp", "prefix")?;
    match tempfile::Builder::new().prefix(&prefix).tempdir() {
        Ok(dir) => {
            let path = dir.keep();
            Ok(RuntimeValue::String(
                path.to_string_lossy().to_string().into(),
            ))
        }
        Err(e) => Err(ExecutorError::runtime_only(format!(
            "Failed to create temp directory with prefix '{}': {}",
            prefix, e
        ))),
    }
}

/// Native implementation: tmpfile —— 创建空临时文件并返回其路径。
///
/// 同 `mkdtemp`：不自动回收，调用方用完自行 remove。
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn native_tmpfile(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let prefix = expect_path(args, 0, "tmpfile", "prefix")?;
    match tempfile::Builder::new().prefix(&prefix).tempfile() {
        Ok(file) => match file.keep() {
            Ok((_file, path)) => Ok(RuntimeValue::String(
                path.to_string_lossy().to_string().into(),
            )),
            Err(e) => Err(ExecutorError::runtime_only(format!(
                "Failed to persist temp file with prefix '{}': {}",
                prefix, e
            ))),
        },
        Err(e) => Err(ExecutorError::runtime_only(format!(
            "Failed to create temp file with prefix '{}': {}",
            prefix, e
        ))),
    }
}

// Path Operations (#104)

/// Native implementation: path_join —— `Path::join` 语义：
/// `rel` 为绝对路径时直接替换 `base`（与 Rust/Python 一致）。
pub(crate) fn native_path_join(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let base = expect_path(args, 0, "path_join", "base")?;
    let rel = expect_path(args, 1, "path_join", "rel")?;
    Ok(RuntimeValue::String(
        Path::new(&base)
            .join(&rel)
            .to_string_lossy()
            .to_string()
            .into(),
    ))
}

/// Native implementation: path_basename —— 最终分量；无分量（如 "/"、".."）返回 ""。
pub(crate) fn native_path_basename(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let path = expect_path(args, 0, "path_basename", "path")?;
    let name = Path::new(&path)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    Ok(RuntimeValue::String(name.into()))
}

/// Native implementation: path_dirname —— 目录部分；无父目录（如 "a.txt"）返回 ""。
pub(crate) fn native_path_dirname(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let path = expect_path(args, 0, "path_dirname", "path")?;
    let parent = Path::new(&path)
        .parent()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    Ok(RuntimeValue::String(parent.into()))
}

/// Native implementation: path_extension —— 不含点的扩展名；无扩展名返回 ""。
pub(crate) fn native_path_extension(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let path = expect_path(args, 0, "path_extension", "path")?;
    let ext = Path::new(&path)
        .extension()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    Ok(RuntimeValue::String(ext.into()))
}
