//! Standard OS library (YaoXiang)
//!
//! This module provides operating system functionality for YaoXiang programs.

use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::{LazyLock, Mutex};

use crate::backends::common::RuntimeValue;
use crate::backends::ExecutorError;
use crate::std::{NativeContext, NativeExport, StdModule};

// ============================================================================
// OsModule - StdModule Implementation
// ============================================================================

/// OS module implementation.
pub struct OsModule;

impl Default for OsModule {
    fn default() -> Self {
        Self
    }
}

impl StdModule for OsModule {
    fn module_path(&self) -> &str {
        "std.os"
    }

    fn exports(&self) -> Vec<NativeExport> {
        vec![
            // File operations
            NativeExport::new(
                "open",
                "std.os.open",
                "(path: String, mode: String) -> File",
                native_open,
            ),
            NativeExport::new(
                "close",
                "std.os.close",
                "(file: File) -> Void",
                native_close,
            ),
            NativeExport::new(
                "read",
                "std.os.read",
                "(file: File, n: Int) -> String",
                native_read,
            ),
            NativeExport::new(
                "write",
                "std.os.write",
                "(file: File, content: String) -> Int",
                native_write,
            ),
            NativeExport::new(
                "seek",
                "std.os.seek",
                "(file: File, offset: Int) -> Bool",
                native_seek,
            ),
            NativeExport::new("tell", "std.os.tell", "(file: File) -> Int", native_tell),
            NativeExport::new(
                "flush",
                "std.os.flush",
                "(file: File) -> Void",
                native_flush,
            ),
            // Directory operations
            NativeExport::new(
                "mkdir",
                "std.os.mkdir",
                "(path: String) -> Bool",
                native_mkdir,
            ),
            NativeExport::new(
                "rmdir",
                "std.os.rmdir",
                "(path: String) -> Bool",
                native_rmdir,
            ),
            NativeExport::new(
                "read_dir",
                "std.os.read_dir",
                "(path: String) -> String",
                native_read_dir,
            ),
            // File/Directory utilities
            NativeExport::new(
                "remove",
                "std.os.remove",
                "(path: String) -> Bool",
                native_remove,
            ),
            NativeExport::new(
                "exists",
                "std.os.exists",
                "(path: String) -> Bool",
                native_exists,
            ),
            NativeExport::new(
                "is_file",
                "std.os.is_file",
                "(path: String) -> Bool",
                native_is_file,
            ),
            NativeExport::new(
                "is_dir",
                "std.os.is_dir",
                "(path: String) -> Bool",
                native_is_dir,
            ),
            NativeExport::new(
                "copy",
                "std.os.copy",
                "(src: String, dst: String) -> Bool",
                native_copy,
            ),
            NativeExport::new(
                "rename",
                "std.os.rename",
                "(old: String, new: String) -> Bool",
                native_rename,
            ),
            // Environment variables
            NativeExport::new(
                "get_env",
                "std.os.get_env",
                "(name: String) -> String",
                native_get_env,
            ),
            NativeExport::new(
                "set_env",
                "std.os.set_env",
                "(name: String, value: String) -> Void",
                native_set_env,
            ),
            // Process and working directory
            NativeExport::new("args", "std.os.args", "() -> String", native_args),
            NativeExport::new(
                "chdir",
                "std.os.chdir",
                "(path: String) -> Bool",
                native_chdir,
            ),
            NativeExport::new("getcwd", "std.os.getcwd", "() -> String", native_getcwd),
            NativeExport::new(
                "append_file",
                "std.os.append_file",
                "(path: String, content: String) -> Bool",
                native_append_file,
            ),
        ]
    }
}

/// Singleton instance for std.os module.
pub const OS_MODULE: OsModule = OsModule;

// ============================================================================
// Global State
// ============================================================================

/// Global file handle storage for open files.
static OPEN_FILES: LazyLock<Mutex<HashMap<i64, File>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Global counter for generating unique file descriptors.
static FILE_DESCRIPTOR_COUNTER: LazyLock<Mutex<i64>> = LazyLock::new(|| Mutex::new(0i64));

