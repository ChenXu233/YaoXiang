# Standard Library Specification

This document defines the standard library specification for the YaoXiang programming language.

> **This chapter is a language-level specification** (design conventions and semantic model). The
> **signatures and runnable examples** for each module are documented in the
> [Standard Library Reference](../stdlib/index.md)—that section is derived from
> `StdModule::exports()` and guarded by a CI gate. Every signature in this document is copied
> verbatim from the implementations under `src/std/` (`*.rs` / `*.yx`).

---

## Chapter 0: Module Overview

There are **18 module paths** under `std`, implemented in two forms:

| Implementation form          | Modules                                                                                                    | Registration source                                                                                                                          |
| ---------------------------- | ---------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| native `StdModule`           | `assert` `concurrent` `convert` `dict` `fs` `io` `math` `net` `os` `range` `result` `string` `time` `weak` | `src/std/mod.rs:17-39` (module declarations) + `:460-482` (`all_module_infos()`)                                                             |
| Pure yx (`.yx` embedded)     | `list` `test` `option` `json`                                                                              | `src/std/yx_sources.rs:12-18`                                                                                                                |
| Dual implementation (merged) | `result`                                                                                                   | `src/std/result.rs` (utility family) + `src/std/result.yx` (`Result` type); `src/frontend/module/registry.rs:315-334` performs a union merge |

**Platform availability**: `concurrent` / `fs` / `net` / `os` / `weak`, as well as `io.read_line`
and `time.sleep`, depend on operating system capabilities and are **not exported** for the `wasm32`
target (the `#[cfg(not(target_arch = "wasm32"))]` on each module declaration in `src/std/mod.rs`).

---

## Chapter 1: Core Library

### 1.1 Basic Types

| Type           | Module         | Description                                                              |
| -------------- | -------------- | ------------------------------------------------------------------------ |
| `Option(T)`    | `std.option`   | Optional value (sum type)                                                |
| `Result(T, E)` | `std.result`   | Error handling (sum type)                                                |
| `Vec(T)`       | Core primitive | Raw buffer with a runtime length; `std.list` is implemented on top of it |
| `Dict(K, V)`   | Core primitive | Dictionary; `std.dict` provides module-level read/write functions        |
| `String`       | `std.string`   | String; `std.string` provides operation functions                        |
| `Array(T, N)`  | Core primitive | Fixed-size array (see [Syntax Specification §1.6.4](syntax.md))          |

> **`List(T)` / `Map(K, V)` are not standard library types**. `std.list` is a **module-level set of
> functions**, built on the core primitive `Vec(T)` (`src/std/list.yx:1-21`); likewise, `std.dict`
> is a set of module-level functions that operate on `Dict(K, V)` (`src/std/dict.rs`). Writing
> `List(T)` / `Map(K, V)` as types will fail during type checking.

### 1.2 The Option Type

The complete contents of `src/std/option.yx`:

```
Option: (T: Type) -> Type = {
    some: (T) -> Option(T),
    none: () -> Option(T),
    Try(Option(T), T, Void),
}
```

**Variant construction** (expression positions must be type-qualified; see
[Syntax Specification §1.4.2](syntax.md)):

| Variant       | Syntax                | Description  |
| ------------- | --------------------- | ------------ |
| `Option.some` | `Option(Int).some(5)` | Has a value  |
| `Option.none` | `Option(Int).none()`  | Has no value |

**Methods** (declared in the type body; they are **not module-level
exports**—`option.is_failure(...)` reports `E1042`):

| Method       | Signature                             | `some` arm              | `none` arm              |
| ------------ | ------------------------------------- | ----------------------- | ----------------------- |
| `is_failure` | `(T: Type)(self: &Option(T)) -> Bool` | `false`                 | `true`                  |
| `success`    | `(T: Type)(self: &Option(T)) -> T`    | Returns the payload     | `assert(false)`         |
| `residual`   | `(T: Type)(self: &Option(T)) -> Void` | `assert(false)`         | Returns `void`          |
| `from_error` | `(T: Type)(v: Void) -> Option(T)`     | Always returns `none()` | Always returns `none()` |

