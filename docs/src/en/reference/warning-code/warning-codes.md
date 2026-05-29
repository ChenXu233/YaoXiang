---
title: Warning Codes
description: Compiler warning codes and descriptions
---

# Warning Codes

This document lists warning codes that the YaoXiang compiler may produce. Warnings do not prevent compilation but may indicate potential issues in the code.

## Configuration

Warning behavior can be configured via `yaoxiang.toml`:

```toml
[lint]
# Dead code warning level: off | warn | deny
dead-code = "warn"
```

- `off`: Disable the warning
- `warn`: Display the warning (default)
- `deny`: Treat the warning as an error

## Warning List

### W1001: Unused Exported Function

**Reason**: An exported function is never called by any code.

**Example**:
```yaoxiang
pub fn dead_function() { }  // W1001: Unused exported function

fn main() {
    // dead_function is never called
}
```

**Suggestion**:
- If the function doesn't need to be used externally, remove the `pub` modifier
- If the function needs to be kept but is unused for now, set `dead-code = "off"` in the configuration

---

### W1002: Unused Exported Type

**Reason**: An exported type (type alias or custom type) is never used.

**Example**:
```yaoxiang
pub type DeadType = Int  // W1002: Unused exported type

fn main() {
    let x: Int = 42;
}
```

**Suggestion**:
- Remove the unnecessary `pub` modifier
- If the type needs to be exported but is unused for now, ignore this warning

---

### W1003: Unused Import

**Reason**: A module or symbol imported via `use` is never used.

**Example**:
```yaoxiang
use std.json  // W1003: Unused import

fn main() {
    // json module is never used
}
```

**Suggestion**:
- Remove unused imports to keep the code clean
- If you need to keep the import (for side effects), consider using `use std.json.*` or add a comment explaining why

---

### W1004: Unused Exported Variable

**Reason**: A variable exported with `pub let` is never read.

**Example**:
```yaoxiang
pub let dead_var = 42  // W1004: Unused exported variable

fn main() {
    // dead_var is never read
}
```

**Suggestion**:
- Remove the unnecessary `pub` modifier
- If the variable needs to be exported but is unused for now, ignore this warning

---

### W1005: Unused Exported Method

**Reason**: A method exported on a type is never called.

**Example**:
```yaoxiang
type Foo { value: Int }

pub fn Foo.dead_method(self) { }  // W1005: Unused exported method

fn main() {
    let foo = Foo { value: 1 };
    // dead_method is never called
}
```

**Suggestion**:
- Remove the unnecessary `pub` modifier
- If the method needs to be kept but is unused for now, ignore this warning

---

## Warning Level Details

| Level | Effect |
|------|--------|
| `off` | Completely disable this warning |
| `warn` | Display warning but continue compilation (default) |
| `deny` | Treat warning as an error, block compilation |

### Use Cases

- **During development**: Use `warn` level to be aware of potential issues in the code
- **Before release**: Use `deny` level to ensure no unused code remains
- **Legacy code**: Use `off` level to temporarily ignore warnings

---

## Difference from Error Codes

Warning codes use the `W` prefix (e.g., W1001), while error codes use the `E` prefix (e.g., E1001).

- **Error**: Blocks compilation and must be fixed
- **Warning**: Indicates potential issues and can be optionally fixed