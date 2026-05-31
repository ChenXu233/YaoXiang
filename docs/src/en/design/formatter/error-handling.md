---
title: "Error Handling in Formatting"
description: "Specification of formatter behavior when encountering errors"
---

# Error Handling

---

## §E1 Formatting Errors

**§E1.1 Syntax Errors.** When source code contains syntax errors, the formatter should:

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