The `assert(false)` branch is a dead end of type `Never` (`Never <: T`; see
[Type System §2.2](type-system.md)) and reports `E6005` at runtime.

> **There is no `is_some` / `is_none` / `unwrap` / `unwrap_or` / `map`**—these names have zero
> occurrences anywhere under `src/`. Use `is_failure()` to test for failure and `success()` to
> retrieve the payload.
>
> **`?` propagation is currently unavailable on `Option`**: Although the type body instantiates
> `Try(Option(T), T, Void)`, the type checker recognizes only `Result` (`o?` reports `E1081`).
> `from_error` is the internal bridge for `?`, so it also has no reachable call site
> (`o.from_error()` reports `E6006` at runtime).

### 1.3 The Result Type

The `Result` type itself (`src/std/result.yx:19-23`):

```
Result: (T: Type, E: Type) -> Type = {
    ok: (T) -> Result(T, E),
    err: (E) -> Result(T, E),
    Try(Result(T, E), T, E),
}
```

**Variant construction** (type qualification is required—bare `ok(5)` reports
`E1001 Unknown variable: 'ok'`):

| Variant      | Syntax                         | Description   |
| ------------ | ------------------------------ | ------------- |
| `Result.ok`  | `Result(Int, String).ok(5)`    | Success value |
| `Result.err` | `Result(Int, String).err("e")` | Error value   |

**Methods**: the four `Try` methods in the type body (`is_failure` / `success` / `residual` /
`from_error`, callable only with method syntax) plus eight native utility exports
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

> **There is no `map` / `map_err`**. `?` propagation on `Result` **is available** (unlike on
> `Option`).

