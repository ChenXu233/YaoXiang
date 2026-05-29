# FFI Extension Design Scheme

> **Status**: ✅ Completed (all 10 steps implemented)
>
> **Implementation Date**: 2025

## I. Background and Goals

### 1.1 Current State (Before Implementation)

Current FFI architecture:

```rust
type NativeHandler = fn(&[RuntimeValue]) -> Result<RuntimeValue, ExecutorError>;
```

**Problems**:

- Native functions cannot access heap, cannot return List/Dict
- Native functions cannot call user-passed YaoXiang functions (higher-order functions cannot be implemented)
- Hardcoded special handling scattered throughout the interpreter (len, dict_keys, etc.)

### 1.2 Goals

1. ✅ Allow native functions to access heap, return List/Dict
2. ✅ Allow native functions to call YaoXiang functions (support higher-order functions)
3. ✅ Unify architecture, eliminate interpreter hardcoding

---

## II. Overall Design

### 2.1 Core Type Definitions

```rust
// Execution context - passed to native functions
pub struct NativeContext<'a> {
    /// Heap memory management
    pub heap: &'a mut Heap,
    /// Callback: for calling YaoXiang functions (higher-order function scenarios)
    pub call_fn: Option<&'a mut dyn FnMut(&RuntimeValue, &[RuntimeValue]) -> Result<RuntimeValue, ExecutorError>>,
}

// Native function signature change
pub type NativeHandler = fn(args: &[RuntimeValue], ctx: &mut NativeContext<'_>) -> Result<RuntimeValue, ExecutorError>;
```

> **Implementation Note**: The final implementation uses a `call_fn` callback closure instead of directly holding an `Interpreter` reference,
> which avoids Rust's self-referential borrowing checker issues (since Interpreter owns both heap and ffi simultaneously).

### 2.2 Module Structure

```
src/backends/interpreter/
├── ffi.rs          # Modified: NativeHandler type, calling convention
└── executor.rs    # Modified: construct Context when calling native

src/std/
├── mod.rs         # Modified: NativeHandler type definition
├── io.rs          # Modified: all function signatures
├── math.rs        # Modified: all function signatures
├── string.rs      # Modified: implement heap access
├── list.rs        # Modified: implement heap access + higher-order functions
├── dict.rs        # Modified: implement heap access
└── ... other modules   # Modified: all function signatures
```

### 2.3 Calling Flow

```
User code calls native function
    ↓
BytecodeExecutor executes CallNative/CallStatic
    ↓
Get NativeHandler from FFIRegistry
    ↓
Construct NativeContext { heap, call_fn }
    ↓
Call handler(args, &mut ctx)
    ↓
Inside handler:
  - Access ctx.heap to allocate/modify List/Dict
  - Call ctx.call_function() to execute user functions
    ↓
Return RuntimeValue
```

---

## III. Detailed Implementation Steps

### Step 1: Modify FFI Type Definitions

**File**: `src/std/mod.rs`

**Changes**:

1. Add `NativeContext` struct definition
2. Modify `NativeHandler` type alias
3. Modify `NativeExport` struct (optional)

**Acceptance Criteria**:

- [x] `NativeContext` struct contains `heap` and `call_fn` fields
- [x] `NativeHandler` type is `fn(args: &[RuntimeValue], ctx: &mut NativeContext<'_>) -> Result<RuntimeValue, ExecutorError>`
- [x] Compilation passes

**Test Plan**:

- Compilation test: `cargo check` passes

---

### Step 2: Modify FFI Registry

**File**: `src/backends/interpreter/ffi.rs`

**Changes**:

1. Modify `register()` method signature
2. Modify `call()` method, pass ctx when calling

**Acceptance Criteria**:

- [x] `register(name, handler)` accepts handler with new signature
- [x] `call(name, args, ctx)` passes ctx to handler
- [x] Compilation passes

**Test Plan**:

- Compilation test: `cargo check` passes