/// Allocates a unique file descriptor.
fn allocate_fd() -> i64 {
    if let Ok(mut counter) = FILE_DESCRIPTOR_COUNTER.lock() {
        *counter += 1;
        *counter
    } else {
        0
    }
}

// ============================================================================
// File Operations
// ============================================================================

/// Native implementation: open
fn native_open(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    if args.len() < 2 {
        return Err(ExecutorError::Runtime(
            "open expects 2 arguments (path: String, mode: String)".to_string(),
        ));
    }

    let path = match &args[0] {
        RuntimeValue::String(s) => s.to_string(),
        other => {
            return Err(ExecutorError::Type(format!(
                "open expects String path, got {:?}",
                other.value_type(None)
            )))
        }
    };

    let mode = match &args[1] {
        RuntimeValue::String(s) => s.to_string(),
        other => {
            return Err(ExecutorError::Type(format!(
                "open expects String mode, got {:?}",
                other.value_type(None)
            )))
        }
    };

    let file = match mode.as_str() {
        "r" => OpenOptions::new().read(true).open(&path),
        "w" => OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path),
        "a" => OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .open(&path),
        "r+" => OpenOptions::new().read(true).write(true).open(&path),
        "w+" => OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path),
        "a+" => OpenOptions::new()
            .read(true)
            .append(true)
            .create(true)
            .open(&path),
        _ => {
            return Err(ExecutorError::Runtime(format!(
                "Invalid file mode: {}. Use 'r', 'w', 'a', 'r+', 'w+', 'a+'",
                mode
            )))
        }
    };

    match file {
        Ok(file) => {
            let fd = allocate_fd();
            if let Ok(mut files) = OPEN_FILES.lock() {
                files.insert(fd, file);
                Ok(RuntimeValue::Int(fd))
            } else {
                Err(ExecutorError::Runtime(
                    "Failed to lock file table".to_string(),
                ))
            }
        }
        Err(e) => Err(ExecutorError::Runtime(format!(
            "Failed to open file '{}': {}",
            path, e
        ))),
    }
}

/// Native implementation: close
fn native_close(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    if args.is_empty() {
        return Err(ExecutorError::Runtime(
            "close expects 1 argument (file: File)".to_string(),
        ));
    }

    let fd = match &args[0] {
        RuntimeValue::Int(fd) => *fd,
        other => {
            return Err(ExecutorError::Type(format!(
                "close expects File (Int) argument, got {:?}",
                other.value_type(None)
            )))
        }
    };

    if let Ok(mut files) = OPEN_FILES.lock() {
        if files.remove(&fd).is_some() {
            Ok(RuntimeValue::Unit)
        } else {
            Err(ExecutorError::Runtime(format!(
                "Invalid file descriptor: {}",
                fd
            )))
        }
    } else {
        Err(ExecutorError::Runtime(
            "Failed to lock file table".to_string(),
        ))
    }
}

/// Native implementation: read
fn native_read(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    if args.len() < 2 {
        return Err(ExecutorError::Runtime(
            "read expects 2 arguments (file: File, n: Int)".to_string(),
        ));
    }

    let fd = match &args[0] {
        RuntimeValue::Int(fd) => *fd,
        other => {
            return Err(ExecutorError::Type(format!(
                "read expects File (Int) argument, got {:?}",
                other.value_type(None)
            )))
        }
    };

    let n = match &args[1] {
        RuntimeValue::Int(n) => *n as usize,
        other => {
            return Err(ExecutorError::Type(format!(
                "read expects Int argument, got {:?}",
                other.value_type(None)
            )))
        }
    };

    if let Ok(mut files) = OPEN_FILES.lock() {
        if let Some(file) = files.get_mut(&fd) {
            let mut buffer = vec![0u8; n];
            match file.read(&mut buffer) {
                Ok(bytes_read) => {
                    buffer.truncate(bytes_read);
                    match String::from_utf8(buffer) {
                        Ok(content) => Ok(RuntimeValue::String(content.into())),
                        Err(e) => Ok(RuntimeValue::String(
                            String::from_utf8_lossy(&e.into_bytes()).into(),
                        )),
                    }
                }
                Err(e) => Err(ExecutorError::Runtime(format!(
                    "Failed to read from file: {}",
                    e
                ))),
            }
        } else {
            Err(ExecutorError::Runtime(format!(
                "Invalid file descriptor: {}",
                fd
            )))
        }
    } else {
        Err(ExecutorError::Runtime(
            "Failed to lock file table".to_string(),
        ))
    }
}