**Error carrier and error codes (#323 M4)**:

The `Error` Err carrier used by the std modules carries normalized error codes. The codes reuse the
E6xxx/E7xxx ranges from RFC-013 as a stable cross-version contract—programs can branch on the codes,
and `yx explain E6009` can be used to look up the documentation. The runtime representation is
`Struct { code: String, message: String }` (`src/std/result.rs:52-68`). Registered codes are listed
in `src/std/result.rs:26-32` under `RUNTIME_ERROR_CODES`: `E6009` (invalid range step), `E6010`
(parse_int failure), `E6011` (parse_float failure), `E6012` (invalid code point), and `E6013` (JSON
parsing failure).

`Error` values can only be constructed with `result.error(code, message)`—the type family registers
only the type identity and provides no value-space constructor.

### 1.4 Error Propagation

```
ErrorPropagate ::= Expr '?'
```

The `?` operator automatically propagates errors of type `Result` (after `use std.result`, match
variant destructuring is the explicit equivalent of `?`—the variant set is imported with `use`; see
[Syntax Specification §2.8](syntax.md)):

```
// Return the value on success, or propagate the error upward
data = fetch_data()?

// Conceptually equivalent form
data = match fetch_data() {
    ok(v) -> v
    err(e) -> return err(e)
}
```

Constraint: `?` may only appear in a function whose return type implements `Try`, with an outer
return type of `Result(T, E)`; otherwise, `E1081` is reported. The `Option` `Try` instantiation is
not yet recognized by the checker (see §1.2).

### 1.5 Assertions (`std.assert`)

`std.assert` has only one native export (`src/std/assert.rs:23-30`):

```
assert: (cond: Bool, ?msg: String) -> Never
```

Two type families are also registered (`src/std/assert.rs:32-51`):

```
IsTrue: (b: Bool) -> Type = match b {
    true -> Void,       // ⊤, execution continues
    false -> Never,     // ⊥, divergence
}

Assert: (cond: Bool) -> Type = IsTrue(cond)
```

**Current status**:

- The runtime `assert` is available—it reports `E6005` when the condition is false; when the
  condition is true, the implementation returns `Void` (`src/std/assert.rs:82-83`), which is
  consistent with the declared `Never` (`Never <: T`)
- **There is no overload using `Assert(IsTrue(cond))` as the return type**, nor is there an overload
  that accepts a `Result`
- The two type families **cannot be written in type positions** (`Assert(true)` reports `E0010`, and
  `assert.Assert(true)` reports `E0012`)—compile-time refinement primitives are not yet connected

**Dispatch**:

| Condition                                              | Behavior                                                                         |
| ------------------------------------------------------ | -------------------------------------------------------------------------------- |
| All free variables of `cond` are known at compile time | The compiler evaluates it: true → erased, false → compile-time error             |
| Runtime free variables exist                           | A runtime check is inserted, and the flow-sensitive assumption set Γ is injected |

`assert(false, "msg")` is equivalent to raising an error—no separate throw/raise keyword is needed.

---

## Chapter 2: I/O Library

### 2.1 Standard Input and Output

`std.io` has four exports in total (`src/std/io.rs:53-77`):

```
print: (...args) -> Void
println: (...args) -> ()
read_line: () -> String            // Not exported on wasm32
format_fallback: (value, type_name: &String) -> String
```

> **There is no `read_char`**. Whole-file reads and writes (`read_file` / `write_file` /
> `append_file`) have moved to `std.fs` (`src/std/io.rs:69` notes #104).

### 2.2 Files and Directories

All path-level operations are in `std.fs` (22 exports, `src/std/fs.rs:36-169`); handle-level
incremental reads and writes are in `std.os` (12 exports, `src/std/os.rs:26-89`). **There are no
`File` / `Dir` record types, and no `create` / `delete` / `create_dir` / `delete_dir`.**

`std.os` (handle model; all parameters are `&File`—handles may be reused):

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
read_file / write_file / append_file      // Whole-file reads and writes
exists / is_file / is_dir / stat           // Metadata checks
mkdir / mkdir_all / rmdir / remove          // Directory management and deletion
copy / rename                              // Moving
read_dir / walk                            // Directory enumeration
temp_dir / mkdtemp / tmpfile               // Temporary paths
path_join / path_basename / path_dirname / path_extension   // Pure path operations
```

---

## Chapter 3: Mathematics Library

### 3.1 Basic Mathematical Functions

`std.math` has 18 exports in total (`src/std/math.rs:20-75`). **The integer family and
floating-point family have two independent sets of names, with no implicit conversions or
overloads**:

```
abs:   (n: Int) -> Int          // Integer absolute value
max:   (a: Int, b: Int) -> Int
min:   (a: Int, b: Int) -> Int
clamp: (value: Int, min: Int, max: Int) -> Int

fabs:  (n: Float) -> Float      // Floating-point absolute value
fmax:  (a: Float, b: Float) -> Float
fmin:  (a: Float, b: Float) -> Float
pow:   (base: Float, exp: Float) -> Float
sqrt:  (n: Float) -> Float
floor: (n: Float) -> Float
ceil:  (n: Float) -> Float
round: (n: Float) -> Float
```

> **`abs` / `max` / `min` accept only `Int`**—there is no `abs: (x: Float) -> Float` overload; use
> `fabs` / `fmax` / `fmin` for floating-point values.
>
> **`clamp` returns `E6007` when `min > max`** (`src/std/math.rs:117-130`) and no longer panics the
> interpreter process.
>
> **There is no logarithm family**: `log` / `log2` / `log10` have zero occurrences anywhere under
> `src/`.

### 3.2 Trigonometric Functions

```
sin: (n: Float) -> Float
cos: (n: Float) -> Float
tan: (n: Float) -> Float
```

The parameter is in **radians**. **There are no inverse trigonometric functions**—`asin` / `acos` /
`atan` / `atan2` have zero occurrences anywhere under `src/`.

### 3.3 Constants

```
PI:  Float = 3.141592653589793
E:   Float = 2.718281828459045
TAU: Float = 6.283185307179586
```

The export names are the uppercase `PI` / `E` / `TAU` (`src/std/math.rs:71-73`); there are no
lowercase `pi` / `e` versions. Import them by name: `use std.math.{PI, E, TAU}`.

---

## Chapter 4: String Library

### 4.1 String Operations

`std.string` has 21 exports in total (`src/std/string.rs:22-146`). Except for `format`, all of them
take a read-only borrow of `&String`:

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

> **There is no `length`**—the length function is called `len` and returns the **UTF-8 byte
> length**.
>
> **There is no `find`**—the search-index function is called `index_of` and returns a **byte**
> offset, or `-1` if there is no match.
>
> **There is only one `trim`**—there are no `trim_left` / `trim_right` functions.

### 4.2 String Conversion and Code Points

```
parse_int:      (s: &String) -> Result(Int, Error)      // Failure code E6010
parse_float:    (s: &String) -> Result(Float, Error)    // Failure code E6011
char_code:      (s: &String, i: Int) -> Int            // Out-of-bounds returns -1
from_char_code: (n: Int) -> Result(String, Error)      // Invalid code point error code E6012
```

Value-to-string conversion is handled by `std.convert` (`to_string` and 10 other exports).

---

## Chapter 5: Collection Library

### 5.1 Lists (`std.list`)

`std.list` is a **module-level set of functions defined over `Vec(T)`** (`src/std/list.yx`), **not**
a record type. It has 25 exports (24 functions and one `Iter` record type):

```
empty:      (T: Type) -> Vec(T)
of:         (T: Type) -> (data: Vec(T)) -> Vec(T)

// Consumes the source list and returns a new list
push:       (A: Type) -> (list: Vec(A), item: A) -> Vec(A)
append:     (A: Type) -> (list: Vec(A), item: A) -> Vec(A)     // Alias for push
prepend:    (A: Type) -> (list: Vec(A), item: A) -> Vec(A)
set:        (A: Type) -> (list: Vec(A), index: Int, value: A) -> Vec(A)
pop:        (A: Type) -> (list: Vec(A)) -> Vec(A)
remove_at:  (A: Type) -> (list: Vec(A), index: Int) -> Vec(A)

// Read-only borrow; returns a new list or scalar
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

**Boundary semantics** (`src/std/list.yx:76-205`): value access and slicing use `[]` indexing
directly; **out-of-bounds access reports `E6003`**, returning neither a sentinel value nor a clamped
boundary. Applying `first` / `last` to an empty list also reports `E6003`. A negative index for
`remove_at` reports `E6003`, but an index **greater than or equal to the length is silently
truncated** (only `length` is decremented by 1).

**There is no `insert` / `clear` / `sort`**, nor are there any in-place mutating method forms—`pop`
/ `remove_at` take their arguments by value and **return a shortened new list** (value semantics;
the source list is consumed).

### 5.2 Dictionaries (`std.dict`)

`std.dict` is a **module-level set of functions that operate on `Dict(K, V)`** (`src/std/dict.rs`),
**not** a record type. It has 11 exports:

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

> **There is no `insert` / `clear`** (`Dict` is a core primitive, and its capacity policy is not
> exposed). Create an empty dictionary with `dict.new()`—`{}` is an **empty block** (a `Void`
> value), not an empty dictionary.

---

## Chapter 6: Iterator Library

### 6.1 The Iterator Interface

`Iterator` is a **language-level interface** (RFC-011a), not a standard library module:

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

The **runtime implementation** of the protocol is provided by `std.range`
(`src/std/range.rs:29-90`), while `std.list` has its own separate `Iter` record. The two sides have
**opposite parameter shapes**: `range.has_next` has no `&` (it consumes the iterator by value),
whereas `range.next` has `&`; `list.has_next` / `list.next` both have `&` / `&mut`. For ordinary
iteration, use `for ... in` directly.

> **There is no separate module corresponding to the `Iterator` protocol**—there is no module path
> named `iterator` under `std` (this name has zero occurrences anywhere under `src/`).

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
// Usage (iterator protocol: std.range.iter/has_next/next; for dispatch is based on the static type)
for i in 0..10 {
    print(i)
}

// Step syntax (double dots, no new keyword)
for i in 0..10..2 {
    print(i)
}
```

> **`Range(Int)` has now been formally implemented**—the named fields `r.start`/`r.end`/`r.step` are
> accessible; at runtime, `x in r` uses `std.range.contains` (boundary checks and step alignment),
> demonstrating that the pipeline recognizes the interval proposition
> `x >= r.start && x < r.end && (x - r.start) % r.step == 0` (the interval remains an interval and
> is not materialized). A literal `step=0` is rejected at compile time; a dynamic `step=0` is now
> represented as a `Result`: `std.range.iter` → `Result(Iterator, Error)` (code `E6009`), and
> `std.range.contains` → `Result(Bool, Error)`. At consumption points, use `?` to propagate the
> error along the call stack or `result.unwrap` to branch explicitly. The `for`/`in` desugaring
> unwraps the result in `ir_gen`, and the Err branch (dynamic `step=0`) fails explicitly
> (`abort_invalid_step`); it never enters a silent infinite loop.

---

## Appendix A: Standard Library Module Index

The following table lists only module paths that **actually exist** (corresponding to the two tables
in the [Standard Library Reference](../stdlib/index.md)).

| Module        | Implementation      | Description                                                                               |
| ------------- | ------------------- | ----------------------------------------------------------------------------------------- |
| `std.assert`  | native              | Runtime `assert` (+ two currently unusable type families, `IsTrue` / `Assert`)            |
| `std.option`  | yx                  | `Option(T)` optional values and sum types                                                 |
| `std.result`  | Dual implementation | `Result(T, E)` and `Error`; currently the only place where `?` propagation works          |
| `std.list`    | yx                  | List operations over `Vec(T)` and the `Iter` iterator                                     |
| `std.dict`    | native              | `Dict(K, V)` reads and writes, key-value views, and merging                               |
| `std.string`  | native              | String searching, splitting, formatting, parsing, and code points                         |
| `std.math`    | native              | Integer family, floating-point family, trigonometry, and `PI` / `E` / `TAU`               |
| `std.time`    | native              | Unix timestamps, UTC formatting and parsing, and `datetime_*` accessors                   |
| `std.range`   | native              | `Range` iterator protocol, interval predicates, and lazy adapters                         |
| `std.json`    | yx                  | JSON parsing and serialization (RFC 8259)                                                 |
| `std.convert` | native              | Conversion of arbitrary values to `String`                                                |
| `std.test`    | yx                  | Test assertion library (value semantics, RFC-036 §3), the first pure yx dogfooding module |

### A.2 Platform-Dependent Modules

| Module           | Implementation | Dependency                                             | wasm32                           |
| ---------------- | -------------- | ------------------------------------------------------ | -------------------------------- |
| `std.io`         | native         | Standard I/O                                           | Only `read_line` is not exported |
| `std.fs`         | native         | Filesystem                                             | Not exported                     |
| `std.os`         | native         | Files/processes                                        | Not exported                     |
| `std.net`        | native         | Networking (`ureq` + rustls TLS, synchronous blocking) | Not exported                     |
| `std.concurrent` | native         | Threads                                                | Not exported                     |
| `std.weak`       | native         | Atomic reference counting                              | Not exported                     |

### A.3 Removed Module Names

The following names were once module names under `std`, but have **zero occurrences anywhere under
`src/`** and must no longer be used (to verify: search the full contents of the `src/` directory for
these identifiers):

| Old name     | Now provided by                                   |
| ------------ | ------------------------------------------------- |
| `collection` | `std.list` (`Vec(T)`) + `std.dict` (`Dict(K, V)`) |
| `array`      | Core primitive `Array(T, N)`                      |
| `iterator`   | The protocol surface of `std.range` / `std.list`  |
| `file`       | `std.fs`                                          |
| `dir`        | `std.fs`                                          |
| `math.trig`  | `std.math` (`sin` / `cos` / `tan`)                |
| `math.log`   | No logarithm family                               |
| `random`     | None                                              |
| `regex`      | None                                              |
