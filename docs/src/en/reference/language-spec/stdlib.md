# Standard Library Specification

This document defines the standard library specification for the YaoXiang programming language.

> **This chapter is the language-level specification** (design conventions and semantic model). For
> module-by-module **signatures and runnable examples**, see
> [Standard Library Reference](../stdlib/index.md) — that part is derived from
> `StdModule::exports()` and guarded by a gate. Every signature in this document is taken verbatim
> from the implementation under `src/std/` (`*.rs` / `*.yx`).

---

## Chapter 0: Module Overview

Under `std` there are **18 module paths**, composed of two implementation forms:

| Implementation form          | Modules                                                                                                    | Registration source                                                                                                             |
| ---------------------------- | ---------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| native `StdModule`           | `assert` `concurrent` `convert` `dict` `fs` `io` `math` `net` `os` `range` `result` `string` `time` `weak` | `src/std/mod.rs:17-39` (module declarations) + `:460-482` (`all_module_infos()`)                                                |
| pure yx (`.yx` embedded)     | `list` `test` `option` `json`                                                                              | `src/std/yx_sources.rs:12-18`                                                                                                   |
| Dual implementation (merged) | `result`                                                                                                   | `src/std/result.rs` (utility family) + `src/std/result.yx` (`Result` type), merged in `src/frontend/module/registry.rs:315-334` |

**Platform availability**: `concurrent` / `fs` / `net` / `os` / `weak` and `io.read_line` /
`time.sleep` depend on operating system capabilities and are **not exported** on the `wasm32` target
(`#[cfg(not(target_arch = "wasm32"))]` on the module declarations in `src/std/mod.rs`).

---

## Chapter 1: Core Library

### 1.1 Basic Types

| Type           | Module         | Description                                                               |
| -------------- | -------------- | ------------------------------------------------------------------------- |
| `Option(T)`    | `std.option`   | Optional value (sum type)                                                 |
| `Result(T, E)` | `std.result`   | Error handling (sum type)                                                 |
| `Vec(T)`       | Core primitive | Runtime-length raw buffer; `std.list` is implemented on top of it         |
| `Dict(K, V)`   | Core primitive | Dictionary; `std.dict` provides module-level read/write functions         |
| `String`       | `std.string`   | String; `std.string` provides manipulation functions                      |
| `Array(T, N)`  | Core primitive | Fixed-size array (see syntax in [Syntax Specification §1.6.4](syntax.md)) |

> **`List(T)` / `Map(K, V)` are not standard library types**. `std.list` is a **module-level
> function collection** built on the core primitive `Vec(T)` (`src/std/list.yx:1-21`); `std.dict` is
> similarly a group of module-level functions operating on `Dict(K, V)` (`src/std/dict.rs`). Writing
> `List(T)` / `Map(K, V)` as types will fail at the type-checking stage.

### 1.2 Option Type

Full text of `src/std/option.yx`:

```
pub Option: (T: Type) -> Type = {
    some: (T) -> Option(T),
    none: () -> Option(T),
    Try(Option(T), T, Void),
}
```

**Variant construction** (must be type-qualified at expression position, see
[Syntax Specification §1.4.2](syntax.md)):

| Variant       | Syntax                | Description |
| ------------- | --------------------- | ----------- |
| `Option.some` | `Option(Int).some(5)` | Has a value |
| `Option.none` | `Option(Int).none()`  | No value    |

**Methods** (declared in the type body, **not module-level exports** — `option.is_failure(...)`
reports `E1042`):

| Method       | Signature                             | `some` arm              | `none` arm              |
| ------------ | ------------------------------------- | ----------------------- | ----------------------- |
| `is_failure` | `(T: Type)(self: &Option(T)) -> Bool` | `false`                 | `true`                  |
| `success`    | `(T: Type)(self: &Option(T)) -> T`    | Returns the payload     | `assert(false)`         |
| `residual`   | `(T: Type)(self: &Option(T)) -> Void` | `assert(false)`         | Returns `void`          |
| `from_error` | `(T: Type)(v: Void) -> Option(T)`     | Always returns `none()` | Always returns `none()` |

