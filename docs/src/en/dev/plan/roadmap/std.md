```markdown
---
title: "Standard Library Status"
---

# Standard Library (Std)

> **Module Status**: Has gaps (4 items to improve)
> **Location**: `src/std/`
> **Last Updated**: 2026-06-01

---

## Module Overview

The standard library provides core functional modules for the YaoXiang language. Includes IO, math, strings, lists, dictionaries, file system, network, concurrency, and other modules.

**Code Volume**: 5,071 lines (14 sub-modules)

---

## Feature List

### std.io (379 lines) - ✅ Complete

| Function | Signature | Status |
|----------|-----------|--------|
| `print` | `(...args) -> ()` | ✅ |
| `println` | `(...args) -> ()` | ✅ |
| `read_line` | `() -> String` | ✅ |
| `read_file` | `(path: String) -> String` | ✅ |
| `write_file` | `(path: String, content: String) -> Bool` | ✅ |
| `append_file` | `(path: String, content: String) -> Bool` | ✅ |
| `format_fallback` | `(value, type_name: String) -> String` | ✅ |

### std.math (301 lines) - ✅ Complete

| Function | Signature | Status |
|----------|-----------|--------|
| `abs` | `(n: Int) -> Int` | ✅ |
| `max/min` | `(a: Int, b: Int) -> Int` | ✅ |
| `clamp` | `(value: Int, min: Int, max: Int) -> Int` | ✅ |
| `fabs/fmax/fmin` | Float version | ✅ |
| `pow` | `(base: Float, exp: Float) -> Float` | ✅ |
| `sqrt` | `(n: Float) -> Float` | ✅ |
| `floor/ceil/round` | `(n: Float) -> Float` | ✅ |
| `sin/cos/tan` | `(n: Float) -> Float` | ✅ |
| `PI/E/TAU` | Constants | ✅ |

### std.string (523 lines) - ✅ Complete

| Function | Signature | Status |
|----------|-----------|--------|
| `split` | `(s: String, sep: String) -> List` | ✅ |
| `trim` | `(s: String) -> String` | ✅ |
| `upper/lower` | `(s: String) -> String` | ✅ |
| `replace` | `(s: String, old: String, new: String) -> String` | ✅ |
| `contains/starts_with/ends_with` | `(s: String, sub: String) -> Bool` | ✅ |
| `index_of` | `(s: String, sub: String) -> Int` | ✅ |
| `substring` | `(s: String, start: Int, end: Int) -> String` | ✅ |
| `is_empty/len` | `(s: String) -> Bool/Int` | ✅ |
| `chars` | `(s: String) -> List` | ✅ |
| `concat/repeat/reverse` | String operations | ✅ |
| `format` | `(format: String, ...args) -> String` | ✅ |

### std.list (784 lines) - ✅ Complete

| Function | Signature | Status |
|----------|-----------|--------|
| `push/pop/append/prepend` | List modification | ✅ |
| `remove_at` | `(list: List, index: Int) -> Any` | ✅ |
| `reverse/concat` | List operations | ✅ |
| `map/filter/reduce` | Higher-order functions | ✅ |
| `len/is_empty` | List information | ✅ |
| `get/set` | Index access | ✅ |
| `first/last` | Boundary elements | ✅ |
| `slice` | `(list: List, start: Int, end: Int) -> List` | ✅ |
| `contains/find_index` | Lookup | ✅ |
| `iter/next/has_next` | Iterator protocol | ✅ |

### std.dict (335 lines) - ✅ Complete

| Function | Signature | Status |
|----------|-----------|--------|
| `get/set` | Dict access | ✅ |
| `has` | `(dict: Dict, key: Any) -> Bool` | ✅ |
| `keys/values/entries` | Get collections | ✅ |
| `delete` | `(dict: Dict, key: Any) -> Dict` | ✅ |
| `len/is_empty` | Dict information | ✅ |
| `merge` | `(a: Dict, b: Dict) -> Dict` | ✅ |

### std.convert (149 lines) - ✅ Complete

- ✅ `to_string` — Universal type conversion to string
- ✅ Type-specific `to_string` methods: int, float, bool, char, string, list, dict, tuple, set, range

### std.os (1,023 lines) - ✅ Complete

- ✅ File operations: open, close, read, write, seek, tell, flush
- ✅ Directory operations: mkdir, rmdir, read_dir
- ✅ Path checks: remove, exists, is_file, is_dir
- ✅ File operations: copy, rename
- ✅ Environment variables: get_env, set_env
- ✅ Process information: args, chdir, getcwd

### std.time (507 lines) - ✅ Complete

- ✅ Time retrieval: now, timestamp, timestamp_ms
- ✅ `sleep` — `(seconds: Float) -> Void`
- ✅ Formatting: format_time, parse_time (strftime style)
- ✅ DateTime methods: year, month, day, hour, minute, second, weekday, to_string

### std.net (177 lines) - ⚠️ Stub Implementation

| Function | Signature | Status |
|----------|-----------|--------|
| `http_get` | `(url: String) -> String` | ⚠️ Stub - returns `"GET: {url}"` |
| `http_post` | `(url: String, body: String) -> String` | ⚠️ Stub - returns `"POST {url}: {body}"` |
| `url_encode` | `(s: String) -> String` | ✅ |
| `url_decode` | `(s: String) -> String` | ✅ |

### std.concurrent (85 lines) - ✅ Basic Completion

- ✅ `sleep` — `(millis: Int) -> Void`
- ✅ `thread_id` — `() -> String`
- ✅ `yield_now` — `() -> Void`

### std.ffi (265 lines) - ✅ Complete

- ✅ `native` — `(symbol: String) -> Never` (compile-time interception)

### std.weak (45 lines) - ⚠️ Basic Implementation

- ✅ `weak_new` — `(arc) -> Weak`
- ✅ `weak_upgrade` — `(weak) -> Option`
- ⚠️ Missing `StdModule` trait implementation, cannot be imported via `use std.weak`

### gen_interfaces (208 lines) - ✅ Complete

- ✅ Auto-generate `.yx` interface files
- ✅ Supports writing to directories, finding interface files

---

## Test Coverage

**Only 8 unit tests**, severely insufficient:

| Module | Unit Tests | Status |
|--------|------------|--------|
| io | 0 | ❌ Missing |
| math | 0 | ❌ Missing |
| string | 0 | ❌ Missing |
| list | 0 | ❌ Missing |
| dict | 0 | ❌ Missing |
| convert | 0 | ❌ Missing |
| os | 0 | ❌ Missing |
| time | 0 | ❌ Missing |
| net | 0 | ❌ Missing |
| concurrent | 0 | ❌ Missing |
| ffi | 2 | ✅ Basic coverage |
| gen_interfaces | 6 | ✅ Good coverage |

**Indirect test coverage**:
- `tests/yx_runner.rs` covers some functionality via E2E tests
- `tests/integration/execution.rs` has basic integration tests

---

## Issues Found

1. **net module is a stub implementation**: `http_get` and `http_post` return mock strings
2. **weak module is incomplete**: Missing `StdModule` trait implementation, cannot be imported via `use std.weak`
3. **os.chdir does not actually switch directories**: Only checks if directory exists, does not call `std::env::set_current_dir()`
4. **string.len returns byte count**: `native_len` uses `s.len()` which returns byte count instead of character count

---

## Code Quality Assessment

| Dimension | Score | Description |
|-----------|-------|-------------|
| Incomplete items | 4 | Add tests, fix bugs, weak module, HTTP stubs |
| Test coverage | Severely insufficient | Only 8 unit tests |
| Documentation quality | Good | Each module has module-level `//!` documentation comments |
| Code architecture | Good | Clear module division |

---

## Items to Improve

1. **Add unit tests for each module** (highest priority)
2. **Fix issues with `os.chdir` and `string.len`**
3. **Complete the `StdModule` implementation for the `weak` module**
4. **Implement real HTTP functionality or explicitly mark as stubs**
```