---
title: 'RFC-026a: Extensible FFI Mechanism System'
status: 'Under Review'
issue: '#135'
author: 'Chenxu'
created: '2026-06-05'
updated: '2026-10-14'
group: 'rfc-026'
---

# RFC-026a: Extensible FFI Mechanism System

> **Parent RFC**: [RFC-026: FFI Core Mechanism](../accepted/026-ffi-core-mechanism.md)
>
> This RFC defines the extensibility portion of RFC-026 — how FFI mechanisms beyond the C ABI (Wasm,
> Python, custom ABI) are plugged in as plugins, and the dynamic loading mode.

## Summary

RFC-026 defines the FFI core mechanism, where `Native.c("lib")` goes through the built-in C ABI.
This RFC abstracts ABI mechanisms as a pluggable `FfiMechanism`, so that the core does not hardcode
any specific ABI:

1. **`FfiMechanism` abstraction**: defines the four operations a mechanism must implement (load
   library, resolve symbol, marshal, invoke)
2. **Mechanism tag is mechanism selection**: `Native.c` / `Native.wasm` / `Native.python` select the
   registered mechanism respectively
3. **Compile-time mechanism registry**: mechanism tags are validated at compile time; unregistered
   tags produce a compile error
4. **Static vs dynamic loading**: both modes preserve the safety boundaries of RFC-026

## Motivation

RFC-026 only has the C ABI built in (`Native.c`). But YaoXiang may need in the future:

- Calling Wasm modules (`Native.wasm`)
- Embedding Python extensions (`Native.python`)
- User-defined ABIs (proprietary hardware, RPC bridging)

Rather than hardcoding these ABIs in the compiler, abstract "how to load a library, how to resolve
symbols, how to marshal, how to invoke" into a trait, with each mechanism implemented as a plugin.
The core only knows `FfiMechanism`, and does not know any specific ABI.

### Design Constraints

1. **Compile-time mechanism tag validation**: `xxx` in `Native.xxx(...)` must be a registered
   mechanism, otherwise compile error
2. **Do not hardcode mechanisms**: the compiler does not have a built-in mechanism list (except `.c`
   as the reference implementation); mechanisms are registered by plugins. **Status of `.rs`
   (2026-10-14 clarification)**: the current code already contains two mechanism tags — `.c` (C ABI,
   RFC-026) and `.rs` (YaoXiang std internal Rust function dispatch, documented in
   `interpreter/ffi.rs` module docs and registered in `typecheck/mod.rs` `Native.rs`). `.rs` is an
   std implementation detail and **not an external mechanism** — it does not go through plugin
   registration, and is not open to third-party implementations. The mechanism registry in this RFC
   only manages externally pluggable mechanisms (starting with `.c`), and `.rs` remains a hardcoded
   side path.
3. **Preserve RFC-026 safety boundaries**: any mechanism must adhere to type two-category, marshal
   scratchpad isolation, and Move + RAII
4. **Bootstrap compatibility**: the mechanism registry degrades to YaoXiang's `Dict`/`Set`

---

## Proposal

### 1. `FfiMechanism` Abstraction

Each FFI mechanism implements four operations. This is the key to the core not hardcoding an ABI —
the compiler only calls this interface, and does not know whether it is C, Wasm, or anything else
behind it:

```rust
trait FfiMechanism {
    /// Mechanism tag, e.g. "c" / "wasm" / "python"
    fn tag(&self) -> &str;

    /// Load a library. C: dlopen/static link; Wasm: instantiate module; Python: import.
    /// Returns a library handle internal to the mechanism.
    fn load_library(&self, id: &str) -> Result<LibraryHandle>;

    /// Resolve a symbol. May be called at compile time to verify the symbol exists.
    /// C: dlsym/symbol table lookup; Wasm: export table lookup.
    fn resolve(&self, lib: &LibraryHandle, symbol: &str) -> Result<SymbolHandle>;

    /// Invoke. Marshal arguments according to YaoXiang signature, execute, marshal return value.
    /// Must obey the marshal rules from RFC-026 §3 (scratchpad isolation).
    fn invoke(
        &self,
        sym: &SymbolHandle,
        args: &[RuntimeValue],
        sig: &Signature,
    ) -> Result<RuntimeValue>;
}
```

**Key**: the `invoke` implementation must obey RFC-026 §3 — copy input into the scratchpad, return
via memcpy, and restrict borrows to a single invocation. A mechanism may choose its own ABI details,
but **cannot violate the safety boundary**. This is the plugin's obligation.

