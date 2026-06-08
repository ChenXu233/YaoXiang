---
title: "Formatting Error Handling"
description: "Specifications for formatter behavior when encountering errors"
---

# Error Handling

---

## §E1 Formatting Errors

**§E1.1 Syntax Errors.** When the source code contains syntax errors, the formatter should:

1. Format the correct parts as much as possible
2. Preserve the original content of error nodes
3. Insert `/* error */` placeholders at error locations

**§E1.2 Configuration Errors.** When the configuration file is malformed, a clear error message should be returned.

---

## §E2 Exit Codes

| Exit Code | Meaning |
|-----------|---------|
| 0 | Success |
| 1 | `--check` mode found unformatted files |
| 2 | File not found or configuration error |

---

## §E3 Error Recovery

**§E3.1 Error Placeholders.** When the source code contains expressions that cannot be parsed, the formatter should insert a `/* error */` placeholder.

```
// Original code (has syntax error)
let x = ;

// After formatting
let x = /* error */;
```

**§E3.2 Error Tolerance.** The formatter should format the correct parts as much as possible, preserving the original content of error nodes.

```
// Original code
fn foo() {
    let x = 1;
    let y = ;  // syntax error
    let z = 3;
}

// After formatting
fn foo() {
    let x = 1;
    let y = /* error */;
    let z = 3;
}
```