The `assert(false)` branch is a dead end of type `Never` (`Never <: T`, see
[Type System §2.2](type-system.md)), reporting `E6005` at runtime.

> **There is no `is_some` / `is_none` / `unwrap` / `unwrap_or` / `map`** — these names have **0
> hits** throughout `src/`. Use `is_failure()` to test for failure, and `success()` to retrieve the
> payload.
>
> **`?` propagation is currently unavailable on `Option`**: although the type body instantiates
> `Try(Option(T), T, Void)`, the type checker only recognizes `Result` (`o?` reports `E1081`).
> `from_error` is the internal bridge for `?`, so there is no reachable call site either
> (`o.from_error()` reports `E6006` at runtime).

### 1.3 Result Type

The `Result` type body (`src/std/result.yx:19-23`):

```
pub Result: (T: Type, E: Type) -> Type = {
    ok: (T) -> Result(T, E),
    err: (E) -> Result(T, E),
    Try(Result(T, E), T, E),
}
```

**Variant construction** (must be type-qualified — bare `ok(5)` reports
`E1001 Unknown variable: 'ok'`):

| Variant      | Syntax                        | Description   |
| ------------ | ----------------------------- | ------------- |
| `Result.ok`  | `Result(Int, String).ok(5)`   | Success value |
| `Result.err` | `Result(Int, String).err("e") | Error value   |

**Methods**: the `Try` four-method group in the type body (`is_failure` / `success` / `residual` /
`from_error`, callable only with method syntax) + 8 native utility exports
(`src/std/result.rs:71-126`):

```
is_ok:      (T: Type, E: Type)(self: &Result(T, E)) -> Bool
is_err:     (T: Type, E: Type)(self: &Result(T, E)) -> Bool
unwrap:     (T: Type, E: Type)(self: &Result(T, E)) -> T
unwrap_or:  (T: Type, E: Type)(self: &Result(T, E), default: T) -> T
unwrap_err: (T: Type, E: Type)(self: &Result(T, E)) -> E
code:       (self: &Error) -> String
message:    (self: &Error) -> String
error:      (code: &String, message: &String) -> Error
```

> **There is no `map` / `map_err`**. `?` propagation on `Result` is **available** (unlike `Option`).

**Error carrier and error codes (#323 M4)**:

The `Error` Err carrier of each std module carries normalized error codes, reusing the E6xxx/E7xxx
segments from RFC-013 as a cross-version stable contract — programs can branch on codes, and
`yx explain E6009` looks up the documentation. The runtime representation is
`Struct { code: String, message: String }` (`src/std/result.rs:52-68`). Registered codes are listed
in `RUNTIME_ERROR_CODES` at `src/std/result.rs:26-32`: `E6009` (range step invalid), `E6010`
(parse_int failure), `E6011` (parse_float failure), `E6012` (invalid code point), `E6013` (JSON
parse failure).

`Error` values can only be constructed by `result.error(code, message)` — the type family only
registers type identity, with no value-space constructor.

### 1.4 Error Propagation

```
ErrorPropagate ::= Expr '?'
```

The `?` operator automatically propagates errors of `Result` type (after `use std.result`,
match-variant destructuring is the explicit equivalent of `?` — the variant set is imported via
`use`, see [Syntax Specification §2.8](syntax.md)):

```
// On success, returns the value; on failure, returns err upward
data = fetch_data()?

// Conceptually equivalent form
data = match fetch_data() {
    ok(v) => v
    err(e) => return err(e)
}
```

Constraint: `?` can only appear in functions whose return type implements `Try`, where the outer
return is `Result(T, E)`; otherwise it reports `E1081`. The `Try` instantiation for `Option` is not
yet recognized by the checker (see §1.2).

### 1.5 Assertions (std.assert)

`std.assert` has only one native export (`src/std/assert.rs:23-30`):

```
assert: (cond: Bool, ?msg: String) -> Never
```

Two type families are also registered (`src/std/assert.rs:32-51`):

```
IsTrue: (b: Bool) -> Type = match b {
    true => Void,      // ⊤, program continues
    false => Never,    // ⊥, diverges
}

