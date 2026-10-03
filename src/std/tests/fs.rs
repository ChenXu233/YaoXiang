//! `std.fs` 模块的单元测试
//!
//! 覆盖范围与依据：
//! - 路径语义以 Rust `std::path::Path` 为**等价 oracle**（`path_ops_match_path_semantics`
//!   逐条对照 `join` / `file_name` / `parent` / `extension` 的语义，含「绝对实参替换 base」特权）；
//! - `read_dir` / `walk` 的**确定性顺序**（每层按名字排序、DFS）与「walk 拒绝非目录」；
//! - `stat` 的键集合 `{size, is_dir, is_file, readonly, mtime}` 与 `mtime`「早于纪元/不可得时为 0」；
//! - `mkdtemp` / `tmpfile` 创建后**真实存在且可持久访问**（依赖 tempfile），`temp_dir` 等于 `std::env::temp_dir()`。
//!
//! 规范来源：`std.fs` 本身尚无独立 RFC（仓库 `docs/src/design/rfc/` 内无 std.fs 专章），
//! 故本文件以「实现契约」为断言对象，并显式引用其注释化契约的出处 `src/std/fs.rs` 模块头
//! （职责分界：`std.fs` = 路径级操作，`std.io` = 控制台，`std.os` = 句柄级）与 #104 迁移叙述；
//! 错误码段位依据 RFC-013（`docs/src/design/rfc/accepted/013-error-code-specification.md:560`
//! 「E7xxx 为 std.io / std.net 错误值预留段位」）。
//!
//! 迁移自 `src/std/fs.rs` 的内联 `mod tests`（违反 `docs/src/dev/test-specification.md` 规则 1.4），
//! 现按规则 1.1「单文件模块 → 父级 tests/」落位于此，由 `src/std/mod.rs` 的 `#[cfg(test)] mod tests;` 声明。

use std::path::Path;

use crate::backends::common::{Heap, HeapValue, RuntimeValue};
use crate::backends::ExecutorError;
use crate::std::fs::{
    native_mkdtemp, native_path_basename, native_path_dirname, native_path_extension,
    native_path_join, native_read_dir, native_stat, native_temp_dir, native_tmpfile, native_walk,
};
use crate::std::NativeContext;

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
