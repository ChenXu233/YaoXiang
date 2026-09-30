//! Standard OS library (YaoXiang)
//!
//! This module provides operating system functionality for YaoXiang programs.

use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::{LazyLock, Mutex};

use crate::backends::common::RuntimeValue;
use crate::backends::ExecutorError;
use crate::std::{NativeContext, NativeExport, StdModule};

// OsModule - StdModule Implementation

/// OS module implementation.
#[derive(Default)]
pub struct OsModule;

impl StdModule for OsModule {
    fn module_path(&self) -> &str {
        "std.os"
    }

    fn exports(&self) -> Vec<NativeExport> {
        vec![
            // File operations
            export!(
                "open",
                "std.os.open",
                "(path: &String, mode: &String) -> File",
                native_open
            ),
            export!(
                "close",
                "std.os.close",
                "(file: &File) -> Void",
                native_close
            ),
            export!(
                "read",
                "std.os.read",
                "(file: &File, n: Int) -> String",
                native_read
            ),
            export!(
                "write",
                "std.os.write",
                "(file: &File, content: String) -> Int",
                native_write
            ),
            export!(
                "seek",
                "std.os.seek",
                "(file: &File, offset: Int) -> Bool",
                native_seek
            ),
            export!("tell", "std.os.tell", "(file: &File) -> Int", native_tell),
            export!(
                "flush",
                "std.os.flush",
                "(file: &File) -> Void",
                native_flush
            ),
            // Environment variables
            export!(
                "get_env",
                "std.os.get_env",
                "(name: &String) -> String",
                native_get_env
            ),
            export!(
                "set_env",
                "std.os.set_env",
                "(name: &String, value: &String) -> Void",
                native_set_env
            ),
            // Process and working directory
            export!("args", "std.os.args", "() -> String", native_args),
            export!(
                "chdir",
                "std.os.chdir",
                "(path: &String) -> Bool",
                native_chdir
            ),
            export!("getcwd", "std.os.getcwd", "() -> String", native_getcwd),
        ]
    }
}

/// Singleton instance for std.os module.
pub const OS_MODULE: OsModule = OsModule;

// Global State

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

// File Operations

