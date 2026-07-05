---
title: "RFC-026a: Extensible FFI Mechanism System"
status: "Under Review"
author: "Chenxu"
created: "2026-06-05"
updated: "2026-07-03"
group: "rfc-026"
---

# RFC-026a: Extensible FFI Mechanism System

> **Parent RFC**: [RFC-026: FFI Core Mechanism](../accepted/026-ffi-core-mechanism.md)
>
> This RFC defines the extensibility portion of RFC-026—how to plug in FFI mechanisms beyond the C ABI (Wasm, Python, custom ABIs) as plugins, as well as the dynamic loading mode.

## Abstract

RFC-026 defines the FFI core mechanism, with `Native.c("lib")` going through the built-in C ABI. This RFC abstracts the ABI mechanism into a pluggable `FfiMechanism`, so that the core hardcodes no specific ABI:

1. **`FfiMechanism` Abstraction**: Defines the four operations a mechanism must implement (load library, resolve symbol, marshal, invoke)
2. **Mechanism Tag Equals Mechanism Selection**: `Native.c` / `Native.wasm` / `Native.python` each select a registered mechanism
3. **Compile-Time Mechanism Registry**: Mechanism tags are validated at compile-time; unregistered tags produce compile errors
4. **Static vs. Dynamic Loading**: Both modes preserve RFC-026's safety boundaries

## Motivation

RFC-026 only has a built-in C ABI (`Native.c`). But YaoXiang may need in the future:
- Calling Wasm modules (`Native.wasm`)
- Embedding Python extensions (`Native.python`)
- User-defined ABIs (proprietary hardware, RPC bridging)

Rather than hardcoding these ABIs in the compiler, we abstract "how to load libraries, how to resolve symbols, how to marshal, how to invoke" into a trait, with each mechanism implemented as a plugin. The core only knows `FfiMechanism`, not any specific ABI.

### Design Constraints

1. **Compile-time mechanism tag validation**: The `xxx` in `Native.xxx(...)` must be a registered mechanism, otherwise compile error
2. **No hardcoded mechanisms**: The compiler has no built-in mechanism list (except `.c` as a reference implementation); mechanisms are registered by plugins
3. **Preserve RFC-026 safety boundaries**: Every mechanism must adhere to type duality, marshalling scratch-zone isolation, Move + RAII
4. **Bootstrap compatibility**: The mechanism registry degrades to YaoXiang's `Dict`/`Set`

---

## Proposal

### 1. `FfiMechanism` Abstraction

Each FFI mechanism implements four operations. This is the key to the core not hardcoding any ABI—the compiler only calls this interface and doesn't know whether the backing implementation is C, Wasm, or something else:

```rust
trait FfiMechanism {
    /// Mechanism tag, e.g. "c" / "wasm" / "python"
    fn tag(&self) -> &str;

    /// Load a library. C: dlopen/static link; Wasm: instantiate module; Python: import.
    /// Returns a mechanism-internal library handle.
    fn load_library(&self, id: &str) -> Result<LibraryHandle>;

    /// Resolve a symbol. Can be called at compile-time to verify symbol existence.
    /// C: dlsym/symbol table lookup; Wasm: export table lookup.
    fn resolve(&self, lib: &LibraryHandle, symbol: &str) -> Result<SymbolHandle>;

    /// Invoke. Marshal arguments according to the YaoXiang signature, execute, marshal the return value.
    /// Must obey the marshalling rules from RFC-026 §3 (scratch-zone isolation).
    fn invoke(
        &self,
        sym: &SymbolHandle,
        args: &[RuntimeValue],
        sig: &Signature,
    ) -> Result<RuntimeValue>;
}
```

**Key**: The `invoke` implementation must obey RFC-026 §3—copy inputs to a scratch zone, memcpy the return value, borrow qualification for a single call. The mechanism may choose its own ABI details, but **cannot violate the safety boundary**. This is the plugin's obligation.

### 2. Mechanism Tag Equals Mechanism Selection

```yaoxiang
// .c → C ABI mechanism (RFC-026 built-in reference implementation)
sqlite3 = Native.c("libsqlite3")
SqliteDb.open: (f: String) -> ?SqliteDb = sqlite3("sqlite3_open")

// .wasm → Wasm mechanism (registered by the yx_wasm_ffi plugin)
wasm_mod = Native.wasm("mymodule.wasm")
process: (input: String) -> String = wasm_mod("process")

// .python → Python mechanism (registered by the yx_python_ffi plugin)
np = Native.python("numpy")
```

The `.c` / `.wasm` in `Native.c` / `Native.wasm` are **mechanism tags** that select which registered `FfiMechanism` to use. The core has `.c` built-in as a reference implementation; the rest are provided by plugins.

### 3. Mechanism Registration and Compile-Time Validation

A plugin declares the mechanism tags it provides to the mechanism registry via a `.so` at compile-time:

```text
use yx_wasm_ffi
  → load libyx_wasm_ffi.so
  → call yx_register_mechanism()
  → register FfiMechanism { tag: "wasm", ... }
  → mechanism registry adds "wasm"

// Afterwards:
Native.wasm("mod.wasm")    // ✅ compiles, "wasm" is registered
Native.foo("x")            // ❌ compile error: Unknown FFI mechanism 'foo'
                           //    Try: `use yx_foo_ffi`
```

The compile-time mechanism registry **only stores mechanism tags** (strings) + the corresponding `FfiMechanism` instance pointer. When compiling `Native.xxx(...)`, the table is consulted; if the tag doesn't exist, it's a compile error.

### 4. Static vs. Dynamic Loading

