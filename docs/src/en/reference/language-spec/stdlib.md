# Standard Library Specification

This file defines the standard library specification of the YaoXiang programming language.

> **This chapter is a language-level specification** (design conventions and semantic models). The
> **signatures and runnable examples for each module** are in
> [Standard Library Reference](../stdlib/index.md) — that part is derived from
> `StdModule::exports()` and guarded by a gate. Every signature in this file is taken verbatim from
> the implementation under `src/std/` (`*.rs` / `*.yx`).

---

## Chapter 0: Module Overview

There are **18 module paths** under `std`, composed of two implementation forms:

| Implementation Form          | Modules                                                                                                    | Registration Source                                                                                                             |
| ---------------------------- | ---------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| native `StdModule`           | `assert` `concurrent` `convert` `dict` `fs` `io` `math` `net` `os` `range` `result` `string` `time` `weak` | `src/std/mod.rs:17-39` (module declarations) + `:460-482` (`all_module_infos()`)                                                |
| pure yx (`.yx` embedded)     | `list` `test` `option` `json`                                                                              | `src/std/yx_sources.rs:12-18`                                                                                                   |
| Dual implementation (merged) | `result`                                                                                                   | `src/std/result.rs` (utility family) + `src/std/result.yx` (`Result` type); merged by `src/frontend/module/registry.rs:315-334` |

**Platform availability**: `concurrent` / `fs` / `net` / `os` / `weak` and `io.read_line` /
`time.sleep` depend on operating system capabilities and are **not exported** on the `wasm32` target
(`#[cfg(not(target_arch = "wasm32"))]` on the module declarations in `src/std/mod.rs`).

---

## Chapter 1: Core Library

### 1.1 Basic Types

| Type           | Module         | Description                                                            |
| -------------- | -------------- | ---------------------------------------------------------------------- |
| `Option(T)`    | `std.option`   | Optional value (sum type)                                              |
| `Result(T, E)` | `std.result`   | Error handling (sum type)                                              |
| `Vec(T)`       | Core primitive | Runtime-length raw buffer; `std.list` is implemented on top of it      |
| `Dict(K, V)`   | Core primitive | Dictionary; `std.dict` provides module-level read/write functions      |
| `String`       | `std.string`   | String; `std.string` provides operation functions                      |
| `Array(T, N)`  | Core primitive | Fixed-size array (syntax see [Syntax Specification §1.6.4](syntax.md)) |

> **`List(T)` / `Map(K, V)` are not standard library types**. `std.list` is a **module-level
> function collection** built on the core primitive `Vec(T)` (`src/std/list.yx:1-21`); `std.dict` is
> similarly a group of module-level functions operating on `Dict(K, V)` (`src/std/dict.rs`). Writing
> `List(T)` / `Map(K, V)` as types will fail at the type-checking stage.

### 1.2 Option Type

Full text of `src/std/option.yx`:

```
Option: (T: Type) -> Type = {
    some: (T) -> Option(T),
    none: () -> Option(T),
    Try(Option(T), T, Void),
}
```

**Variant construction** (expression positions must be type-qualified, see
[Syntax Specification §1.4.2](syntax.md)):

| Variant       | Syntax                | Description |
| ------------- | --------------------- | ----------- |
| `Option.some` | `Option(Int).some(5)` | Has value   |
| `Option.none` | `Option(Int).none()`  | No value    |

**Methods** (declared in the type body, **not module-level exports** — `option.is_failure(...)`
reports `E1042`):

| Method       | Signature                             | `some` arm              | `none` arm              |
| ------------ | ------------------------------------- | ----------------------- | ----------------------- |
| `is_failure` | `(T: Type)(self: &Option(T)) -> Bool` | `false`                 | `true`                  |
| `success`    | `(T: Type)(self: &Option(T)) -> T`    | Returns payload         | `assert(false)`         |
| `residual`   | `(T: Type)(self: &Option(T)) -> Void` | `assert(false)`         | Returns `void`          |
| `from_error` | `(T: Type)(v: Void) -> Option(T)`     | Always returns `none()` | Always returns `none()` |

The `assert(false)` branch is a dead end of `Never` type (`Never <: T`, see
[Type System §2.2](type-system.md)), and reports `E6005` at runtime.

> **There are no `is_some` / `is_none` / `unwrap` / `unwrap_or` / `map`** — these names have 0 hits
> in the entire `src/` repository. Use `is_failure()` to check for failure, and `success()` to
> retrieve the payload.
>
> **`?` propagation is currently unavailable on `Option`**: Although the type body instantiates
> `Try(Option(T), T, Void)`, the type checker only recognizes `Result` (`o?` reports `E1081`).
> `from_error` is the internal bridge for `?`, so there is also no reachable call site
> (`o.from_error()` reports `E6006` at runtime).