---

### Step 3: Modify Interpreter Call Sites

**File**: `src/backends/interpreter/executor.rs`

**Changes**:

1. Find `CallNative` bytecode handling location (around line 600)
2. Construct `NativeContext` before calling native function
3. Pass ctx to `ffi.call()`

**Acceptance Criteria**:

- [x] Create NativeContext when calling native functions
- [x] NativeContext contains valid heap reference
- [x] NativeContext contains call_fn callback (for higher-order function scenarios)
- [x] Compilation passes

**Test Plan**:

- Compilation test: `cargo check` passes

---

### Step 4: Update std.io Module

**File**: `src/std/io.rs`

**Changes**:

1. Update all native function signatures
2. Add `ctx` parameter

**Functions Involved**:

- `native_print`
- `native_println`
- `native_read_line`
- `native_read_file`
- `native_write_file`
- `native_append_file`

**Acceptance Criteria**:

- [x] All function signatures match new `NativeHandler` type
- [x] Functions don't use ctx internally (backward compatible)
- [x] Compilation passes

**Test Plan**:

- [x] `std.io.print("test")` works normally
- [x] `std.io.println("test")` works normally

---

### Step 5: Update std.math Module

**File**: `src/std/math.rs`

**Changes**:

1. Update all native function signatures
2. Add `ctx` parameter

**Functions Involved**:

- `native_abs`, `native_max`, `native_min`, `native_clamp`
- `native_fabs`, `native_fmax`, `native_fmin`, `native_pow`
- `native_sqrt`, `native_floor`, `native_ceil`, `native_round`
- `native_sin`, `native_cos`, `native_tan`
- `native_pi`, `native_e`, `native_tau`

**Acceptance Criteria**:

- [x] All function signatures match new type
- [x] Compilation passes

**Test Plan**:

- [x] `std.math.abs(-5)` returns 5
- [x] `std.math.sqrt(4)` returns 2

---

### Step 6: Implement std.string Complete Functionality

**File**: `src/std/string.rs`

**Changes**:

1. Modify function signatures
2. Implement heap access, return real List

**Functions Involved**:

| Function | Implementation |
|----------|---------------|
| `split` | Use ctx.heap to allocate List |
| `chars` | Use ctx.heap to allocate List |
| `trim/upper/lower/replace` | Already implemented (no heap needed) |
| `contains/starts_with/ends_with` | Already implemented (no heap needed) |

**Acceptance Criteria**:

- [x] `std.string.split("a,b", ",")` returns `["a", "b"]`
- [x] `std.string.chars("abc")` returns `["a", "b", "c"]`
- [x] Compilation passes

**Test Plan**:

```yaoxiang
// Test split
let result = std.string.split("hello,world", ",");
assert(std.list.len(result) == 2);

// Test chars
let chars = std.string.chars("abc");
assert(std.list.len(chars) == 3);
```

---

### Step 7: Implement std.list Complete Functionality (Including Higher-Order Functions)

**File**: `src/std/list.rs`

**Changes**:

1. Modify all function signatures
2. Implement heap access
3. Implement higher-order function calls

**Functions Involved**:

| Function | Implementation |
|----------|---------------|
| `push` | Use ctx.heap to allocate new List |
| `pop` | Get elements from heap |
| `prepend` | Use ctx.heap to allocate new List |
| `reverse` | Use ctx.heap to allocate new List |
| `concat` | Use ctx.heap to allocate new List |
| `map` | **Call user function** |
| `filter` | **Call user function** |
| `reduce` | **Call user function** |
| `get/set/first/last/slice` | Heap access |

**Higher-Order Function Implementation Key Points**:

