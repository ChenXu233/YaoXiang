---
title: "RFC-026a: Extensible FFI Mechanism System"
status: "Under Review"
issue: "#135"
author: "晨煦 (Chenxu)"
created: "2026-06-05"
updated: "2026-07-05"
group: "rfc-026"
---

# RFC-026a: Extensible FFI Mechanism System

> **Parent RFC**: [RFC-026: FFI Core Mechanism](../accepted/026-ffi-core-mechanism.md)
>
> This RFC defines the extensibility portion of RFC-026—how FFI mechanisms other than the C ABI (Wasm, Python, custom ABIs) plug in as plugins, and the dynamic loading mode.

## Summary

RFC-026 defines the FFI core mechanism, where `Native.c("lib")` goes through the builtin C ABI. This RFC abstracts the ABI mechanism as a pluggable `FfiMechanism`, so the core hardcodes no specific ABI:

1. **`FfiMechanism` Abstraction**: Defines the four operations every mechanism must implement (load library, resolve symbol, marshal, invoke)
2. **Mechanism Tag as Mechanism Selector**: `Native.c` / `Native.wasm` / `Native.python` each select a registered mechanism
3. **Compile-Time Mechanism Registry**: Mechanism tags are validated at compile-time; unregistered tags produce compile errors
4. **Static vs. Dynamic Loading**: Both modes preserve RFC-026's safety boundaries

## Motivation

RFC-026 only built-ins the C ABI (`Native.c`). But YaoXiang may in the future need to:
- Call Wasm modules (`Native.wasm`)
- Embed Python extensions (`Native.python`)
- Support user-defined ABIs (proprietary hardware, RPC bridging)

Rather than hardcoding these ABIs in the compiler, abstract "how to load a library, how to resolve a symbol, how to marshal, how to invoke" into a trait, with each mechanism implemented as a plugin. The core only knows about `FfiMechanism`, not any specific ABI.

### Design Constraints

1. **Compile-time mechanism tag validation**: The `xxx` in `Native.xxx(...)` must be a registered mechanism, otherwise it's a compile error
2. **No hardcoded mechanisms**: The compiler does not maintain a builtin mechanism list (except `.c` as a reference implementation); mechanisms are registered by plugins
3. **Preserve RFC-026 safety boundaries**: Every mechanism must obey the two-type classification, marshalling scratchpad isolation, and Move + RAII
4. **Self-hosting compatible**: The mechanism registry degrades to YaoXiang's `Dict`/`Set`

---

## Proposal

### 1. `FfiMechanism` Abstraction

Each FFI mechanism implements four operations. This is the key to the core not hardcoding any ABI—the compiler only calls this interface and does not know whether the backend is C, Wasm, or something else:

```rust
trait FfiMechanism {
    /// Mechanism tag, e.g. "c" / "wasm" / "python"
    fn tag(&self) -> &str;

    /// Load a library. C: dlopen/static link; Wasm: instantiate module; Python: import.
    /// Returns a mechanism-internal library handle.
    fn load_library(&self, id: &str) -> Result<LibraryHandle>;

    /// Resolve a symbol. Callable at compile-time to verify the symbol exists.
    /// C: dlsym/symbol table lookup; Wasm: export table lookup.
    fn resolve(&self, lib: &LibraryHandle, symbol: &str) -> Result<SymbolHandle>;

    /// Invoke. Marshal arguments per the YaoXiang signature, execute, marshal the return value.
    /// Must obey the marshalling rules from RFC-026 §3 (scratchpad isolation).
    fn invoke(
        &self,
        sym: &SymbolHandle,
        args: &[RuntimeValue],
        sig: &Signature,
    ) -> Result<RuntimeValue>;
}
```

**Key**: The `invoke` implementation must obey RFC-026 §3—copy arguments into a scratchpad, return via memcpy, single-call borrow qualification. A mechanism may choose its own ABI details, but **may not violate the safety boundaries**. This is the plugin's obligation.

### 2. Mechanism Tag as Mechanism Selector

```yaoxiang
// .c → C ABI mechanism (RFC-026 builtin reference implementation)
sqlite3 = Native.c("libsqlite3")
SqliteDb.open: (f: String) -> ?SqliteDb = sqlite3("sqlite3_open")

// .wasm → Wasm mechanism (registered by the yx_wasm_ffi plugin)
wasm_mod = Native.wasm("mymodule.wasm")
process: (input: String) -> String = wasm_mod("process")

// .python → Python mechanism (registered by the yx_python_ffi plugin)
np = Native.python("numpy")
```

The `.c` / `.wasm` in `Native.c` / `Native.wasm` are **mechanism tags** that select which registered `FfiMechanism` to use. The core builds in `.c` as a reference implementation; the rest are provided by plugins.

### 3. Mechanism Registration and Compile-Time Validation

A plugin declares the mechanism tags it provides to the mechanism registry at compile-time via a `.so`:

```text
use yx_wasm_ffi
  → Load libyx_wasm_ffi.so
  → Call yx_register_mechanism()
  → Register FfiMechanism { tag: "wasm", ... }
  → Mechanism registry gains "wasm"

// Afterwards:
Native.wasm("mod.wasm")    // ✅ Compiles, "wasm" is registered
Native.foo("x")            // ❌ Compile error: Unknown FFI mechanism 'foo'
                           //    Try: `use yx_foo_ffi`
```

The compile-time mechanism registry **only stores mechanism tags** (strings) + the corresponding `FfiMechanism` instance pointer. When compiling `Native.xxx(...)`, the table is consulted; a missing tag yields a compile error.

