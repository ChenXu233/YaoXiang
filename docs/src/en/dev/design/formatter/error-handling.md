---
title: 'Formatting Error Handling'
description: 'Behavior specification for when the formatter encounters errors'
---

# Error Handling

---

## §E1 Formatting Errors

**§E1.1 Syntax Errors.** When the source code contains syntax errors, the formatter should:

1. Use the `parse()` function to parse
2. If parsing has errors, directly return the error information
3. Do not insert any placeholders

**§E1.2 Configuration Errors.** When the configuration file format is incorrect, it should return a
clear error message.

---

## §E2 Exit Codes

| Exit Code | Meaning                                |
| --------- | -------------------------------------- |
| 0         | Success                                |
| 1         | `--check` mode found unformatted files |
| 2         | File not found or configuration error  |

---

## §E3 Error Handling

**§E3.1 Error Reporting.** The formatter uses the `parse()` function to parse source code. If errors
occur during parsing, the formatter directly returns the error information without performing any
formatting.

**§E3.2 No Placeholders.** The formatter does not insert any placeholders (such as `/* error */`) at
error locations. When encountering syntax errors, the formatter directly reports the error and
terminates.