Assert: (cond: Bool) -> Type = IsTrue(cond)
```

**Current state**:

- The runtime `assert` is available — reports `E6005` when the condition is false; when true, the
  implementation returns `Void` (`src/std/assert.rs:82-83`), which is not in conflict with the
  declared `Never` (`Never <: T`).
- **There is no overload `Assert(IsTrue(cond))` as a return type**, nor any overload taking
  `Result`.
- The two type families **cannot be written at type position** (`Assert(true)` reports `E0010`,
  `assert.Assert(true)` reports `E0012`) — the compile-time refinement primitive is not yet wired
  up.

**Dispatch**:

| Condition                                              | Behavior                                                           |
| ------------------------------------------------------ | ------------------------------------------------------------------ |
| All free variables of `cond` are known at compile time | Compiler evaluates; true → erased, false → compile error           |
| Runtime free variables exist                           | Insert a runtime check, inject the flow-sensitive assumption set Γ |

`assert(false, "msg")` is equivalent to raise — no separate `throw`/`raise` keyword is needed.

---

## Chapter 2: IO Library

### 2.1 Standard Input and Output

`std.io` has 4 exports in total (`src/std/io.rs:53-77`):

```
print: (...args) -> Void
println: (...args) -> ()
read_line: () -> String            // Not exported on wasm32
format_fallback: (value, type_name: &String) -> String
```

> **There is no `read_char`**. Whole-file read/write (`read_file` / `write_file` / `append_file`)
> has been moved to `std.fs` (noted as #104 at `src/std/io.rs:69`).

### 2.2 Files and Directories

Path-level operations are all in `std.fs` (22 exports, `src/std/fs.rs:36-169`); handle-level
incremental read/write is in `std.os` (12 exports, `src/std/os.rs:26-89`). **There is no `File` /
`Dir` record type, nor any `create` / `delete` / `create_dir` / `delete_dir`.**

`std.os` (handle model, parameters uniformly `&File` — the handle can be reused):

```
open:  (path: &String, mode: &String) -> File
close: (file: &File) -> Void
read:  (file: &File, n: Int) -> String
write: (file: &File, content: String) -> Int
seek:  (file: &File, offset: Int) -> Bool
tell:  (file: &File) -> Int
flush: (file: &File) -> Void
get_env:  (name: &String) -> String
set_env:  (name: &String, value: &String) -> Void
args:     () -> String
chdir:    (path: &String) -> Bool
getcwd:   () -> String
```

`std.fs` (path model):

```
read_file / write_file / append_file      // Whole-file read/write
exists / is_file / is_dir / stat           // Metadata checks
mkdir / mkdir_all / rmdir / remove          // Directory and delete
copy / rename                              // Move
read_dir / walk                            // Directory enumeration
temp_dir / mkdtemp / tmpfile               // Temporary paths
path_join / path_basename / path_dirname / path_extension   // Pure path operations
```

---

## Chapter 3: Math Library

### 3.1 Basic Math Functions

`std.math` has 18 exports in total (`src/std/math.rs:20-75`). **The integer family and float family
are two independent names, with no implicit conversion or overloading**:

```
abs:   (n: Int) -> Int          // Integer absolute value
max:   (a: Int, b: Int) -> Int
min:   (a: Int, b: Int) -> Int
clamp: (value: Int, min: Int, max: Int) -> Int

