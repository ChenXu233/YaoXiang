---
title: 'Warning Codes'
description: Compiler warning codes and descriptions
---

# Warning Codes

This document lists the warning codes that the YaoXiang compiler may produce. Warnings do not prevent compilation but may indicate potential issues in the code.

## Configuration

You can configure warning behavior through `yaoxiang.toml`:

```toml
[lint]
# dead-code warning level: off | warn | deny
dead-code = "warn"
```

- `off`: Disable the warning
- `warn`: Show the warning (default)
- `deny`: Treat the warning as an error

## Warning List

### W1001: Unused Exported Function

**Reason**: An exported function is never called by any code.

```yaoxiang
pub dead_function: () -> Void = { }  // W1001: Unused exported function

main = {
    // dead_function is never called
}
```

**Suggestion**:

- If the function doesn't need to be used externally, remove the `pub` modifier
- If the function needs to be kept but is temporarily unused, set `dead-code = "off"` in the configuration

---

### W1002: Unused Exported Type

**Reason**: An exported type (type alias or custom type) is never used.

**Example**:

```yaoxiang
DeadType: Type = Int  // W1002: Unused exported type

main = {
    x = 42
}
```

**Suggestion**:

- If the type needs to be exported but is temporarily unused, ignore this warning

---

### W1003: Unused Import

**Reason**: A module or symbol imported via `use` statement is never used.

```yaoxiang
use std.json  // W1003: Unused import

main = {
    // json module is never used
}
```

**Suggestion**:

- Remove unused imports to keep code clean
- If you need to keep the import (for side effects), consider using `use std.json.*` or add a comment explaining why

---

### W1004: Unused Exported Variable

**Reason**: A variable exported with `pub` is never read.

**Example**:

```yaoxiang
pub dead_var = 42  // W1004: Unused exported variable

main = {
    // dead_var is never read
}
```

**Suggestion**:

- Remove the unnecessary `pub` modifier
- If the variable needs to be exported but is temporarily unused, ignore this warning

---

### W1005: Unused Exported Method

**Reason**: A method exported on a type is never called.

**Example**:

```yaoxiang
Foo: Type = { value: Int }

pub Foo.dead_method: (self: Foo) -> Void = { }  // W1005: Unused exported method

main = {
    foo = Foo(1)
    // dead_method is never called
}
```

**Suggestion**:

- Remove the unnecessary `pub` modifier
- If the method needs to be kept but is temporarily unused, ignore this warning

---

## Warning Levels Explained

| Level   | Effect                           |
| ------- | -------------------------------- |
| `off`   | Completely disable this warning  |
| `warn`  | Show warning but continue compiling (default) |
| `deny`  | Treat warning as error, block compilation |

### Use Cases

- **During development**: Use `warn` level to be aware of potential issues in the code
- **Before release**: Use `deny` level to ensure there is no unused code
- **Legacy code**: Use `off` level to temporarily ignore warnings

---

## Difference from Error Codes

Warning codes use the `W` prefix (e.g., W1001), while error codes use the `E` prefix (e.g., E1001).

- **Error**: Blocks compilation and must be fixed
- **Warning**: Hints at potential issues and can be optionally fixed