/// Native implementation: write
fn native_write(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    if args.len() < 2 {
        return Err(ExecutorError::Runtime(
            "write expects 2 arguments (file: File, content: String)".to_string(),
        ));
    }

    let fd = match &args[0] {
        RuntimeValue::Int(fd) => *fd,
        other => {
            return Err(ExecutorError::Type(format!(
                "write expects File (Int) argument, got {:?}",
                other.value_type(None)
            )))
        }
    };

    let content = match &args[1] {
        RuntimeValue::String(s) => s.to_string(),
        other => {
            return Err(ExecutorError::Type(format!(
                "write expects String content, got {:?}",
                other.value_type(None)
            )))
        }
    };

    if let Ok(mut files) = OPEN_FILES.lock() {
        if let Some(file) = files.get_mut(&fd) {
            match file.write_all(content.as_bytes()) {
                Ok(()) => Ok(RuntimeValue::Int(content.len() as i64)),
                Err(e) => Err(ExecutorError::Runtime(format!(
                    "Failed to write to file: {}",
                    e
                ))),
            }
        } else {
            Err(ExecutorError::Runtime(format!(
                "Invalid file descriptor: {}",
                fd
            )))
        }
    } else {
        Err(ExecutorError::Runtime(
            "Failed to lock file table".to_string(),
        ))
    }
}

/// Native implementation: seek
fn native_seek(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    if args.len() < 2 {
        return Err(ExecutorError::Runtime(
            "seek expects 2 arguments (file: File, offset: Int)".to_string(),
        ));
    }

    let fd = match &args[0] {
        RuntimeValue::Int(fd) => *fd,
        other => {
            return Err(ExecutorError::Type(format!(
                "seek expects File (Int) argument, got {:?}",
                other.value_type(None)
            )))
        }
    };

    let offset = match &args[1] {
        RuntimeValue::Int(offset) => *offset,
        other => {
            return Err(ExecutorError::Type(format!(
                "seek expects Int offset, got {:?}",
                other.value_type(None)
            )))
        }
    };

    if let Ok(mut files) = OPEN_FILES.lock() {
        if let Some(file) = files.get_mut(&fd) {
            match file.seek(SeekFrom::Start(offset as u64)) {
                Ok(_) => Ok(RuntimeValue::Bool(true)),
                Err(e) => Err(ExecutorError::Runtime(format!(
                    "Failed to seek in file: {}",
                    e
                ))),
            }
        } else {
            Err(ExecutorError::Runtime(format!(
                "Invalid file descriptor: {}",
                fd
            )))
        }
    } else {
        Err(ExecutorError::Runtime(
            "Failed to lock file table".to_string(),
        ))
    }
}

/// Native implementation: tell
fn native_tell(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    if args.is_empty() {
        return Err(ExecutorError::Runtime(
            "tell expects 1 argument (file: File)".to_string(),
        ));
    }

    let fd = match &args[0] {
        RuntimeValue::Int(fd) => *fd,
        other => {
            return Err(ExecutorError::Type(format!(
                "tell expects File (Int) argument, got {:?}",
                other.value_type(None)
            )))
        }
    };

    if let Ok(mut files) = OPEN_FILES.lock() {
        if let Some(file) = files.get_mut(&fd) {
            match file.stream_position() {
                Ok(pos) => Ok(RuntimeValue::Int(pos as i64)),
                Err(e) => Err(ExecutorError::Runtime(format!(
                    "Failed to get file position: {}",
                    e
                ))),
            }
        } else {
            Err(ExecutorError::Runtime(format!(
                "Invalid file descriptor: {}",
                fd
            )))
        }
    } else {
        Err(ExecutorError::Runtime(
            "Failed to lock file table".to_string(),
        ))
    }
}

