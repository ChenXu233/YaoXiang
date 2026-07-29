---
title: 'Error Codes'
description: Package manager error codes and handling methods
---

# Error Codes

This document lists errors that YaoXiang package manager may return and their handling methods.

## Error List

### E0100: Project Already Exists

```
Error: Project already exists: <path>
```

**Cause**: The project directory you are trying to create already exists.

**Handling Methods**:

1. Choose a different project name
2. Delete or move the existing directory

---

### E0101: Not a Valid Project

```
Error: Not a YaoXiang project: yaoxiang.toml not found
```

**Cause**: The current directory or specified directory does not contain a `yaoxiang.toml` file.

**Handling Methods**:

1. Make sure you are executing commands within the project directory
2. Use `yaoxiang init` to create a new project

---

### E0102: Dependency Not Found

```
Error: Dependency not found: <name>
```

**Cause**: Attempting to operate on a dependency that does not exist.

**Handling Methods**:

1. Check if the dependency name is spelled correctly
2. Use `yaoxiang list` to view existing dependencies

---

### E0103: Dependency Already Exists

```
Error: Dependency already exists: <name>
```

**Cause**: Attempting to add a dependency that already exists.

**Handling Methods**:

1. If you need to update the version, first use `yaoxiang rm` to remove it
2. Or simply use the existing dependency

---

### E0104: Invalid Manifest Format

```
Error: Invalid yaoxiang.toml format: <details>
```

**Cause**: The `yaoxiang.toml` file format is incorrect.

**Handling Methods**:

1. Check if the TOML syntax is correct
2. Ensure all required fields are present
3. Check for syntax errors (such as missing quotes, commas, etc.)

---

### E0105: IO Error

```
Error: IO error: <details>
```

**Cause**: File read/write operation failed.

**Common Causes**:

- Insufficient disk space
- Insufficient permissions
- File is occupied by another program

**Handling Methods**:

1. Check disk space
2. Check file permissions
3. Close other programs that may be occupying the file

---

### E0106: TOML Parse Error

```
Error: TOML parse error: <details>
```

**Cause**: TOML file format error.

**Handling Methods**:

1. Validate TOML syntax
2. Check if special characters are properly escaped

---

## Frequently Asked Questions

### Q: What should I do if dependency installation fails?

1. Check network connection
2. Confirm the dependency name and version are correct
3. Try using `yaoxiang update` to refresh

### Q: What should I do if I encounter version conflicts?

Check if there are incompatible dependency version requirements in `yaoxiang.toml`.

### Q: What if the vendor directory is corrupted?

Delete the `vendor` directory and run `yaoxiang install` again.
