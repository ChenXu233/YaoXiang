---
title: "Warning Codes"
description: Compiler warning codes and their explanations
---

# Warning Codes

This document lists the warning codes that the YaoXiang compiler may produce. Warnings do not prevent compilation, but may indicate potential issues in the code.

## Configuration

Warning behavior can be configured through `yaoxiang.toml`:

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

```yaoxiang
pub dead_function: () -> Void = { }  // W1001: unused exported function

main = {
    // dead_function is never called
}
```

**Suggestions**:
- If the function does not need to be used externally, remove the `pub` modifier
- If the function needs to be kept but is currently unused, you can set `dead-code = "off"` in the configuration

---

### W1002: Unused Exported Type

**Reason**: An exported type (type alias or custom type) is never used.

**Example**:
```yaoxiang
DeadType: Type = Int  // W1002: unused exported type

main = {
    x = 42
}
```

**Suggestions**:
- If the type needs to be exported but is currently unused, ignore this warning

---

### W1003: Unused Import

**Reason**: A module or symbol imported via a `use` statement is never used.

```yaoxiang
use std.json  // W1003: unused import

main = {
    // the json module is never used
}
```

**Suggestions**:
- Remove unused imports to keep the code clean
- If the import needs to be kept (for side effects), consider using `use std.json.*` or adding a comment explaining why

---

### W1004: Unused Exported Variable

**Reason**: A variable exported with `pub` is never read.

**Example**:
```yaoxiang
pub dead_var = 42  // W1004: unused exported variable

main = {
    // dead_var is never read
}
```

**Suggestions**:
- Remove unnecessary `pub` modifiers
- If the variable needs to be exported but is currently unused, ignore this warning

---

### W1005: Unused Exported Method

**Reason**: A method exported on a type is never called.

**Example**:
```yaoxiang
Foo: Type = { value: Int }

pub Foo.dead_method: (self: Foo) -> Void = { }  // W1005: unused exported method

main = {
    foo = Foo(1)
    // dead_method is never called
}
```

**Suggestions**:
- Remove unnecessary `pub` modifiers
- If the method needs to be kept but is currently unused, ignore this warning

---

## Warning Levels Explained

| Level | Effect |
|------|------|
| `off` | Completely disable this warning |
| `warn` | Display the warning but continue compilation (default) |
| `deny` | Treat the warning as an error and prevent compilation |

### Usage Scenarios

- **During development**: Use the `warn` level to learn about potential issues in the code
- **Before release**: Use the `deny` level to ensure there is no unused code
- **Legacy code**: Use the `off` level to temporarily ignore warnings

---

## Difference from Error Codes

Warning codes use the `W` prefix (e.g., W1001), while error codes use the `E` prefix (e.g., E1001).

- **Error**: Prevents compilation and must be fixed
- **Warning**: Indicates potential issues and is optional to fix