fabs:  (n: Float) -> Float      // Float absolute value
fmax:  (a: Float, b: Float) -> Float
fmin:  (a: Float, b: Float) -> Float
pow:   (base: Float, exp: Float) -> Float
sqrt:  (n: Float) -> Float
floor: (n: Float) -> Float
ceil:  (n: Float) -> Float
round: (n: Float) -> Float
```

> **`abs` / `max` / `min` only accept `Int`** — there is no `abs: (x: Float) -> Float` overload; use
> `fabs` / `fmax` / `fmin` for floats.
>
> **`clamp` returns `E6007` when `min > max`** (`src/std/math.rs:117-130`), no longer panicking and
> killing the interpreter process.
>
> **No logarithmic family**: `log` / `log2` / `log10` have **0 hits** throughout `src/`.

### 3.2 Trigonometric Functions

```
sin: (n: Float) -> Float
cos: (n: Float) -> Float
tan: (n: Float) -> Float
```

Parameters are in **radians**. **There are no inverse trigonometric functions** — `asin` / `acos` /
`atan` / `atan2` have **0 hits** throughout `src/`.

### 3.3 Constants

```
PI:  Float = 3.141592653589793
E:   Float = 2.718281828459045
TAU: Float = 6.283185307179586
```

The export names are **uppercase** `PI` / `E` / `TAU` (`src/std/math.rs:71-73`); there is no
lowercase `pi` / `e`. Import by name and use directly: `use std.math.{PI, E, TAU}`.

---

## Chapter 4: String Library

### 4.1 String Operations

`std.string` has 21 exports in total (`src/std/string.rs:22-146`). Except for `format`, all only
borrow `&String` for reading:

```
split:      (s: &String, sep: &String) -> Vec(String)
trim:       (s: &String) -> String
upper:      (s: &String) -> String
lower:      (s: &String) -> String
replace:    (s: &String, old: &String, new: &String) -> String
contains:   (s: &String, sub: &String) -> Bool
starts_with: (s: &String, prefix: &String) -> Bool
ends_with:  (s: &String, suffix: &String) -> Bool
index_of:   (s: &String, sub: &String) -> Int
substring:  (s: &String, start: Int, end: Int) -> String
is_empty:   (s: &String) -> Bool
len:        (s: &String) -> Int
chars:      (s: &String) -> Vec(String)
concat:     (s1: &String, s2: &String) -> String
repeat:     (s: &String, n: Int) -> String
reverse:    (s: &String) -> String
format:     (format: &String, ...args) -> String
```

> **There is no `length`** — the length function is called `len`, and it returns the **UTF-8 byte
> length**.
>
> **There is no `find`** — the find-index function is called `index_of`, returning a **byte**
> offset, with `-1` for not found.
>
> **Only `trim` exists** — there is no `trim_left` / `trim_right`.

### 4.2 String Conversion and Code Points

```
parse_int:      (s: &String) -> Result(Int, Error)      // Failure code E6010
parse_float:    (s: &String) -> Result(Float, Error)    // Failure code E6011
char_code:      (s: &String, i: Int) -> Int            // Out of bounds returns -1
from_char_code: (n: Int) -> Result(String, Error)      // Invalid code point code E6012
```

Value-to-string conversion goes through `std.convert` (11 exports such as `to_string`).

---

## Chapter 5: Collection Library

### 5.1 List (std.list)

`std.list` is a **module-level function collection defined on `Vec(T)`** (`src/std/list.yx`),
**not** a record type. 25 exports (24 functions + 1 `Iter` record type):

```
empty:      (T: Type) -> Vec(T)
of:         (T: Type) -> (data: Vec(T)) -> Vec(T)

// Consume the source list, return a new list
push:       (A: Type) -> (list: Vec(A), item: A) -> Vec(A)
append:     (A: Type) -> (list: Vec(A), item: A) -> Vec(A)     // alias of push
prepend:    (A: Type) -> (list: Vec(A), item: A) -> Vec(A)
set:        (A: Type) -> (list: Vec(A), index: Int, value: A) -> Vec(A)
pop:        (A: Type) -> (list: Vec(A)) -> Vec(A)
remove_at:  (A: Type) -> (list: Vec(A), index: Int) -> Vec(A)

// Read-only borrow, return a new list / scalar
len:        (A: Type) -> (list: &Vec(A)) -> Int
is_empty:   (A: Type) -> (list: &Vec(A)) -> Bool
get:        (A: Type) -> (list: &Vec(A), index: Int) -> A
first:      (A: Type) -> (list: &Vec(A)) -> A
last:       (A: Type) -> (list: &Vec(A)) -> A
contains:   (A: Type) -> (list: &Vec(A), item: &A) -> Bool
find_index: (A: Type) -> (list: &Vec(A), item: &A) -> Int
slice:      (A: Type) -> (list: &Vec(A), start: Int, end: Int) -> Vec(A)
reverse:    (A: Type) -> (list: &Vec(A)) -> Vec(A)
concat:     (A: Type) -> (a: &Vec(A), b: &Vec(A)) -> Vec(A)
map:        (T: Type, R: Type) -> (list: &Vec(T), f: (item: T) -> R) -> Vec(R)
filter:     (T: Type) -> (list: &Vec(T), keep: (item: T) -> Bool) -> Vec(T)
reduce:     (T: Type, Acc: Type) -> (list: &Vec(T), f: (acc: Acc, item: T) -> Acc, init: Acc) -> Acc