The implementation of `load_library` determines when loading occurs. Both modes preserve RFC-026's safety boundary:

| Mode | `load_library` Behavior | Symbol Validation | Typing |
|------|------------------------|-------------------|--------|
| **Static** (default, C ABI) | Compile-time `-llib`, library enters symbol table | Read symbol table at compile-time | Fully concrete |
| **Dynamic** | dlopen/instantiate on first call at runtime | Validated at first load; missing → fail-fast | Declaratively trusted, validated on load |

```yaoxiang
// Static: C library linked in at compile-time
sqlite3 = Native.c("libsqlite3")           // compile-time -lsqlite3

// Dynamic: plugin discovered at runtime
plugin = Native.c.dynamic("./plugins/foo.so")   // runtime dlopen
```

Whether static or dynamic, marshalling goes through RFC-026 §3's scratch-zone isolation. In dynamic mode, a missing symbol is a **clean runtime error** (fail-fast), not a crash.

### 5. Complete Information Flow

```
use yx_wasm_ffi                     ← register "wasm" mechanism
       │
       ▼
wasm_mod = Native.wasm("mod.wasm")
  Compile-time: look up mechanism registry, "wasm" exists ✅
         → call wasm mechanism's load_library("mod.wasm")
         → instantiate Wasm module, return library handle
       │
       ▼
process: (input: String) -> String = wasm_mod("process")
  Compile-time: call wasm mechanism's resolve(lib, "process") to verify export exists ✅
         → generate CallNative { mechanism: "wasm", lib, symbol: "process", sig }
       │
       ▼  Runtime
  CallNative executes
  → mechanism's invoke(sym, args, sig)
  → marshal per sig (scratch-zone isolation) → execute Wasm → marshal return
```

### 6. Bootstrap Degradation

The `FfiMechanism` trait + mechanism registry, which exist in the Rust-managed period, degrade to ordinary YaoXiang structures after bootstrap:

```yaoxiang
// After bootstrap, the mechanism registry is a Dict
let mechanisms: Dict(String, FfiMechanism) = {}
mechanisms["c"] = c_mechanism
mechanisms["wasm"] = wasm_mechanism

// FfiMechanism is an interface in YaoXiang (RFC-011a dynamic dispatch)
// Native.c("lib") → mechanisms["c"].load_library("lib")
```

The Rust period uses a trait object (`Box<dyn FfiMechanism>`); after bootstrap it uses a YaoXiang interface (RFC-011a). The interface is consistent: load, resolve, marshal, invoke.

---

## Trade-offs

### Advantages

1. **Zero hardcoded ABIs**: The core only knows `FfiMechanism`; a new ABI = a new plugin
2. **Unified safety boundary**: All mechanisms are forced to obey RFC-026 §3 marshalling rules
3. **Compile-time mechanism validation**: Unregistered mechanisms are caught at compile-time, not discovered at runtime
4. **Unified static/dynamic abstraction**: The implementation details of `load_library` are hidden inside the mechanism

### Disadvantages

1. **Plugin authoring threshold**: Implementing `FfiMechanism` requires understanding the target ABI + the marshalling contract
2. **Mechanism obligation is by convention**: Scratch-zone isolation in marshalling depends on the plugin's compliance; the core cannot verify plugin implementations

---

## Implementation Strategy

### Phase 1a: Mechanism Abstraction (v0.8)

- [ ] Define the `FfiMechanism` trait (load_library / resolve / invoke)
- [ ] Refactor the C ABI implementation from RFC-026 as `CMechanism: FfiMechanism`
- [ ] Implement the compile-time mechanism registry (tag → mechanism instance)
- [ ] `Native.xxx` consults the mechanism registry at compile-time for validation

### Phase 1b: Dynamic Loading + Plugins (v0.9)

- [ ] Implement `.so` plugin loading (`yx_register_mechanism`)
- [ ] Implement dynamic library loading mode (`Native.c.dynamic`)
- [ ] Reference plugin: `yx_wasm_ffi` (Wasm mechanism)

---

## Relationship to Other RFCs

- **RFC-026** (parent): FFI core mechanism—`FfiMechanism` must obey its marshalling rules and safety boundary
- **RFC-011a**: Interfaces and dynamic dispatch—after bootstrap, `FfiMechanism` degrades to a YaoXiang interface
- **RFC-014**: Package management system—discovery and loading of `.so` plugins depends on the package manager
- **RFC-021** (deprecated): Library-driven FFI extension—this RFC sinks its `ffi.load_library` API down into the mechanism plugin layer

---

## Design Decision Records

| Decision | Resolution | Reason | Date |
|----------|------------|--------|------|
| Mechanism abstraction | `FfiMechanism` trait, four operations | Core hardcodes no ABI, only knows the interface | 2026-07-03 |
| Mechanism obligation | Plugins must obey RFC-026 marshalling rules | Safety boundary is not broken by different mechanisms | 2026-07-03 |
| Mechanism tag validation | Compile-time registry lookup | Unregistered mechanisms → compile-time error | 2026-07-03 |
| Static / dynamic | Determined by `load_library` implementation | Timing is a mechanism detail; safety boundary unchanged | 2026-07-03 |
| Bootstrap degradation | trait → YaoXiang interface (RFC-011a) | No excessive abstraction in the host language | 2026-07-03 |

---

## Lifecycle and Destination

| Status | Location | Description |
|--------|----------|-------------|
| **Under Review** | `docs/design/rfc/review/` | Open community discussion |
| **Accepted** | `docs/design/rfc/accepted/` | Official design document |