/// Native implementation: open
fn native_open(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    if args.len() < 2 {
        return Err(ExecutorError::runtime_only(
            "open expects 2 arguments (path: String, mode: String)".to_string(),
        ));
    }

    let path = match &args[0] {
        RuntimeValue::String(s) => s.to_string(),
        other => {
            return Err(ExecutorError::type_only(format!(
                "open expects String path, got {:?}",
                other.value_type(None)
            )))
        }
    };

    let mode = match &args[1] {
        RuntimeValue::String(s) => s.to_string(),
        other => {
            return Err(ExecutorError::type_only(format!(
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
        "a" => OpenOptions::new().append(true).create(true).open(&path),
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
            return Err(ExecutorError::runtime_only(format!(
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
                Err(ExecutorError::runtime_only(
                    "Failed to lock file table".to_string(),
                ))
            }
        }
        Err(e) => Err(ExecutorError::runtime_only(format!(
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
        return Err(ExecutorError::runtime_only(
            "close expects 1 argument (file: File)".to_string(),
        ));
    }

    let fd = match &args[0] {
        RuntimeValue::Int(fd) => *fd,
        other => {
            return Err(ExecutorError::type_only(format!(
                "close expects File (Int) argument, got {:?}",
                other.value_type(None)
            )))
        }
    };

    if let Ok(mut files) = OPEN_FILES.lock() {
        if files.remove(&fd).is_some() {
            Ok(RuntimeValue::Void)
        } else {
            Err(ExecutorError::runtime_only(format!(
                "Invalid file descriptor: {}",
                fd
            )))
        }
    } else {
        Err(ExecutorError::runtime_only(
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
        return Err(ExecutorError::runtime_only(
            "read expects 2 arguments (file: File, n: Int)".to_string(),
        ));
    }

    let fd = match &args[0] {
        RuntimeValue::Int(fd) => *fd,
        other => {
            return Err(ExecutorError::type_only(format!(
                "read expects File (Int) argument, got {:?}",
                other.value_type(None)
            )))
        }
    };

    let n = match &args[1] {
        RuntimeValue::Int(n) => *n as usize,
        other => {
            return Err(ExecutorError::type_only(format!(
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
                Err(e) => Err(ExecutorError::runtime_only(format!(
                    "Failed to read from file: {}",
                    e
                ))),
            }
        } else {
            Err(ExecutorError::runtime_only(format!(
                "Invalid file descriptor: {}",
                fd
            )))
        }
    } else {
        Err(ExecutorError::runtime_only(
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
        return Err(ExecutorError::runtime_only(
            "write expects 2 arguments (file: File, content: String)".to_string(),
        ));
    }

    let fd = match &args[0] {
        RuntimeValue::Int(fd) => *fd,
        other => {
            return Err(ExecutorError::type_only(format!(
                "write expects File (Int) argument, got {:?}",
                other.value_type(None)
            )))
        }
    };

    let content = match &args[1] {
        RuntimeValue::String(s) => s.to_string(),
        other => {
            return Err(ExecutorError::type_only(format!(
                "write expects String content, got {:?}",
                other.value_type(None)
            )))
        }
    };

    if let Ok(mut files) = OPEN_FILES.lock() {
        if let Some(file) = files.get_mut(&fd) {
            match file.write_all(content.as_bytes()) {
                Ok(()) => Ok(RuntimeValue::Int(content.len() as i64)),
                Err(e) => Err(ExecutorError::runtime_only(format!(
                    "Failed to write to file: {}",
                    e
                ))),
            }
        } else {
            Err(ExecutorError::runtime_only(format!(
                "Invalid file descriptor: {}",
                fd
            )))
        }
    } else {
        Err(ExecutorError::runtime_only(
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
        return Err(ExecutorError::runtime_only(
            "seek expects 2 arguments (file: File, offset: Int)".to_string(),
        ));
    }

    let fd = match &args[0] {
        RuntimeValue::Int(fd) => *fd,
        other => {
            return Err(ExecutorError::type_only(format!(
                "seek expects File (Int) argument, got {:?}",
                other.value_type(None)
            )))
        }
    };

    let offset = match &args[1] {
        RuntimeValue::Int(offset) => *offset,
        other => {
            return Err(ExecutorError::type_only(format!(
                "seek expects Int offset, got {:?}",
                other.value_type(None)
            )))
        }
    };

    if let Ok(mut files) = OPEN_FILES.lock() {
        if let Some(file) = files.get_mut(&fd) {
            match file.seek(SeekFrom::Start(offset as u64)) {
                Ok(_) => Ok(RuntimeValue::Bool(true)),
                Err(e) => Err(ExecutorError::runtime_only(format!(
                    "Failed to seek in file: {}",
                    e
                ))),
            }
        } else {
            Err(ExecutorError::runtime_only(format!(
                "Invalid file descriptor: {}",
                fd
            )))
        }
    } else {
        Err(ExecutorError::runtime_only(
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
        return Err(ExecutorError::runtime_only(
            "tell expects 1 argument (file: File)".to_string(),
        ));
    }

    let fd = match &args[0] {
        RuntimeValue::Int(fd) => *fd,
        other => {
            return Err(ExecutorError::type_only(format!(
                "tell expects File (Int) argument, got {:?}",
                other.value_type(None)
            )))
        }
    };

    if let Ok(mut files) = OPEN_FILES.lock() {
        if let Some(file) = files.get_mut(&fd) {
            match file.stream_position() {
                Ok(pos) => Ok(RuntimeValue::Int(pos as i64)),
                Err(e) => Err(ExecutorError::runtime_only(format!(
                    "Failed to get file position: {}",
                    e
                ))),
            }
        } else {
            Err(ExecutorError::runtime_only(format!(
                "Invalid file descriptor: {}",
                fd
            )))
        }
    } else {
        Err(ExecutorError::runtime_only(
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
        return Err(ExecutorError::runtime_only(
            "flush expects 1 argument (file: File)".to_string(),
        ));
    }

    let fd = match &args[0] {
        RuntimeValue::Int(fd) => *fd,
        other => {
            return Err(ExecutorError::type_only(format!(
                "flush expects File (Int) argument, got {:?}",
                other.value_type(None)
            )))
        }
    };

    if let Ok(mut files) = OPEN_FILES.lock() {
        if let Some(file) = files.get_mut(&fd) {
            match file.flush() {
                Ok(()) => Ok(RuntimeValue::Void),
                Err(e) => Err(ExecutorError::runtime_only(format!(
                    "Failed to flush file: {}",
                    e
                ))),
            }
        } else {
            Err(ExecutorError::runtime_only(format!(
                "Invalid file descriptor: {}",
                fd
            )))
        }
    } else {
        Err(ExecutorError::runtime_only(
            "Failed to lock file table".to_string(),
        ))
    }
}

/// Native implementation: get_env
fn native_get_env(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    if args.is_empty() {
        return Err(ExecutorError::runtime_only(
            "get_env expects 1 argument (name: String)".to_string(),
        ));
    }

    let name = match &args[0] {
        RuntimeValue::String(s) => s.to_string(),
        other => {
            return Err(ExecutorError::type_only(format!(
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
        return Err(ExecutorError::runtime_only(
            "set_env expects 2 arguments (name: String, value: String)".to_string(),
        ));
    }

    let name = match &args[0] {
        RuntimeValue::String(s) => s.to_string(),
        other => {
            return Err(ExecutorError::type_only(format!(
                "set_env expects String name, got {:?}",
                other.value_type(None)
            )))
        }
    };

    let value = match &args[1] {
        RuntimeValue::String(s) => s.to_string(),
        other => {
            return Err(ExecutorError::type_only(format!(
                "set_env expects String value, got {:?}",
                other.value_type(None)
            )))
        }
    };

    std::env::set_var(&name, &value);
    Ok(RuntimeValue::Void)
}

// Process and Working Directory

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
        return Err(ExecutorError::runtime_only(
            "chdir expects 1 argument (path: String)".to_string(),
        ));
    }

    let path = match &args[0] {
        RuntimeValue::String(s) => s.to_string(),
        other => {
            return Err(ExecutorError::type_only(format!(
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
                Err(ExecutorError::runtime_only(format!(
                    "Directory does not exist: {}",
                    path
                )))
            }
        }
        Err(e) => Err(ExecutorError::runtime_only(format!(
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
        Err(e) => Err(ExecutorError::runtime_only(format!(
            "Failed to get current directory: {}",
            e
        ))),
    }
}
