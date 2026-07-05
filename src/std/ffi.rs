//! Standard FFI library (YaoXiang)
//!
//! This module provides the Foreign Function Interface (FFI) for YaoXiang,
//! allowing users to declare and use native (Rust) functions from YaoXiang code.
//!
//! # Architecture
//!
//! ```text
//! ┌───────────────────────────────────────────────────────────┐
//! │  YaoXiang Source                                          │
//! │                                                           │
//! │  my_add: (a: Int, b: Int) -> Int = native("my_add")      │
//! │                │                                          │
//! │  ┌─────────────┘                                          │
//! │  │                                                        │
//! │  ▼  Compile Time                                          │
//! │  ┌──────────────────────────────────────┐                 │
//! │  │ IR Gen: detect native("my_add")      │                 │
//! │  │ → resolved name == std.ffi.native    │                 │
//! │  │ → create NativeBinding               │                 │
//! │  │ → skip function body generation      │                 │
//! │  └──────────────┬───────────────────────┘                 │
//! │                 │                                          │
//! │                 ▼                                          │
//! │  ┌──────────────────────────────────────┐                 │
//! │  │ Codegen: register "my_add" as native │                 │
//! │  │ → any call to my_add(1, 2) emits     │                 │
//! │  │   CallNative { "my_add" }            │                 │
//! │  └──────────────┬───────────────────────┘                 │
//! │                 │                                          │
//! │                 ▼  Runtime                                 │
//! │  ┌──────────────────────────────────────┐                 │
//! │  │ FfiRegistry.call("my_add", args)     │                 │
//! │  │ → execute registered Rust handler    │                 │
//! │  └──────────────────────────────────────┘                 │
//! └───────────────────────────────────────────────────────────┘
//! ```
//!
//! # Usage from YaoXiang
//!
//! ```yaoxiang
//! # Declare a native function binding using the std.ffi.native function
//! my_add: (a: Int, b: Int) -> Int = native("my_add")
//!
//! # Call it (dispatches to Rust handler via FFI)
//! result = my_add(1, 2)
//! println(result)   # → 3
//! ```
//!
//! `native` is a real function declared in `std.ffi` with signature
//! `native(symbol: String) -> Never`. It is intercepted at compile time
//! by the IR generator — when the name `std.ffi.native` is resolved in
//! a function declaration's value position, the compiler records a
//! `NativeBinding` instead of emitting bytecode. At runtime, attempting
//! to call `native(...)` will fail with a clear error.
//!
//! # Usage from Rust (embedding API)
//!
//! ```rust,ignore
//! use yaoxiang::backends::interpreter::ffi::FfiRegistry;
//! use yaoxiang::backends::common::RuntimeValue;
//!
//! // Create an interpreter and register custom native functions
//! let mut interpreter = Interpreter::new();
//! interpreter.ffi_registry_mut().register("my_add", |args, ctx| {
//!     let a = args[0].to_int().unwrap_or(0);
//!     let b = args[1].to_int().unwrap_or(0);
//!     Ok(RuntimeValue::Int(a + b))
//! });
//! ```
// ============================================================================
// (All code removed — NativeBinding, FfiModule, and native_ffi_native have
//  been deleted as part of the migration from native() to Native.c()/Native.rs().
//  The module docstring above is preserved for historical context.)
// ============================================================================