### 1.3 Result Type

`Result` type body (`src/std/result.yx:19-23`):

```
Result: (T: Type, E: Type) -> Type = {
    ok: (T) -> Result(T, E),
    err: (E) -> Result(T, E),
    Try(Result(T, E), T, E),
}
```

**Variant construction** (must be type-qualified — bare `ok(5)` reports
`E1001 Unknown variable: 'ok'`):

| Variant      | Syntax                         | Description   |
| ------------ | ------------------------------ | ------------- |
| `Result.ok`  | `Result(Int, String).ok(5)`    | Success value |
| `Result.err` | `Result(Int, String).err("e")` | Error value   |

**Methods**: The `Try` four methods of the type body (`is_failure` / `success` / `residual` /
`from_error`, can only be called via method syntax) + 8 native utility exports
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

> **No `map` / `map_err`**. `?` propagation on `Result` is **available** (unlike `Option`).

**Error carrier and error codes (#323 M4)**:

The `Error` Err carrier of std modules carries normalized error codes, reusing the E6xxx/E7xxx
segments of RFC-013, as a cross-version stable contract — programs can branch by code, and
`yx explain E6009` can look up documentation. The runtime representation is
`Struct { code: String, message: String }` (`src/std/result.rs:52-68`). For registered codes see
`RUNTIME_ERROR_CODES` at `src/std/result.rs:26-32`: `E6009` (range step invalid), `E6010` (parse_int
failed), `E6011` (parse_float failed), `E6012` (invalid code point), `E6013` (JSON parse failed).

`Error` values can only be constructed by `result.error(code, message)` — the type family only
registers type identity, with no value-space constructors.

### 1.4 Error Propagation

```
ErrorPropagate ::= Expr '?'
```

The `?` operator automatically propagates errors of `Result` type (after `use std.result`, match
variant destructuring is the explicit equivalent form of `?` — the variant set is imported along
with `use`, see [Syntax Specification §2.8](syntax.md)):

```
// On success returns the value; on failure returns err upward
data = fetch_data()?

// Conceptually equivalent form
data = match fetch_data() {
    ok(v) => v
    err(e) => return err(e)
}
```

Constraint: `?` can only appear in functions whose return type implements `Try`, and the outer
function returns `Result(T, E)`; otherwise it reports `E1081`. The `Try` instantiation of `Option`
is not yet recognized by the checker (see §1.2).

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

**Current status**:

- The runtime `assert` is available — when the condition is false it reports `E6005`; when true, the
  implementation returns `Void` (`src/std/assert.rs:82-83`), which does not contradict the declared
  `Never` (`Never <: T`)
- **There is no `Assert(IsTrue(cond))` overload as a return type**, nor is there an overload that
  accepts `Result`
- The two type families **cannot be written at type position** (`Assert(true)` reports `E0010`,
  `assert.Assert(true)` reports `E0012`) — the compile-time refinement primitive is not yet wired up

**Dispatch**:

| Condition                                            | Behavior                                                     |
| ---------------------------------------------------- | ------------------------------------------------------------ |
| All free variables of cond are known at compile time | Compiler evaluates, true → erased, false → compile error     |
| There exist runtime free variables                   | Insert runtime check, inject flow-sensitive assumption set Γ |

`assert(false, "msg")` is equivalent to raise — no separate throw/raise keyword is needed.

---

## Chapter 2: IO Library

### 2.1 Standard Input and Output

`std.io` has a total of 4 exports (`src/std/io.rs:53-77`):

```
print: (...args) -> Void
println: (...args) -> ()
read_line: () -> String            // not exported on wasm32
format_fallback: (value, type_name: &String) -> String
```

> **No `read_char`**. Whole-file read/write (`read_file` / `write_file` / `append_file`) have been
> moved to `std.fs` (noted as #104 at `src/std/io.rs:69`).

### 2.2 Files and Directories

All path-level operations are in `std.fs` (22 exports, `src/std/fs.rs:36-169`); handle-level
incremental read/write is in `std.os` (12 exports, `src/std/os.rs:26-89`). **There are no `File` /
`Dir` record types, and no `create` / `delete` / `create_dir` / `delete_dir`.**

`std.os` (handle model, all parameters are `&File` — handles can be reused):

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
read_file / write_file / append_file      // whole-file read/write
exists / is_file / is_dir / stat           // metadata checks
mkdir / mkdir_all / rmdir / remove          // directory and deletion
copy / rename                              // moving
read_dir / walk                            // directory enumeration
temp_dir / mkdtemp / tmpfile               // temporary paths
path_join / path_basename / path_dirname / path_extension   // pure path operations
```

---

## Chapter 3: Math Library

### 3.1 Basic Math Functions

`std.math` has a total of 18 exports (`src/std/math.rs:20-75`). **The integer family and the float
family are two independent names, with no implicit conversion or overloading**:

```
abs:   (n: Int) -> Int          // integer absolute value
max:   (a: Int, b: Int) -> Int
min:   (a: Int, b: Int) -> Int
clamp: (value: Int, min: Int, max: Int) -> Int

fabs:  (n: Float) -> Float      // float absolute value
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
> **`clamp` returns `E6007` when `min > max`** (`src/std/math.rs:117-130`), and no longer panics to
> kill the interpreter process.
>
> **No logarithm function family**: `log` / `log2` / `log10` have 0 hits in the entire repository.

### 3.2 Trigonometric Functions

```
sin: (n: Float) -> Float
cos: (n: Float) -> Float
tan: (n: Float) -> Float
```

Parameters are in **radians**. **No inverse trigonometric functions** — `asin` / `acos` / `atan` /
`atan2` have 0 hits in the entire repository.

### 3.3 Constants

```
PI:  Float = 3.141592653589793
E:   Float = 2.718281828459045
TAU: Float = 6.283185307179586
```

The export names are **uppercase** `PI` / `E` / `TAU` (`src/std/math.rs:71-73`), with no lowercase
`pi` / `e`. Import by name and use: `use std.math.{PI, E, TAU}`.

---

## Chapter 4: String Library

### 4.1 String Operations

`std.string` has a total of 21 exports (`src/std/string.rs:22-146`). Except for `format`, all only
borrow `&String`:

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

> **No `length`** — the length function is named `len`, and it returns the **UTF-8 byte length**.
>
> **No `find`** — the index lookup function is named `index_of`, returning a **byte** offset, and
> `-1` when not found.
>
> **Only `trim`** — there is no `trim_left` / `trim_right`.

### 4.2 String Conversion and Code Points

```
parse_int:      (s: &String) -> Result(Int, Error)      // failure code E6010
parse_float:    (s: &String) -> Result(Float, Error)    // failure code E6011
char_code:      (s: &String, i: Int) -> Int            // out of bounds returns -1
from_char_code: (n: Int) -> Result(String, Error)      // invalid code point code E6012
```

Value-to-string conversion goes through `std.convert` (11 exports such as `to_string`).

---

## Chapter 5: Collections Library

### 5.1 List (std.list)

`std.list` is a **module-level function collection defined on `Vec(T)`** (`src/std/list.yx`),
**not** a record type. 25 exports (24 functions + 1 `Iter` record type):

```
empty:      (T: Type) -> Vec(T)
of:         (T: Type) -> (data: Vec(T)) -> Vec(T)

// Consumes the source list, returns a new list
push:       (A: Type) -> (list: Vec(A), item: A) -> Vec(A)
append:     (A: Type) -> (list: Vec(A), item: A) -> Vec(A)     // alias of push
prepend:    (A: Type) -> (list: Vec(A), item: A) -> Vec(A)
set:        (A: Type) -> (list: Vec(A), index: Int, value: A) -> Vec(A)
pop:        (A: Type) -> (list: Vec(A)) -> Vec(A)
remove_at:  (A: Type) -> (list: Vec(A), index: Int) -> Vec(A)

// Read-only borrow, returns a new list / scalar
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

**Boundary semantics** (`src/std/list.yx:76-205`): indexing and slicing use `[]` directly, **out of
bounds is `E6003`**, neither returning a sentinel value nor clamping the boundary. `first` / `last`
on an empty list also report `E6003`. A **negative** index to `remove_at` reports `E6003`, but
**silently truncates when ≥ length** (only subtracting 1 from `length`).

**No** `insert` / `clear` / `sort`, and no in-place mutation methods — `pop` / `remove_at` take by
value and **return a new shortened list** (value semantics, the source list is consumed).

### 5.2 Dictionary (std.dict)

`std.dict` is a **module-level function collection operating on `Dict(K, V)`** (`src/std/dict.rs`),
**not** a record type. 11 exports:

```
new:     (K: Type, V: Type)() -> Dict(K, V)
get:     (K: Type, V: Type)(dict: &Dict(K, V), key: Any) -> Any    // missing key reports E6008
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

> **No** `insert` / `clear` (`Dict` is a core primitive, and the capacity strategy is not open). Use
> `dict.new()` for an empty dictionary — `{}` is an **empty block** (value `Void`), not an empty
> dictionary.

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
(`src/std/range.rs:29-90`), while `std.list` has its own `Iter` record. The **parameter shapes of
the two sides are opposite**: `range.has_next` has no `&` (consumes the iterator by value) while
`range.next` has `&`; `list.has_next` / `list.next` both have `&` / `&mut`. Use `for ... in` for
everyday iteration.

> **The `Iterator` protocol has no corresponding independent module** — there is no module path
> named `iterator` under `std` (this name has 0 hits in the entire `src/` repository).

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
// Usage (iterator protocol: std.range.iter/has_next/next, `for` is dispatched via static types)
for i in 0..10 {
    print(i)
}

// step form (double dot, no new keyword)
for i in 0..10..2 {
    print(i)
}
```

> **`Range(Int)` has officially landed** — the named fields `r.start`/`r.end`/`r.step` are
> accessible; at runtime, `x in r` goes through `std.range.contains` (boundary check + step
> alignment), proving that the pipeline is recognized as the range proposition
> `x >= r.start && x < r.end && (x - r.start) % r.step == 0` (range stays a range, not
> materialized). A step=0 literal is rejected at compile time; dynamic step=0 has been Result-ized:
> `std.range.iter` → `Result(Iterator, Error)` (code `E6009`), `std.range.contains` →
> `Result(Bool, Error)`; consumption points use `?` to propagate up the call stack or
> `result.unwrap` for explicit branching; `for`/`in` sugar desugaring unpacks at ir_gen, and the Err
> branch (dynamic step=0) explicitly fails (`abort_invalid_step`) — never silently infinite-looping.

---

## Appendix A: Standard Library Module Index

The table below only lists **actually existing** module paths (corresponding to the two tables in
[Standard Library Reference](../stdlib/index.md)).

| Module        | Implementation      | Description                                                                               |
| ------------- | ------------------- | ----------------------------------------------------------------------------------------- |
| `std.assert`  | native              | Runtime `assert` (+ two type families `IsTrue` / `Assert` not yet usable)                 |
| `std.option`  | yx                  | `Option(T)` optional value and sum type                                                   |
| `std.result`  | Dual implementation | `Result(T, E)` and `Error`; currently the only place `?` propagation is usable            |
| `std.list`    | yx                  | List operations on `Vec(T)` and the `Iter` iterator                                       |
| `std.dict`    | native              | `Dict(K, V)` read/write, key-value views and merging                                      |
| `std.string`  | native              | String search, splitting, formatting, parsing and code points                             |
| `std.math`    | native              | Integer family, float family, trigonometric functions, and `PI` / `E` / `TAU`             |
| `std.time`    | native              | Unix timestamp, UTC formatting and parsing, `datetime_*` accessors                        |
| `std.range`   | native              | `Range` iterator protocol, range predicates and lazy adapters                             |
| `std.json`    | yx                  | JSON parsing and serialization (RFC 8259)                                                 |
| `std.convert` | native              | Conversion of any value to `String`                                                       |
| `std.test`    | yx                  | Test assertion library (value semantics, RFC-036 §3), the first pure yx dogfooding module |

### A.2 Platform-Dependent Modules

| Module           | Implementation | Dependency                                          | wasm32                           |
| ---------------- | -------------- | --------------------------------------------------- | -------------------------------- |
| `std.io`         | native         | Standard I/O                                        | Only `read_line` is not exported |
| `std.fs`         | native         | File system                                         | Not exported                     |
| `std.os`         | native         | File/Process                                        | Not exported                     |
| `std.net`        | native         | Network (`ureq` + rustls TLS, synchronous blocking) | Not exported                     |
| `std.concurrent` | native         | Threads                                             | Not exported                     |
| `std.weak`       | native         | Atomic reference counting                           | Not exported                     |

### A.3 Removed Module Names

The following names were once module names of `std`, but have **0 hits in the entire `src/`
repository** and must not be used anymore (verification method: full-text search these identifiers
in the `src/` directory):

| Old Name     | Now Replaced By                                   |
| ------------ | ------------------------------------------------- |
| `collection` | `std.list` (`Vec(T)`) + `std.dict` (`Dict(K, V)`) |
| `array`      | Core primitive `Array(T, N)`                      |
| `iterator`   | Protocol surface of `std.range` / `std.list`      |
| `file`       | `std.fs`                                          |
| `dir`        | `std.fs`                                          |
| `math.trig`  | `std.math` (`sin` / `cos` / `tan`)                |
| `math.log`   | No logarithm function family                      |
| `random`     | None                                              |
| `regex`      | None                                              |
