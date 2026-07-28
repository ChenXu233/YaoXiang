---
title: 'Formatting Error Handling'
description: 'Specifications for formatter behavior when encountering errors'
---

# Error Handling

---

## §E1 Formatting Errors

**§E1.1 Syntax Errors.** When the source code contains syntax errors, the formatter should:

1. Use the `parse()` function for parsing
2. If parsing has errors, return the error message directly
3. Do not insert any placeholders

**§E1.2 Configuration Errors.** When the configuration file format is incorrect, a clear error message should be returned.

---

## §E2 Exit Codes

| Exit Code | Meaning                              |
| --------- | ------------------------------------ |
| 0         | Success                              |
| 1         | `--check` mode found unformatted files |
| 2         | File not found or configuration error |

---

## §E3 Error Handling

**§E3.1 Error Reporting.** The formatter uses the `parse()` function to parse source code. If an error occurs during parsing, the formatter returns the error message directly without formatting.

**§E3.2 No Placeholders.** The formatter does not insert any placeholders at error locations (such as `/* error */`). When encountering syntax errors, the formatter reports the error directly and terminates.
