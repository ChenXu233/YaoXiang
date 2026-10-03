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

    fn exports(&self) -> Vec<NativeExport> {
        vec![
            // ---- 文件整体读写（自 std.io 迁入，#104）----
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
            // ---- 目录与文件管理（自 std.os 迁入，#104）----
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
            // ---- 遍历与元数据（#104 新增）----
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
            // ---- 临时文件与目录（#104 新增；tempfile 仅原生目标编译）----
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
            // ---- 路径操作（#104 新增）----
            export!(
                "path_join",
                "std.fs.path_join",
                "(base: &String, rel: &String) -> String",
                native_path_join
            ),
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
fn native_read_dir(
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
fn native_walk(
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
fn native_stat(
    args: &[RuntimeValue],
    ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let path = expect_path(args, 0, "stat", "path")?;
    match fs::metadata(&path) {
        Ok(meta) => {
            let mtime = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);

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
                RuntimeValue::Int(mtime),
            );

            Ok(RuntimeValue::Dict(ctx.heap.allocate(HeapValue::Dict(map))))
        }
        Err(e) => Err(ExecutorError::runtime_only(format!(
            "Failed to stat '{}': {}",
            path, e
        ))),
    }
}

// Temporary Files & Directories (#104, native targets only)

/// Native implementation: temp_dir —— 系统临时目录路径。
#[cfg(not(target_arch = "wasm32"))]
fn native_temp_dir(
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
fn native_mkdtemp(
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
fn native_tmpfile(
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
fn native_path_join(
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
fn native_path_basename(
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
fn native_path_dirname(
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
fn native_path_extension(
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

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use crate::backends::common::heap::Heap;

    /// 调用 native 的脚手架：独立 Heap，实参表直传。
    fn call(
        f: fn(&[RuntimeValue], &mut NativeContext<'_>) -> Result<RuntimeValue, ExecutorError>,
        args: &[&str],
    ) -> Result<RuntimeValue, ExecutorError> {
        let mut heap = Heap::new();
        let mut ctx = NativeContext::new(&mut heap);
        let values: Vec<RuntimeValue> = args
            .iter()
            .map(|s| RuntimeValue::String((*s).into()))
            .collect();
        f(&values, &mut ctx)
    }

    fn string_of(value: RuntimeValue) -> String {
        match value {
            RuntimeValue::String(s) => s.to_string(),
            other => panic!("expected String, got {:?}", other),
        }
    }

    fn list_of(value: RuntimeValue) -> Vec<String> {
        let RuntimeValue::List(handle) = value else {
            panic!("expected List");
        };
        let guard = handle.lock();
        match &*guard {
            HeapValue::List(items) => items
                .iter()
                .map(|v| match v {
                    RuntimeValue::String(s) => s.to_string(),
                    other => panic!("list item should be String, got {:?}", other),
                })
                .collect(),
            other => panic!("heap value should be List, got {:?}", other),
        }
    }

    fn dict_entry(
        value: &RuntimeValue,
        key: &str,
    ) -> RuntimeValue {
        let RuntimeValue::Dict(handle) = value else {
            panic!("expected Dict");
        };
        let guard = handle.lock();
        match &*guard {
            HeapValue::Dict(map) => map
                .get(&RuntimeValue::String(key.into()))
                .cloned()
                .unwrap_or_else(|| panic!("missing key '{}'", key)),
            other => panic!("heap value should be Dict, got {:?}", other),
        }
    }

    /// 建一棵固定的目录树：a.txt、b.txt、sub/c.txt，返回根路径字符串。
    ///
    /// 写入顺序刻意与排序结果不同，用例才能分辨「原样列出」与「按名字排序」。
    fn listing_tree(tmp: &tempfile::TempDir) -> String {
        for name in ["b.txt", "a.txt"] {
            std::fs::write(tmp.path().join(name), name)
                .unwrap_or_else(|e| panic!("write {name} failed: {e:?}"));
        }
        let sub = tmp.path().join("sub");
        std::fs::create_dir(&sub).unwrap_or_else(|e| panic!("create_dir sub failed: {e:?}"));
        std::fs::write(sub.join("c.txt"), "c")
            .unwrap_or_else(|e| panic!("write sub/c.txt failed: {e:?}"));
        tmp.path().to_string_lossy().to_string()
    }

    /// root 下 DFS 遍历期望得到的绝对路径（每层按名字排序）。
    ///
    /// 逐段 join：`Path::join("sub/c.txt")` 会把 `/` 原样留在字符串里，
    /// 与被测实现产出的平台分隔符不一致。
    fn expected_walk_paths(root: &Path) -> Vec<String> {
        let sub = root.join("sub");
        vec![
            root.join("a.txt").to_string_lossy().to_string(),
            root.join("b.txt").to_string_lossy().to_string(),
            sub.to_string_lossy().to_string(),
            sub.join("c.txt").to_string_lossy().to_string(),
        ]
    }

    #[test]
    // 本测试拿 `Path::join` 当**语义 oracle**（断言 native 实现与 std 一致），
    // 而 clippy 的 join_absolute_paths 建议会把 `base.join("/abs")` 换成
    // `PathBuf::from("/abs")`——碰巧同结果却抹掉被测的那条语义（绝对实参替换
    // base）。故保留原写法。
    #[allow(clippy::join_absolute_paths)]
    fn path_ops_match_path_semantics() {
        assert_eq!(string_of(call(native_path_join, &["a", "b"]).unwrap()), {
            Path::new("a").join("b").to_string_lossy().to_string()
        });
        // 绝对实参替换 base（Path::join 语义）
        assert_eq!(
            string_of(call(native_path_join, &["a", "/abs"]).unwrap()),
            Path::new("a").join("/abs").to_string_lossy().to_string()
        );
        assert_eq!(
            string_of(call(native_path_basename, &["dir/file.txt"]).unwrap()),
            "file.txt"
        );
        assert_eq!(string_of(call(native_path_basename, &["/"]).unwrap()), "");
        assert_eq!(
            string_of(call(native_path_dirname, &["dir/file.txt"]).unwrap()),
            "dir"
        );
        assert_eq!(
            string_of(call(native_path_dirname, &["a.txt"]).unwrap()),
            ""
        );
        assert_eq!(
            string_of(call(native_path_extension, &["a.tar.gz"]).unwrap()),
            "gz"
        );
        assert_eq!(
            string_of(call(native_path_extension, &["noext"]).unwrap()),
            ""
        );
    }

    #[test]
    fn read_dir_and_walk_are_sorted_lists() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = listing_tree(&tmp);

        let names = list_of(call(native_read_dir, &[&root]).unwrap());
        assert_eq!(
            names,
            vec!["a.txt", "b.txt", "sub"],
            "read_dir sorted names"
        );

        let walked = list_of(call(native_walk, &[&root]).unwrap());
        assert_eq!(
            walked,
            expected_walk_paths(tmp.path()),
            "walk DFS with per-level sort"
        );

        // walk 拒绝非目录
        assert!(call(native_walk, &[&tmp.path().join("a.txt").to_string_lossy()]).is_err());
    }

    #[test]
    fn stat_reports_size_and_kinds() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let file = tmp.path().join("f.txt");
        std::fs::write(&file, "12345").unwrap();
        let dir = tmp.path().join("d");
        std::fs::create_dir(&dir).unwrap();

        let file_path = file.to_string_lossy().to_string();
        let st = call(native_stat, &[&file_path]).unwrap();
        assert_eq!(dict_entry(&st, "size"), RuntimeValue::Int(5));
        assert_eq!(dict_entry(&st, "is_file"), RuntimeValue::Bool(true));
        assert_eq!(dict_entry(&st, "is_dir"), RuntimeValue::Bool(false));
        assert!(matches!(dict_entry(&st, "mtime"), RuntimeValue::Int(_)));

        let dir_path = dir.to_string_lossy().to_string();
        let st = call(native_stat, &[&dir_path]).unwrap();
        assert_eq!(dict_entry(&st, "is_dir"), RuntimeValue::Bool(true));

        assert!(call(
            native_stat,
            &[&tmp.path().join("missing").to_string_lossy()]
        )
        .is_err());
    }

    #[test]
    fn mkdtemp_and_tmpfile_create_and_persist() {
        let dir = string_of(call(native_mkdtemp, &["yxfstest-"]).unwrap());
        assert!(Path::new(&dir).is_dir(), "mkdtemp directory exists");
        assert!(dir.contains("yxfstest-"), "prefix respected");
        std::fs::remove_dir(&dir).unwrap();

        let file = string_of(call(native_tmpfile, &["yxfstest-"]).unwrap());
        assert!(Path::new(&file).is_file(), "tmpfile exists");
        assert!(
            std::fs::read(&file).unwrap().is_empty(),
            "tmpfile starts empty"
        );
        std::fs::remove_file(&file).unwrap();

        assert_eq!(
            string_of(call(native_temp_dir, &[]).unwrap()),
            std::env::temp_dir().to_string_lossy().to_string()
        );
    }
}
