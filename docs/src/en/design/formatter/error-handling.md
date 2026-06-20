---
title: "Formatter Error Handling"
description: Behavior specification for the formatter when encountering errors
---

# Error Handling

---

## §E1 Formatting Errors

**§E1.1 Syntax Errors.** When the source code contains syntax errors, the formatter should:

1. Use the `parse()` function to parse
2. If parsing produces errors, directly return the error message
3. Do not insert any placeholders

**§E1.2 Configuration Errors.** When the configuration file is incorrectly formatted, a clear error message should be returned.

---

## §E2 Exit Codes

| Exit Code | Meaning |
|-----------|---------|
| 0 | Success |
| 1 | `--check` mode discovered unformatted files |
| 2 | File not found or configuration error |

---

## §E3 Error Handling

**§E3.1 Error Reporting.** The formatter uses the `parse()` function to parse source code. If errors occur during parsing, the formatter directly returns the error message and does not perform formatting.

**§E3.2 No Placeholders.** The formatter does not insert any placeholders (such as `/* error */`) at the error location. When encountering syntax errors, the formatter directly reports the error and terminates.