### 2. Mechanism Tag Is Mechanism Selection

```yaoxiang
// .c → C ABI mechanism (RFC-026 built-in reference implementation)
sqlite3 = Native.c("libsqlite3")
SqliteDb.open: (f: String) -> ?SqliteDb = sqlite3("sqlite3_open")

// .wasm → Wasm mechanism (yx_wasm_ffi plugin registered)
wasm_mod = Native.wasm("mymodule.wasm")
process: (input: String) -> String = wasm_mod("process")

// .python → Python mechanism (yx_python_ffi plugin registered)
np = Native.python("numpy")
```

The `.c` / `.wasm` in `Native.c` / `Native.wasm` are **mechanism tags**, selecting which registered
`FfiMechanism` to use. The core has `.c` built in as the reference implementation; others are
provided by plugins.

### 3. Mechanism Registration and Compile-Time Validation

A plugin declares the mechanism tags it provides to the mechanism registry at compile time through
an `.so`:

```text
use yx_wasm_ffi
  → load libyx_wasm_ffi.so
  → call yx_register_mechanism()
  → register FfiMechanism { tag: "wasm", ... }
  → mechanism registry adds "wasm"

// Afterwards:
Native.wasm("mod.wasm")    // ✅ Compiles, "wasm" is registered
Native.foo("x")            // ❌ Compile error: Unknown FFI mechanism 'foo'
                           //    Try: `use yx_foo_ffi`
```

The compile-time mechanism registry **only stores mechanism tags** (strings) + the corresponding
`FfiMechanism` instance pointer. When compiling `Native.xxx(...)`, the table is looked up; if the
tag does not exist, a compile error is raised.

#### 3.1 Plugin Trust Model (Added 2026-10-14)

Loading `.so` plugins at compile time means **the compile process executes arbitrary native code** —
the plugin runs in the same process as the compiler with the same privileges. This is not a problem
unique to this RFC (proc-macro and compiler plugin ecosystems have the same nature), but the
position must be made explicit before acceptance and cannot be assumed:

| Layer             | Measure                                                                                                                                                                | Status                                                               |
| ----------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------- |
| Source constraint | Mechanism plugins can only come from the package manager (RFC-014) trusted-source install path; **do not search `PATH`, do not load `.so` from the current directory** | Frozen by this RFC                                                   |
| Transparency      | The actually loaded plugin path is output in compile diagnostics (visible by default with `--verbose`); builds are reproducible                                        | Frozen by this RFC                                                   |
| Signing           | The registry (RFC-014a) signs plugin packages; the compiler verifies the signature before loading                                                                      | Deferred to joint design with RFC-014a                               |
| ABI versioning    | The `yx_register_mechanism` entry carries a version negotiation argument (the `ABI_VERSION` constant); mismatch refuses to load and reports a compile error            | Frozen by this RFC; specific value determined at implementation time |

**Threat characterization**: a mechanism plugin = arbitrary code execution at compile time. Same
level of trust as Rust proc-macros; controlled by source constraints + versioning on the control
plane, no sandbox promised. This is consistent with the philosophy of RFC-026 §6 (unsafe escape
hatch): explicit trust, no hidden isolation.

### 4. Static vs Dynamic Loading

The implementation of `load_library` determines the loading timing. Both modes preserve RFC-026's
safety boundaries:

| Mode                        | `load_library` Behavior                                     | Symbol Verification                       | Type                                  |
| --------------------------- | ----------------------------------------------------------- | ----------------------------------------- | ------------------------------------- |
| **Static** (default, C ABI) | `-llib` at compile time, library goes into the symbol table | Reads the symbol table at compile time    | Fully concrete                        |
| **Dynamic**                 | dlopen/instantiate on first call at runtime                 | Verify on first load; missing = fail-fast | Declared as trusted; verified on load |

```yaoxiang
// Static: C library linked in at compile time
sqlite3 = Native.c("libsqlite3")           // compile-time -lsqlite3

// Dynamic: plugins discovered at runtime (syntax carrier TBD, see open question Q2)
plugin = <dynamic load entry>("./plugins/foo.so")   // runtime dlopen
```

> **⚠️ Syntax TBD (2026-10-14)**: the original document here wrote `Native.c.dynamic(...)`. The
> current frontend registers `Native.c` as a dotted fully-qualified variable, and `FieldAccess`
> resolves the path assembled from `Var("Native")` via `extract_namespace_path` (the comment at
> `typecheck/mod.rs:218-247` explicitly states "Native itself must be a Struct with fields").
> `.dynamic` is a third-level path, which this mechanism does not currently carry. Candidates and
> trade-offs are in "Open Question Q2"; before the decision is finalized, `Native.c.dynamic` is not
> a committed syntax.