/// Native implementation: flush
fn native_flush(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    if args.is_empty() {
        return Err(ExecutorError::Runtime(
            "flush expects 1 argument (file: File)".to_string(),
        ));
    }

    let fd = match &args[0] {
        RuntimeValue::Int(fd) => *fd,
        other => {
            return Err(ExecutorError::Type(format!(
                "flush expects File (Int) argument, got {:?}",
                other.value_type(None)
            )))
        }
    };

    if let Ok(mut files) = OPEN_FILES.lock() {
        if let Some(file) = files.get_mut(&fd) {
            match file.flush() {
                Ok(()) => Ok(RuntimeValue::Unit),
                Err(e) => Err(ExecutorError::Runtime(format!(
                    "Failed to flush file: {}",
                    e
                ))),
            }
        } else {
            Err(ExecutorError::Runtime(format!(
                "Invalid file descriptor: {}",
                fd
            )))
        }
    } else {
        Err(ExecutorError::Runtime(
            "Failed to lock file table".to_string(),
        ))
    }
}

// ============================================================================
// Directory Operations
// ============================================================================

/// Native implementation: mkdir
fn native_mkdir(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    if args.is_empty() {
        return Err(ExecutorError::Runtime(
            "mkdir expects 1 argument (path: String)".to_string(),
        ));
    }

    let path = match &args[0] {
        RuntimeValue::String(s) => s.to_string(),
        other => {
            return Err(ExecutorError::Type(format!(
                "mkdir expects String argument, got {:?}",
                other.value_type(None)
            )))
        }
    };

    match fs::create_dir(&path) {
        Ok(()) => Ok(RuntimeValue::Bool(true)),
        Err(e) => Err(ExecutorError::Runtime(format!(
            "Failed to create directory '{}': {}",
            path, e
        ))),
    }
}

/// Native implementation: rmdir
fn native_rmdir(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    if args.is_empty() {
        return Err(ExecutorError::Runtime(
            "rmdir expects 1 argument (path: String)".to_string(),
        ));
    }

    let path = match &args[0] {
        RuntimeValue::String(s) => s.to_string(),
        other => {
            return Err(ExecutorError::Type(format!(
                "rmdir expects String argument, got {:?}",
                other.value_type(None)
            )))
        }
    };

    match fs::remove_dir(&path) {
        Ok(()) => Ok(RuntimeValue::Bool(true)),
        Err(e) => Err(ExecutorError::Runtime(format!(
            "Failed to remove directory '{}': {}",
            path, e
        ))),
    }
}

/// Native implementation: read_dir
fn native_read_dir(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    if args.is_empty() {
        return Err(ExecutorError::Runtime(
            "read_dir expects 1 argument (path: String)".to_string(),
        ));
    }

    let path = match &args[0] {
        RuntimeValue::String(s) => s.to_string(),
        other => {
            return Err(ExecutorError::Type(format!(
                "read_dir expects String argument, got {:?}",
                other.value_type(None)
            )))
        }
    };

    match fs::read_dir(&path) {
        Ok(entries) => {
            let names: Vec<String> = entries
                .filter_map(|entry| {
                    entry
                        .ok()
                        .and_then(|e| e.file_name().to_str().map(|s| s.to_string()))
                })
                .collect();
            Ok(RuntimeValue::String(names.join("\n").into()))
        }
        Err(e) => Err(ExecutorError::Runtime(format!(
            "Failed to read directory '{}': {}",
            path, e
        ))),
    }
}

// ============================================================================
// File/Directory Utilities
// ============================================================================