```rust
fn native_map(args: &[RuntimeValue], ctx: &mut NativeContext<'_>) -> Result<RuntimeValue, ExecutorError> {
    // args[0] is list, args[1] is user function
    let list_handle = /* extract from args[0] */;
    let func_value = /* extract from args[1] */;

    // Get list elements (clone to avoid borrow conflict)
    let items = match ctx.heap.get(list_handle) {
        Some(HeapValue::List(items)) => items.clone(),
        _ => return Err(...)
    };

    // Call user function for each element
    let mut result_items = Vec::new();
    for item in &items {
        let mapped = ctx.call_function(&func_value, &[item.clone()])?;
        result_items.push(mapped);
    }

    // Return new list
    let new_handle = ctx.heap.allocate(HeapValue::List(result_items));
    Ok(RuntimeValue::List(new_handle))
}
```

**Acceptance Criteria**:

- [x] `std.list.push([1, 2], 3)` returns `[1, 2, 3]`
- [x] `std.list.pop([1, 2, 3])` returns `3` and remaining `[1, 2]`
- [x] `std.list.map([1, 2], x => x * 2)` returns `[2, 4]`
- [x] `std.list.filter([1, 2, 3], x => x > 1)` returns `[2, 3]`
- [x] `std.list.reduce([1, 2, 3], (acc, x) => acc + x, 0)` returns `6`
- [x] Compilation passes

**Test Plan**:

```yaoxiang
// Test push
let list1 = std.list.push([1, 2], 3);
assert(std.list.len(list1) == 3);

// Test map
let doubled = std.list.map([1, 2, 3], x => x * 2);
assert(std.list.get(doubled, 0) == 2);

// Test filter
let filtered = std.list.filter([1, 2, 3, 4], x => x > 2);
assert(std.list.len(filtered) == 2);

// Test reduce
let sum = std.list.reduce([1, 2, 3], (acc, x) => acc + x, 0);
assert(sum == 6);
```

---

### Step 8: Implement std.dict Complete Functionality

**File**: `src/std/dict.rs`

**Changes**:

1. Modify all function signatures
2. Implement heap access
3. Support Any type keys

**Functions Involved**:

| Function | Implementation |
|----------|---------------|
| `get` | Get Dict from heap, look up key |
| `set` | Use ctx.heap to allocate new Dict |
| `has` | Get Dict from heap, check key |
| `keys/values/entries` | Use ctx.heap to allocate List |
| `delete` | Use ctx.heap to allocate new Dict |
| `merge` | Use ctx.heap to merge two Dicts |

**Acceptance Criteria**:

- [x] `std.dict.get({a: 1}, "a")` returns `1`
- [x] `std.dict.set({a: 1}, "b", 2)` returns `{a: 1, b: 2}`
- [x] `std.dict.keys({a: 1, b: 2})` returns `["a", "b"]`
- [x] `std.dict.has({a: 1}, "a")` returns `true`
- [x] Compilation passes

**Test Plan**:

```yaoxiang
// Test get
let d = {name: "tom", age: 20};
assert(std.dict.get(d, "name") == "tom");

// Test set
let d1 = {a: 1};
let d2 = std.dict.set(d1, "b", 2);
assert(std.dict.has(d2, "b") == true);

// Test keys
let keys = std.dict.keys({x: 1, y: 2});
assert(std.list.len(keys) == 2);
```

---

### Step 9: Update Other std Modules

**Files Involved**:

- `src/std/net.rs`
- `src/std/time.rs`
- `src/std/os.rs`
- `src/std/concurrent.rs`
- `src/std/weak.rs`
- `src/std/ffi.rs` (if test code exists)

**Changes**:

- Update all native function signatures, add ctx parameter
- Functions that don't need to use ctx can ignore it

**Acceptance Criteria**:

- [x] All std modules compile successfully
- [x] Existing functionality unaffected

---

### Step 10: Clean Up Interpreter Hardcoding

**File**: `src/backends/interpreter/executor.rs`

**Code to Remove**:

- `len()` special handling (around lines 609-634)
- `dict_keys()` special handling (around lines 637-666)

**Note**:

- ✅ Complete steps 6-8 first, ensure std library functions work normally
- Then use `std.list.len()` instead of built-in `len()`
- Use `std.dict.keys()` instead of built-in `dict_keys()`

> **Implementation Note**: In actual implementation, since the compiler IR generation phase produces bare names `"len"` and `"dict_keys"` calls,
> we additionally register generic `builtin_len` and `builtin_dict_keys` functions in `register_all()`,
> which handle length calculation for List/Tuple/Array/Dict/String/Bytes types and dictionary key extraction respectively.

**Acceptance Criteria**:

- [x] After removing len() hardcoding, `len([1,2,3])` still works (via builtin_len FFI registration)
- [x] After removing dict_keys() hardcoding, `dict_keys({a:1})` still works (via builtin_dict_keys FFI registration)
- [x] Compilation passes

---

## IV. Test Plan

### 4.1 Unit Tests

Add tests under `src/std/`:

```rust
#[cfg(test)]
mod tests {
    // string tests
    #[test]
    fn test_split() { ... }

    // list tests
    #[test]
    fn test_push() { ... }
    #[test]
    fn test_map() { ... }

    // dict tests
    #[test]
    fn test_get() { ... }
}
```

### 4.2 Integration Tests

Create test file `tests/std_primitives.yx`:

```yaoxiang
// String tests
let s1 = std.string.trim("  hello  ");
assert(s1 == "hello");

let s2 = std.string.split("a,b,c", ",");
assert(std.list.len(s2) == 3);

// List tests
let l1 = std.list.push([1, 2], 3);
assert(std.list.len(l1) == 3);

let doubled = std.list.map([1, 2, 3], x => x * 2);
assert(std.list.get(doubled, 0) == 2);

// Dict tests
let d = std.dict.set({a: 1}, "b", 2);
assert(std.dict.has(d, "b") == true);

// Higher-order function tests
let filtered = std.list.filter([1, 2, 3, 4, 5], x => x > 2);
assert(std.list.len(filtered) == 3);

let sum = std.list.reduce([1, 2, 3, 4], (acc, x) => acc + x, 0);
assert(sum == 10);
```

### 4.3 Regression Tests

Ensure existing functionality is unaffected:

```bash
# Run existing tests
cargo test

# Run integration tests
cargo run -- tests/std_primitives.yx
```

---

## V. Risks and Rollback

### 5.1 Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| Large scope of changes | May introduce bugs | Step-by-step, compile and test each step |
| Break existing native functions | Runtime errors | Update all std module signatures |
| Higher-order function calls complex | High implementation difficulty | Reference existing interpreter call logic |

### 5.2 Rollback Plan

If problems occur, use git to rollback:

```bash
git checkout -- src/std/ src/backends/interpreter/ffi.rs src/backends/interpreter/executor.rs
```

---

## VI. Time Estimation

| Step | Estimated Time |
|------|----------------|
| Steps 1-3 (FFI core) | 1-2 hours |
| Steps 4-5 (update io/math) | 30 minutes |
| Step 6 (string complete) | 30 minutes |
| Step 7 (list + higher-order functions) | 1-2 hours |
| Step 8 (dict) | 1 hour |
| Steps 9-10 (cleanup) | 30 minutes |
| **Total** | **5-6 hours** |

---

## VII. Summary

**Capabilities After Completion**:

```yaoxiang
// Strings
std.string.split("a,b,c", ",")  // ["a", "b", "c"]
std.string.chars("hi")          // ["h", "i"]

// Lists
std.list.push([1,2], 3)         // [1, 2, 3]
std.list.map([1,2], x => x*2)   // [2, 4]
std.list.filter([1,2,3], x => x>1)  // [2, 3]
std.list.reduce([1,2,3], (a,x)=>a+x, 0)  // 6

// Dicts
std.dict.get({a:1}, "a")       // 1
std.dict.keys({a:1, b:2})      // ["a", "b"]
```