### 4. Static vs. Dynamic Loading

The implementation of `load_library` determines the load timing. Both modes preserve RFC-026's safety boundaries:

| Mode | `load_library` Behavior | Symbol Verification | Type |
|------|-------------------|---------|------|
| **Static** (default, C ABI) | Compile-time `-llib`, library enters the symbol table | Reads the symbol table at compile-time | Fully concrete |
| **Dynamic** | dlopen/instantiate on first call at runtime | Verified on first load; missing symbols fail-fast | Declaratively trusted, verified on load |

```yaoxiang
// Static: C library linked at compile-time
sqlite3 = Native.c("libsqlite3")           // compile-time -lsqlite3

// Dynamic: plugin discovered at runtime
plugin = Native.c.dynamic("./plugins/foo.so")   // runtime dlopen
```

Whether static or dynamic, marshalling goes through the scratchpad isolation from RFC-026 §3. In dynamic mode, missing symbols are a **clean runtime error** (fail-fast), not a crash.

### 5. Complete Information Flow

```
use yx_wasm_ffi                     ← Register "wasm" mechanism
       │
       ▼
wasm_mod = Native.wasm("mod.wasm")
  Compile-time: Check mechanism registry; "wasm" exists ✅
         → Call wasm mechanism's load_library("mod.wasm")
         → Instantiate the Wasm module, return library handle
       │
       ▼
process: (input: String) -> String = wasm_mod("process")
  Compile-time: Call wasm mechanism's resolve(lib, "process") to verify the export exists ✅
         → Generate CallNative { mechanism: "wasm", lib, symbol: "process", sig }
       │
       ▼  Runtime
  CallNative executes
  → Mechanism's invoke(sym, args, sig)
  → Marshal per sig (scratchpad isolation) → Execute Wasm → Marshal return
```

### 6. Degradation After Self-Hosting

The Rust-hosted `FfiMechanism` trait + mechanism registry degrade after self-hosting to ordinary YaoXiang structures:

```yaoxiang
// After self-hosting, the mechanism registry is a Dict
let mechanisms: Dict(String, FfiMechanism) = {}
mechanisms["c"] = c_mechanism
mechanisms["wasm"] = wasm_mechanism

// FfiMechanism is an interface in YaoXiang (RFC-011a dynamic dispatch)
// Native.c("lib") → mechanisms["c"].load_library("lib")
```

The Rust era uses a trait object (`Box<dyn FfiMechanism>`); after self-hosting, it uses a YaoXiang interface (RFC-011a). The interface is consistent: load, resolve, marshal, invoke.

---

## Trade-offs

### Advantages

1. **Zero hardcoded ABIs**: The core only knows `FfiMechanism`; a new ABI = a new plugin
2. **Unified safety boundary**: All mechanisms are forced to obey the RFC-026 §3 marshalling rules
3. **Compile-time mechanism validation**: A missing mechanism tag produces a compile error, not a runtime surprise
4. **Unified static/dynamic abstraction**: The implementation details of `load_library` are hidden inside the mechanism

### Disadvantages

1. **Plugin authoring门槛**: Implementing `FfiMechanism` requires understanding the target ABI + the marshalling contract
2. **Mechanism obligation is by convention**: The core cannot force-verify that plugins obey scratchpad isolation; it relies on convention

---

## Implementation Strategy

### Phase 1a: Mechanism Abstraction (v0.8)

- [ ] Define the `FfiMechanism` trait (load_library / resolve / invoke)
- [ ] Refactor the C ABI implementation from RFC-026 into `CMechanism: FfiMechanism`
- [ ] Implement the compile-time mechanism registry (tag → mechanism instance)
- [ ] Have `Native.xxx` consult the mechanism registry at compile-time for validation

### Phase 1b: Dynamic Loading + Plugins (v0.9)

- [ ] Implement `.so` plugin loading (`yx_register_mechanism`)
- [ ] Implement the dynamic library loading mode (`Native.c.dynamic`)
- [ ] Reference plugin: `yx_wasm_ffi` (Wasm mechanism)

---

## Relationship to Other RFCs

- **RFC-026** (parent): FFI core mechanism—`FfiMechanism` must obey its marshalling rules and safety boundaries
- **RFC-011a**: Interfaces and Dynamic Dispatch—after self-hosting, `FfiMechanism` degrades to a YaoXiang interface
- **RFC-014**: Package Management System—discovery and loading of `.so` plugins depends on the package manager
- **RFC-021** (deprecated): Library-Driven FFI Extension—this RFC sinks its `ffi.load_library` API down to the mechanism plugin layer

---

## Design Decision Log

| Decision | Resolution | Reason | Date |
|------|------|------|------|
| Mechanism abstraction | `FfiMechanism` trait, four operations | Core hardcodes no ABI, only knows the interface | 2026-07-03 |
| Mechanism obligation | Plugins must obey RFC-026 marshalling rules | Safety boundary does not break because of mechanism differences | 2026-07-03 |
| Mechanism tag validation | Compile-time registry lookup | Unregistered mechanisms produce compile errors | 2026-07-03 |
| Static/dynamic | Decided by `load_library` implementation | Timing is a mechanism detail; safety boundary is unchanged | 2026-07-03 |
| Self-hosting degradation | trait → YaoXiang interface (RFC-011a) | Avoid over-abstracting against the host language | 2026-07-03 |

---

## Lifecycle and Destination

| Status | Location | Description |
|------|------|------|
| **Under Review** | `docs/design/rfc/review/` | Open to community discussion |
| **Accepted** | `docs/design/rfc/accepted/` | Formal design document |