/// Native implementation: remove
fn native_remove(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    if args.is_empty() {
        return Err(ExecutorError::Runtime(
            "remove expects 1 argument (path: String)".to_string(),
        ));
    }

    let path = match &args[0] {
        RuntimeValue::String(s) => s.to_string(),
        other => {
            return Err(ExecutorError::Type(format!(
                "remove expects String argument, got {:?}",
                other.value_type(None)
            )))
        }
    };

    match fs::remove_file(&path) {
        Ok(()) => Ok(RuntimeValue::Bool(true)),
        Err(e) => Err(ExecutorError::Runtime(format!(
            "Failed to remove file '{}': {}",
            path, e
        ))),
    }
}

/// Native implementation: exists
fn native_exists(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    if args.is_empty() {
        return Err(ExecutorError::Runtime(
            "exists expects 1 argument (path: String)".to_string(),
        ));
    }

    let path = match &args[0] {
        RuntimeValue::String(s) => s.to_string(),
        other => {
            return Err(ExecutorError::Type(format!(
                "exists expects String argument, got {:?}",
                other.value_type(None)
            )))
        }
    };

    Ok(RuntimeValue::Bool(Path::new(&path).exists()))
}

/// Native implementation: is_file
fn native_is_file(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    if args.is_empty() {
        return Err(ExecutorError::Runtime(
            "is_file expects 1 argument (path: String)".to_string(),
        ));
    }

    let path = match &args[0] {
        RuntimeValue::String(s) => s.to_string(),
        other => {
            return Err(ExecutorError::Type(format!(
                "is_file expects String argument, got {:?}",
                other.value_type(None)
            )))
        }
    };

    Ok(RuntimeValue::Bool(Path::new(&path).is_file()))
}

/// Native implementation: is_dir
fn native_is_dir(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    if args.is_empty() {
        return Err(ExecutorError::Runtime(
            "is_dir expects 1 argument (path: String)".to_string(),
        ));
    }

    let path = match &args[0] {
        RuntimeValue::String(s) => s.to_string(),
        other => {
            return Err(ExecutorError::Type(format!(
                "is_dir expects String argument, got {:?}",
                other.value_type(None)
            )))
        }
    };

    Ok(RuntimeValue::Bool(Path::new(&path).is_dir()))
}

/// Native implementation: copy
fn native_copy(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    if args.len() < 2 {
        return Err(ExecutorError::Runtime(
            "copy expects 2 arguments (src: String, dst: String)".to_string(),
        ));
    }

    let src = match &args[0] {
        RuntimeValue::String(s) => s.to_string(),
        other => {
            return Err(ExecutorError::Type(format!(
                "copy expects String src, got {:?}",
                other.value_type(None)
            )))
        }
    };

    let dst = match &args[1] {
        RuntimeValue::String(s) => s.to_string(),
        other => {
            return Err(ExecutorError::Type(format!(
                "copy expects String dst, got {:?}",
                other.value_type(None)
            )))
        }
    };

    match fs::copy(&src, &dst) {
        Ok(_) => Ok(RuntimeValue::Bool(true)),
        Err(e) => Err(ExecutorError::Runtime(format!(
            "Failed to copy file from '{}' to '{}': {}",
            src, dst, e
        ))),
    }
}

/// Native implementation: rename
fn native_rename(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    if args.len() < 2 {
        return Err(ExecutorError::Runtime(
            "rename expects 2 arguments (old: String, new: String)".to_string(),
        ));
    }

    let old = match &args[0] {
        RuntimeValue::String(s) => s.to_string(),
        other => {
            return Err(ExecutorError::Type(format!(
                "rename expects String old, got {:?}",
                other.value_type(None)
            )))
        }
    };

    let new = match &args[1] {
        RuntimeValue::String(s) => s.to_string(),
        other => {
            return Err(ExecutorError::Type(format!(
                "rename expects String new, got {:?}",
                other.value_type(None)
            )))
        }
    };

    match fs::rename(&old, &new) {
        Ok(()) => Ok(RuntimeValue::Bool(true)),
        Err(e) => Err(ExecutorError::Runtime(format!(
            "Failed to rename '{}' to '{}': {}",
            old, new, e
        ))),
    }
}

// ============================================================================
// Environment Variables
// ============================================================================

