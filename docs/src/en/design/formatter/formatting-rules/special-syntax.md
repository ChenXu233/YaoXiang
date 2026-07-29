---
title: 'Special Syntax Rules'
description: 'Formatting rules for F-Strings, import statements, error handling, and unsafe blocks'
---

# Special Syntax Rules

---

## §13 F-String

**§13.1 F-String format.** F-String uses the `f"..."` format, with interpolation using `{expr}`.

```
// ✅ Correct
let msg = f"Hello, {name}!";
let msg = f"Result: {x + y}";
```

**§13.2 Format specification.** F-String supports format specifications `{expr:spec}`.

```
// ✅ Correct
let msg = f"{value:.2f}";
```

---

## §14 Import Statements

**§14.1 Import ordering.** When `sort_imports = true`, import statements are sorted in the following
order:

1. Standard library (`std`, `core`, `alloc`)
2. External crates
3. Relative paths (starting with `.` or `..`)

**§14.2 Within-group sorting.** Imports within the same group are sorted alphabetically.

```
// Before sorting
use z_crate;
use std::collections;
use a_crate;
use ./local;

// After sorting
use std::collections;
use a_crate;
use z_crate;
use ./local;
```

---

## §17 Error Handling

**§17.1 Try operator.** Use the `expr?` format.

```
// ✅ Correct
let x = foo()?;

// ❌ Incorrect
let x = foo() ?;
```

---

## §18 Unsafe Blocks

**§18.1 Unsafe format.** Use the `unsafe { ... }` format.

```
// ✅ Correct
let x = unsafe { dangerous_function() };

// ❌ Incorrect
let x = unsafe{ dangerous_function() };
```
