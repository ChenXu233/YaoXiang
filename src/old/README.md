# Old Architecture (Legacy Code)

This directory contains the **legacy architecture** that has been replaced by the new backend abstraction layer.

## What Moved Here

- **`vm/`** - Original virtual machine implementation (superseded by `backends/interpreter/`)

## Why It's Here

This code is **preserved for reference only**. It should **not** be used in new development.

## Migration Path

Old API:
```rust
use yaoxiang::{run, VM, CompiledModule};

let mut vm = VM::new();
vm.execute_module(&compiled)?;
```

New API:
```rust
use yaoxiang::backends::interpreter::Interpreter;

let mut interpreter = Interpreter::new();
interpreter.execute_module(&bytecode_module)?;
```

## Deprecation Status

⚠️ **DEPRECATED** - This code is no longer maintained and will be removed in a future version.

---

*Last Updated: 2026-01-23*