Whether static or dynamic, marshaling goes through the scratchpad isolation of RFC-026 §3. In
dynamic mode, a missing symbol is a **clean runtime error** (fail-fast), not a crash.

### 5. Complete Information Flow

```
use yx_wasm_ffi                     ← register the "wasm" mechanism
       │
       ▼
wasm_mod = Native.wasm("mod.wasm")
  Compile time: look up "wasm" in the mechanism registry ✅
         → call wasm mechanism's load_library("mod.wasm")
         → instantiate the Wasm module, return the library handle
       │
       ▼
process: (input: String) -> String = wasm_mod("process")
  Compile time: call wasm mechanism's resolve(lib, "process") to verify the export exists ✅
         → generate CallNative { mechanism: "wasm", lib, symbol: "process", sig }
       │
       ▼  Runtime
  CallNative executes
  → mechanism's invoke(sym, args, sig)
  → marshal according to sig (scratchpad isolation) → execute Wasm → marshal return
```

### 6. Degradation After Bootstrap

The `FfiMechanism` trait + mechanism registry during the Rust-hosted period degrade to ordinary
YaoXiang structures after bootstrap:

```yaoxiang
// After bootstrap, the mechanism registry is a Dict
let mechanisms: Dict(String, FfiMechanism) = {}
mechanisms["c"] = c_mechanism
mechanisms["wasm"] = wasm_mechanism

// FfiMechanism is an interface in YaoXiang (RFC-011a dynamic dispatch)
// Native.c("lib") → mechanisms["c"].load_library("lib")
```

The Rust period uses trait objects (`Box<dyn FfiMechanism>`); after bootstrap, YaoXiang interfaces
are used (RFC-011a). The interface is consistent: load, resolve, marshal, invoke.

---

## Trade-offs

### Advantages

1. **Zero hardcoded ABI**: the core only knows `FfiMechanism`; a new ABI = a new plugin
2. **Unified safety boundary**: all mechanisms are forced to obey the marshal rules of RFC-026 §3
3. **Compile-time mechanism validation**: a non-existent mechanism tag reports a compile error, not
   a runtime surprise
4. **Unified static/dynamic abstraction**: the implementation details of `load_library` are hidden
   inside the mechanism

### Disadvantages

1. **Plugin authoring threshold**: implementing `FfiMechanism` requires understanding the target
   ABI + the marshal contract
2. **Mechanism obligations rest on convention, but the convention must have teeth (Added
   2026-10-14)**: scratchpad isolation in marshaling depends on the plugin complying, and the core
   cannot fully statically verify any plugin implementation at compile time. Layered enforcement
   clauses:
   - **Interface obligation (mandatory)**: the `FfiMechanism` trait method signatures are the
     contract — `invoke` must marshal according to `sig`; violating the signature is an
     implementation defect, with runtime panic as a fallback;
   - **Official plugin obligation (mandatory)**: official plugins such as `yx_wasm_ffi` must run
     **mechanism contract tests** (a standard set of marshal cases: cross-boundary
     String/List/struct, out-of-bounds write, dangling pointers) together with the compiler's test
     suite; a red contract test = release blocking;
   - **Third-party plugin obligation (convention + documentation)**: the registry records the
     plugin's self-declared obligation compliance; the documentation explicitly states "the marshal
     safety of non-official mechanisms is the responsibility of the plugin author." The core
     provides a contract test framework for third-party self-testing.

---

## Open Questions (Added 2026-10-14 Review)

**Q1: Where plugin signature verification belongs**. This RFC freezes source constraints and ABI
versioning; signature verification is deferred to joint design with RFC-014a (the registry). If
RFC-014a lands later than stage 1b, in the interim window mechanism plugins are only constrained by
source path — is that acceptable? (Lean: acceptable, consistent with the early state of the
proc-macro ecosystem; after RFC-014a is finalized, verification is added.)

**Q2: Syntax carrier for the dynamic load entry**. `Native.c.dynamic(...)` is unreachable under the
current FieldAccess resolution design (see warning box in §4). Candidates:

| Option                                                       | Pros                                                  | Cons                                                                                             |
| ------------------------------------------------------------ | ----------------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| a. `Native.c_dynamic("path")` (flat tag)                     | Zero parsing changes; tag table carries it directly   | The tag namespace is polluted by `_`-composite tags                                              |
| b. `Native.dynamic("c", "path")` (parameterized)             | Orthogonal syntax; mechanism tags remain single-level | `dynamic` becomes a reserved special name, itself an exception                                   |
| c. Library-layer function `std.ffi.dynamic_load(mech, path)` | Zero compiler changes; pure std implementation        | Bypasses compile-time mechanism validation (the tag is a runtime string), violating constraint 1 |

(Lean: b — `dynamic` is an orthogonal dimension of loading mode rather than a new mechanism, so
parameterized expression is the most honest; the cost is that FieldAccess must support single-level
special-method dispatch, which is a manageable amount of implementation. Before the decision is
finalized, stage 1b does not include dynamic loading.)

**Q3: Whether `.rs` keeps the hardcoded side path long term**. This RFC characterizes it as an std
implementation detail (see design constraint 2). If in the future all std is rewritten through
YaoXiang bootstrap, does `.rs` exit the stage or convert to an ordinary mechanism — to be decided by
the bootstrap roadmap (RFC-039).

**Q4: Wasm mechanism's GC boundary**. How do Wasm linear-memory object lifecycles interoperate with
YaoXiang heap's Move + RAII (RFC-026 §4) — when a `LibraryHandle` drops, is the Wasm instance
forcibly released? The reference plugin `yx_wasm_ffi` must answer this at design time; this RFC does
not pre-answer it.

---

## Implementation Strategy

### Stage 1a: Mechanism Abstraction (v0.8)

- [ ] Define the `FfiMechanism` trait (load_library / resolve / invoke)
- [ ] Refactor the C ABI implementation of RFC-026 as `CMechanism: FfiMechanism`
- [ ] Implement the compile-time mechanism registry (tag → mechanism instance)
- [ ] `Native.xxx` looks up the mechanism registry at compile time for validation

### Stage 1b: Dynamic Loading + Plugins (v0.9)

- [ ] Implement `.so` plugin loading (`yx_register_mechanism`, with ABI version negotiation)
- [ ] Implement the dynamic library loading mode (syntax pending Q2)
- [ ] Reference plugin: `yx_wasm_ffi` (Wasm mechanism, must pass mechanism contract tests)

---

## Relationship to Other RFCs

- **RFC-026** (parent): FFI core mechanism — `FfiMechanism` must obey its marshal rules and safety
  boundaries
- **RFC-011a**: interfaces and dynamic dispatch — after bootstrap, `FfiMechanism` degrades to a
  YaoXiang interface
- **RFC-014**: package management system — discovery and loading of `.so` plugins depend on the
  package manager
- **RFC-021** (deprecated): library-driven FFI extension — this RFC pushes its `ffi.load_library`
  API down to the mechanism plugin layer

---

## Design Decision Record

| Decision                 | Determination                                                   | Reason                                                              | Date       |
| ------------------------ | --------------------------------------------------------------- | ------------------------------------------------------------------- | ---------- |
| Mechanism abstraction    | `FfiMechanism` trait, four operations                           | The core hardcodes no ABI, only the interface                       | 2026-07-03 |
| Mechanism obligation     | Plugins must obey the marshal rules of RFC-026                  | The safety boundary does not break because of a different mechanism | 2026-07-03 |
| Mechanism tag validation | Look up the registry at compile time                            | Unregistered mechanisms report at compile time                      | 2026-07-03 |
| Static/Dynamic           | Determined by the `load_library` implementation                 | Timing is a mechanism detail; safety boundary is unchanged          | 2026-07-03 |
| Bootstrap degradation    | trait → YaoXiang interface (RFC-011a)                           | No over-abstraction in the host language                            | 2026-07-03 |
| Plugin trust model       | Source constraints + ABI versioning, no sandbox promised        | Same level of trust as proc-macro; made explicit                    | 2026-10-14 |
| Status of `.rs`          | std implementation detail, not going through the registry       | Aligns with the statement "everything except `.c` is a plugin"      | 2026-10-14 |
| `.dynamic` syntax        | Rolled back to open question Q2; original example not committed | The current FieldAccess does not carry third-level paths            | 2026-10-14 |

---

## Lifecycle and Destination

| Status           | Location                    | Description               |
| ---------------- | --------------------------- | ------------------------- |
| **Under Review** | `docs/design/rfc/review/`   | Open community discussion |
| **Accepted**     | `docs/design/rfc/accepted/` | Formal design document    |