// Iterator
Iter:       (T: Type) -> Type                 // { buf: Vec(T), pos: Int }
iter:       (T: Type) -> (list: Vec(T)) -> Iter(T)
has_next:   (T: Type) -> (it: &Iter(T)) -> Bool
next:       (T: Type) -> (it: &mut Iter(T)) -> T
```

**Boundary semantics** (`src/std/list.yx:76-205`): value access and slicing use the `[]` subscript
directly, **out-of-bounds immediately reports `E6003`**, returning neither a sentinel value nor
clamping the boundary. `first` / `last` on an empty list likewise report `E6003`. A **negative**
index for `remove_at` reports `E6003`, but when the index is **≥ length, it silently truncates**
(only subtracts 1 from `length`).

**There is no** `insert` / `clear` / `sort`, nor any in-place mutating method — `pop` / `remove_at`
receive by value and **return a new shortened list** (value semantics, the source list is consumed).

### 5.2 Dictionary (std.dict)

`std.dict` is a **module-level function collection operating on `Dict(K, V)`** (`src/std/dict.rs`),
**not** a record type. 11 exports:

```
new:     (K: Type, V: Type)() -> Dict(K, V)
get:     (K: Type, V: Type)(dict: &Dict(K, V), key: Any) -> Any    // Missing key reports E6008
set:     (K: Type, V: Type)(dict: Dict(K, V), key: Any, value: Any) -> Dict(K, V)
has:     (K: Type, V: Type)(dict: &Dict(K, V), key: Any) -> Bool
delete:  (K: Type, V: Type)(dict: Dict(K, V), key: Any) -> Dict(K, V)
keys:    (A: Type, B: Type, C: Type)(dict: &Dict(A, B)) -> Vec(C)
values:  (A: Type, B: Type, C: Type)(dict: &Dict(A, B)) -> Vec(C)
entries: (A: Type, B: Type, C: Type)(dict: &Dict(A, B)) -> Vec(C)
len:     (K: Type, V: Type)(dict: &Dict(K, V)) -> Int
is_empty:(K: Type, V: Type)(dict: &Dict(K, V)) -> Bool
merge:   (A: Type, B: Type)(a: &Dict(A, B), b: &Dict(A, B)) -> Dict(A, B)
```

> **There is no** `insert` / `clear` (`Dict` is a core primitive, and the capacity policy is not
> exposed). Use `dict.new()` for an empty dictionary — `{}` is an **empty block** (value `Void`),
> not an empty dictionary.

---

## Chapter 6: Iterator Library

### 6.1 Iterator Interface

`Iterator` is a **language interface** (RFC-011a), not a standard library module:

```
Iterator: (T: Type) -> Type = {
    Item: T,
    next: () -> Option(T),
    has_next: () -> Bool,
    map: (R: Type) -> ((f: (T) -> R) -> Iterator(R)),
    filter: (predicate: (T) -> Bool) -> Iterator(T),
    collect: () -> List(T),
    reduce: (R: Type) -> ((initial: R, f: (R, T) -> R) -> R),
    for_each: (f: (T) -> Void) -> Void
}
```

The **runtime implementation** of the protocol surface is provided by `std.range`
(`src/std/range.rs:29-90`), and `std.list` has its own separate `Iter` record. The two sides have
**opposite parameter forms**: `range.has_next` has no `&` (consumes the iterator by value) while
`range.next` has `&`; `list.has_next` / `list.next` both have `&` / `&mut`. For day-to-day
traversal, use `for ... in` directly.

> **There is no corresponding module for the `Iterator` protocol** — there is no module path named
> `iterator` under `std` (that name has **0 hits** throughout `src/`).

### 6.2 Iterator Adapters

```
Range: Type = {
    start: Int,
    end: Int,
    step: Int,
    Iterator(Int)
}
```

```
// Usage (iterator protocol: std.range.iter/has_next/next, for dispatches via static typing)
for i in 0..10 {
    print(i)
}

