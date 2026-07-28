---
title: 'RFC-026a: Extensible FFI Mechanism System'
status: 'Review'
issue: '#135'
author: 'Chen Xu'
created: '2026-06-05'
updated: '2026-07-05'
group: 'rfc-026'
---

# RFC-026a: Extensible FFI Mechanism System

> **Parent RFC**: [RFC-026: FFI Core Mechanism](../accepted/026-ffi-core-mechanism.md)
>
> This RFC defines the extensibility portion of RFC-026—how to plug in FFI mechanisms beyond C ABI (Wasm, Python, custom ABI) as plugins, along with dynamic loading patterns.

## Summary

RFC-026 defines the FFI core mechanism, with `Native.c("lib")` going through the built-in C ABI. This RFC abstracts the ABI mechanism as a pluggable `FfiMechanism`, so the core does not hardcode any specific ABI:

1. **`FfiMechanism` abstraction**: Define four operations that mechanisms must implement (load library, resolve symbols, marshal, invoke)
2. **Mechanism tag as mechanism selection**: `Native.c` / `Native.wasm` / `Native.python` respectively select registered mechanisms
3. **Compile-time mechanism registry**: Mechanism tags are validated at compile-time; unregistered tags produce compile errors
4. **Static vs dynamic loading**: Both modes maintain RFC-026's security boundaries

## Motivation

RFC-026 only includes the built-in C ABI (`Native.c`). But YaoXiang may need in the future:

- Invoke Wasm modules (`Native.wasm`)
- Embed Python extensions (`Native.python`)
- User-defined ABIs (proprietary hardware, RPC bridging)

Instead of hardcoding these ABIs in the compiler, we abstract "how to load libraries, how to resolve symbols, how to marshal, how to invoke" into a trait, with each mechanism implemented as a plugin. The core only knows `FfiMechanism`, not any specific ABI.

### Design Constraints

1. **Compile-time mechanism tag validation**: `xxx` in `Native.xxx(...)` must be a registered mechanism, otherwise compile error
2. **No hardcoded mechanisms**: The compiler does not have a built-in mechanism list (except `.c` as a reference implementation); mechanisms are registered by plugins
3. **Maintain RFC-026 security boundaries**: Any mechanism must comply with type dichotomy, marshaling temporary zone isolation, Move + RAII
4. **Bootstrapping compatibility**: The mechanism registry degrades to YaoXiang's `Dict`/`Set`

---

## Proposal

### 1. `FfiMechanism` Abstraction

Each FFI mechanism implements four operations. This is the key to the core not hardcoding ABI—the compiler only calls this interface and doesn't know if the backend is C, Wasm, or something else:

```rust
trait FfiMechanism {
    /// Mechanism tag, e.g., "c" / "wasm" / "python"
    fn tag(&self) -> &str;

    /// Load library. C: dlopen/static link; Wasm: instantiate module; Python: import.
    /// Returns a mechanism-internal library handle.
    fn load_library(&self, id: &str) -> Result<LibraryHandle>;

    /// Resolve symbol. Can be called at compile-time to verify symbol existence.
    /// C: dlsym/symbol table lookup; Wasm: export table lookup.
    fn resolve(&self, lib: &LibraryHandle, symbol: &str) -> Result<SymbolHandle>;

    /// Invoke. Marshal arguments, execute, marshal return value according to YaoXiang signature.
    /// Must comply with RFC-026 §3 marshaling rules (temporary zone isolation).
    fn invoke(
        &self,
        sym: &SymbolHandle,
        args: &[RuntimeValue],
        sig: &Signature,
    ) -> Result<RuntimeValue>;
}
```

**Key**: The `invoke` implementation must comply with RFC-026 §3—input parameters copied to temporary zone, return value via memcpy, borrow限定 single invocation. Mechanisms can choose their own ABI details, but **cannot violate security boundaries**. This is the plugin's obligation.

### 2. Mechanism Tag as Mechanism Selection

```yaoxiang
// .c → C ABI mechanism (RFC-026 built-in reference implementation)
sqlite3 = Native.c("libsqlite3")
SqliteDb.open: (f: String) -> ?SqliteDb = sqlite3("sqlite3_open")

// .wasm → Wasm mechanism (registered by yx_wasm_ffi plugin)
wasm_mod = Native.wasm("mymodule.wasm")
process: (input: String) -> String = wasm_mod("process")

// .python → Python mechanism (registered by yx_python_ffi plugin)
np = Native.python("numpy")
```

The `.c` / `.wasm` in `Native.c` / `Native.wasm` are **mechanism tags**, selecting which registered `FfiMechanism` to use. The core has `.c` built-in as a reference implementation; others are provided by plugins.

### 3. Mechanism Registration and Compile-time Validation

Plugins declare the mechanism tags they provide at compile-time via `.so`:

```text
use yx_wasm_ffi
  → Load libyx_wasm_ffi.so
  → Call yx_register_mechanism()
  → Register FfiMechanism { tag: "wasm", ... }
  → Add "wasm" to mechanism registry

// Afterwards:
Native.wasm("mod.wasm")    // ✅ Compiles, "wasm" is registered
Native.foo("x")            // ❌ Compile error: Unknown FFI mechanism 'foo'
                           //    Try: `use yx_foo_ffi`
```

The compile-time mechanism registry **only stores mechanism tags** (strings) + pointers to corresponding `FfiMechanism` instances. When compiling `Native.xxx(...)`, the table is checked; if the tag doesn't exist, compile error.

### 4. Static vs Dynamic Loading

The `load_library` implementation determines loading timing; both modes maintain RFC-026's security boundaries:

| Mode             | `load_library` behavior              | Symbol validation                    | Type               |
| ---------------- | ------------------------------------ | ------------------------------------ | ------------------ |
| **Static** (default, C ABI) | Compile-time `-llib`, library in symbol table | Compile-time symbol table read       | Fully concrete     |
| **Dynamic**      | dlopen/instantiate at first invocation | Validate on first load, fail-fast on missing | Trust declaration, validate on load |

```yaoxiang
// Static: C library linked at compile time
sqlite3 = Native.c("libsqlite3")           // Compile-time -lsqlite3

// Dynamic: Plugins discovered at runtime
plugin = Native.c.dynamic("./plugins/foo.so")   // Runtime dlopen
```

Regardless of static or dynamic, marshaling follows RFC-026 §3 temporary zone isolation. In dynamic mode, missing symbols are **clean runtime errors** (fail-fast), not crashes.

### 5. Complete Information Flow

```
use yx_wasm_ffi                     ← Register "wasm" mechanism
       │
       ▼
wasm_mod = Native.wasm("mod.wasm")
  Compile-time: Check mechanism registry "wasm" exists ✅
         → Call wasm mechanism's load_library("mod.wasm")
         → Instantiate Wasm module, return library handle
       │
       ▼
process: (input: String) -> String = wasm_mod("process")
  Compile-time: Call wasm mechanism's resolve(lib, "process") to verify export exists ✅
         → Generate CallNative { mechanism: "wasm", lib, symbol: "process", sig }
       │
       ▼  Runtime
  Execute CallNative
  → Call mechanism's invoke(sym, args, sig)
  → Marshal according to sig (temporary zone isolation) → Execute Wasm → Marshal return
```

### 6. Degradation After Bootstrapping

The `FfiMechanism` trait + mechanism registry during Rust-hosted period degrades to ordinary YaoXiang structures after bootstrapping:

```yaoxiang
// After bootstrapping, mechanism registry is a Dict
let mechanisms: Dict(String, FfiMechanism) = {}
mechanisms["c"] = c_mechanism
mechanisms["wasm"] = wasm_mechanism

// FfiMechanism is an interface in YaoXiang (RFC-011a dynamic dispatch)
// Native.c("lib") → mechanisms["c"].load_library("lib")
```

During the Rust period, use trait objects (`Box<dyn FfiMechanism>`); after bootstrapping, use YaoXiang interfaces (RFC-011a). Interface is consistent: load, resolve, marshal, invoke.

---

## Tradeoffs

### Advantages

1. **Zero hardcoded ABIs**: Core only knows `FfiMechanism`; new ABI = new plugin
2. **Unified security boundaries**: All mechanisms are forced to comply with RFC-026 §3 marshaling rules
3. **Compile-time mechanism validation**: Unregistered mechanism tags produce compile errors, not runtime surprises
4. **Unified static/dynamic abstraction**: `load_library` implementation details are hidden within mechanisms

### Disadvantages

1. **Plugin authoring barrier**: Implementing `FfiMechanism` requires understanding target ABI + marshaling contract
2. **Mechanism obligations rely on convention**: Marshaling temporary zone isolation depends on plugin compliance; core cannot forcefully verify plugin implementation

---

## Implementation Strategy

### Phase 1a: Mechanism Abstraction (v0.8)

- [ ] Define `FfiMechanism` trait (load_library / resolve / invoke)
- [ ] Refactor RFC-026's C ABI implementation to `CMechanism: FfiMechanism`
- [ ] Implement compile-time mechanism registry (tag → mechanism instance)
- [ ] `Native.xxx` compile-time check against mechanism registry

### Phase 1b: Dynamic Loading + Plugins (v0.9)

- [ ] Implement `.so` plugin loading (`yx_register_mechanism`)
- [ ] Implement dynamic library loading mode (`Native.c.dynamic`)
- [ ] Reference plugin: `yx_wasm_ffi` (Wasm mechanism)

---

## Relationship with Other RFCs

- **RFC-026** (parent): FFI core mechanism—`FfiMechanism` must comply with its marshaling rules and security boundaries
- **RFC-011a**: Interfaces and dynamic dispatch—after bootstrapping, `FfiMechanism` degrades to YaoXiang interface
- **RFC-014**: Package management system—`.so` plugin discovery and loading depend on package manager
- **RFC-021** (deprecated): Library-driven FFI extensions—this RFC sinks its `ffi.load_library` API to the mechanism plugin layer

---

## Design Decision Record

| Decision             | Decision                                | Reason                                          | Date       |
| -------------------- | --------------------------------------- | ----------------------------------------------- | ---------- |
| Mechanism abstraction | `FfiMechanism` trait, four operations   | Core doesn't hardcode ABI, only knows interface | 2026-07-03 |
| Mechanism obligation  | Plugins must comply with RFC-026 marshaling rules | Security boundaries don't break with different mechanisms | 2026-07-03 |
| Mechanism tag validation | Compile-time registry check         | Unregistered mechanisms produce compile errors  | 2026-07-03 |
| Static/dynamic       | Determined by `load_library` implementation | Timing is a mechanism detail, security boundary unchanged | 2026-07-03 |
| Bootstrapping degradation | trait → YaoXiang interface (RFC-011a) | No excessive abstraction in host language       | 2026-07-03 |

---

## Lifecycle and Disposition

| Status        | Location                          | Description              |
| ------------- | --------------------------------- | ------------------------ |
| **Review**    | `docs/design/rfc/review/`         | Open for community discussion |
| **Accepted**  | `docs/design/rfc/accepted/`       | Formal design document   |
