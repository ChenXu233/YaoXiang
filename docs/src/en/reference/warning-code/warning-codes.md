---
title: 'Warning Codes'
description: 'Compiler warning codes and explanations'
---

# Warning Codes

This document lists the warning codes that the YaoXiang compiler may emit. Warnings do not prevent
compilation, but may indicate potential issues in the code.

## Configuration

Warning behavior can be configured via `yaoxiang.toml`:

```toml
[lint]
# Dead code warning level: off | warn | deny
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

**Suggestions**:

- If the function does not need to be used externally, remove the `pub` modifier
- If the function needs to be kept but is currently unused, you can set `dead-code = "off"` in the
  configuration

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

**Suggestions**:

- If the type needs to be exported but is currently unused, ignore this warning

---

### W1003: Unused Import

**Reason**: A module or symbol imported by a `use` statement is never used.

```yaoxiang
use std.json  // W1003: Unused import

main = {
    // json module is never used
}
```

**Suggestions**:

- Remove unused imports to keep the code clean
- If the import needs to be kept (for side effects), consider using `use std.json.*` or add a
  comment explaining why

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

**Suggestions**:

- Remove unnecessary `pub` modifiers
- If the variable needs to be exported but is currently unused, ignore this warning

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

**Suggestions**:

- Remove unnecessary `pub` modifiers
- If the method needs to be kept but is currently unused, ignore this warning

---

### W1063: Const Generic Constraint Cannot Be Evaluated

**Reason**: The value constraint of a const generic parameter cannot be evaluated at compile-time.

**Message**: ``const generic constraint cannot be evaluated: `{constraint}` ({var} = {value})``

```yaoxiang
BadArray(Int, n)  // n is not a compile-time constant
```

**Suggestions**:

- Make sure const parameters are compile-time constants

---

### W1080: Compile-Time Proof Degradation

**Reason**: A constraint cannot be proven at compile-time and has been degraded to a runtime check.

**Message**: `Constraint cannot be proven at compile-time, has been degraded to a runtime check`

**Suggestions**:

- Consider adding a proof function to improve safety

---

## Warning Level Details

| Level  | Effect                                              |
| ------ | --------------------------------------------------- |
| `off`  | Completely disable this warning                     |
| `warn` | Show the warning but continue compiling (default)   |
| `deny` | Treat the warning as an error and block compilation |

### Use Cases

- **During development**: Use the `warn` level to be aware of potential issues in your code
- **Before release**: Use the `deny` level to ensure there is no unused code
- **Legacy code**: Use the `off` level to temporarily ignore warnings

---

## Difference from Error Codes

Warning codes use the `W` prefix (e.g., W1001), while error codes use the `E` prefix (e.g., E1001).

- **Error**: Blocks compilation and must be fixed
- **Warning**: Indicates a potential issue, and fixing it is optional