// Step form (double dot, no new keyword)
for i in 0..10..2 {
    print(i)
}
```

> **`Range(Int)` is now officially landed** — the named fields `r.start`/`r.end`/`r.step` are
> accessible; `x in r` at runtime goes through `std.range.contains` (boundary check + step
> alignment), proving that the pipeline recognizes the interval proposition
> `x >= r.start && x < r.end && (x - r.start) % r.step == 0` (intervals stay intervals, not
> materialized). A literal step=0 is rejected at compile time; a dynamic step=0 is now Result-ified:
> `std.range.iter` → `Result(Iterator, Error)` (code `E6009`), `std.range.contains` →
> `Result(Bool, Error)`, and the consumption point uses `?` to propagate along the call stack or
> `result.unwrap` to branch explicitly; the `for`/`in` sugar desugars in `ir_gen` and unwraps; the
> Err branch (dynamic step=0) fails explicitly (`abort_invalid_step`), never silently dead-looping.

---

## Appendix A: Standard Library Module Index

The table below lists only **modules that actually exist** (corresponding to the two tables in
[Standard Library Reference](../stdlib/index.md)).

| Module        | Implementation | Description                                                                               |
| ------------- | -------------- | ----------------------------------------------------------------------------------------- |
| `std.assert`  | native         | Runtime `assert` (+ two type families `IsTrue` / `Assert` not yet usable)                 |
| `std.option`  | yx             | `Option(T)` optional value and sum type                                                   |
| `std.result`  | Dual           | `Result(T, E)` and `Error`; the only currently usable site for `?` propagation            |
| `std.list`    | yx             | List operations on `Vec(T)` and the `Iter` iterator                                       |
| `std.dict`    | native         | `Dict(K, V)` read/write, key-value views, and merging                                     |
| `std.string`  | native         | String search, split, format, parse, and code points                                      |
| `std.math`    | native         | Integer family, float family, trigonometric functions, and `PI` / `E` / `TAU`             |
| `std.time`    | native         | Unix timestamp, UTC format and parse, `datetime_*` accessors                              |
| `std.range`   | native         | `Range` iterator protocol, interval predicates, and lazy adapters                         |
| `std.json`    | yx             | JSON parse and serialize (RFC 8259)                                                       |
| `std.convert` | native         | Conversion from any value to `String`                                                     |
| `std.test`    | yx             | Test assertion library (value semantics, RFC-036 §3), the first pure-yx dogfooding module |

### A.2 Platform-Dependent Modules

| Module           | Implementation | Dependency                                          | wasm32                           |
| ---------------- | -------------- | --------------------------------------------------- | -------------------------------- |
| `std.io`         | native         | Standard I/O                                        | Only `read_line` is not exported |
| `std.fs`         | native         | File system                                         | Not exported                     |
| `std.os`         | native         | File/process                                        | Not exported                     |
| `std.net`        | native         | Network (`ureq` + rustls TLS, synchronous blocking) | Not exported                     |
| `std.concurrent` | native         | Thread                                              | Not exported                     |
| `std.weak`       | native         | Atomic reference counting                           | Not exported                     |

### A.3 Removed Module Names

The following names were once module names of `std`, but have **0 hits throughout `src/`**, and must
no longer be used (verification method: full-text search of these identifiers in the `src/`
directory):

| Old name     | Now provided by                                   |
| ------------ | ------------------------------------------------- |
| `collection` | `std.list` (`Vec(T)`) + `std.dict` (`Dict(K, V)`) |
| `array`      | Core primitive `Array(T, N)`                      |
| `iterator`   | Protocol surface of `std.range` / `std.list`      |
| `file`       | `std.fs`                                          |
| `dir`        | `std.fs`                                          |
| `math.trig`  | `std.math` (`sin` / `cos` / `tan`)                |
| `math.log`   | No logarithmic family                             |
| `random`     | None                                              |
| `regex`      | None                                              |