/// Native implementation: get_env
fn native_get_env(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    if args.is_empty() {
        return Err(ExecutorError::Runtime(
            "get_env expects 1 argument (name: String)".to_string(),
        ));
    }

    let name = match &args[0] {
        RuntimeValue::String(s) => s.to_string(),
        other => {
            return Err(ExecutorError::Type(format!(
                "get_env expects String argument, got {:?}",
                other.value_type(None)
            )))
        }
    };

    match std::env::var(&name) {
        Ok(value) => Ok(RuntimeValue::String(value.into())),
        Err(_) => Ok(RuntimeValue::String("".into())),
    }
}

/// Native implementation: set_env
fn native_set_env(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    if args.len() < 2 {
        return Err(ExecutorError::Runtime(
            "set_env expects 2 arguments (name: String, value: String)".to_string(),
        ));
    }

    let name = match &args[0] {
        RuntimeValue::String(s) => s.to_string(),
        other => {
            return Err(ExecutorError::Type(format!(
                "set_env expects String name, got {:?}",
                other.value_type(None)
            )))
        }
    };

    let value = match &args[1] {
        RuntimeValue::String(s) => s.to_string(),
        other => {
            return Err(ExecutorError::Type(format!(
                "set_env expects String value, got {:?}",
                other.value_type(None)
            )))
        }
    };

    std::env::set_var(&name, &value);
    Ok(RuntimeValue::Unit)
}

// ============================================================================
// Process and Working Directory
// ============================================================================

/// Native implementation: args
fn native_args(
    _args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let args: Vec<String> = std::env::args().collect();
    Ok(RuntimeValue::String(args.join("\n").into()))
}

/// Native implementation: chdir
fn native_chdir(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    if args.is_empty() {
        return Err(ExecutorError::Runtime(
            "chdir expects 1 argument (path: String)".to_string(),
        ));
    }

    let path = match &args[0] {
        RuntimeValue::String(s) => s.to_string(),
        other => {
            return Err(ExecutorError::Type(format!(
                "chdir expects String argument, got {:?}",
                other.value_type(None)
            )))
        }
    };

    match std::env::current_dir() {
        Ok(_cwd) => {
            if Path::new(&path).is_dir() {
                Ok(RuntimeValue::Bool(true))
            } else {
                Err(ExecutorError::Runtime(format!(
                    "Directory does not exist: {}",
                    path
                )))
            }
        }
        Err(e) => Err(ExecutorError::Runtime(format!(
            "Failed to get current directory: {}",
            e
        ))),
    }
}

/// Native implementation: getcwd
fn native_getcwd(
    _args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    match std::env::current_dir() {
        Ok(path) => Ok(RuntimeValue::String(path.to_string_lossy().into())),
        Err(e) => Err(ExecutorError::Runtime(format!(
            "Failed to get current directory: {}",
            e
        ))),
    }
}

/// Native implementation: append_file
fn native_append_file(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    if args.len() < 2 {
        return Err(ExecutorError::Runtime(
            "append_file expects 2 arguments (path: String, content: String)".to_string(),
        ));
    }

    let path = match &args[0] {
        RuntimeValue::String(s) => s.to_string(),
        other => {
            return Err(ExecutorError::Type(format!(
                "append_file expects String path, got {:?}",
                other.value_type(None)
            )))
        }
    };

    let content = match &args[1] {
        RuntimeValue::String(s) => s.to_string(),
        other => {
            return Err(ExecutorError::Type(format!(
                "append_file expects String content, got {:?}",
                other.value_type(None)
            )))
        }
    };

    match OpenOptions::new().append(true).create(true).open(&path) {
        Ok(mut file) => match file.write_all(content.as_bytes()) {
            Ok(()) => Ok(RuntimeValue::Bool(true)),
            Err(e) => Err(ExecutorError::Runtime(format!(
                "Failed to append to file '{}': {}",
                path, e
            ))),
        },
        Err(e) => Err(ExecutorError::Runtime(format!(
            "Failed to open file '{}' for appending: {}",
            path, e
        ))),
    }
}
