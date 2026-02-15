---
title: Error Codes
description: Package Manager Error Codes and Handling
---

# Error Codes

This document lists errors that the YaoXiang Package Manager may return and how to handle them.

## Error List

### E0100: Project Already Exists

```
Error: Project already exists: <path>
```

**Cause**: The project directory you are trying to create already exists.

**Handling**:
1. Choose a different project name
2. Delete or move the existing directory

---

### E0101: Not a Valid Project

```
Error: Not a YaoXiang project: yaoxiang.toml not found
```

**Cause**: The current directory or specified directory does not contain a `yaoxiang.toml` file.

**Handling**:
1. Ensure you are inside the project directory when executing the command
2. Use `yaoxiang init` to create a new project

---

### E0102: Dependency Not Found

```
Error: Dependency not found: <name>
```

**Cause**: Trying to operate on a dependency that does not exist.

**Handling**:
1. Check if the dependency name is spelled correctly
2. Use `yaoxiang list` to view existing dependencies

---

### E0103: Dependency Already Exists

```
Error: Dependency already exists: <name>
```

**Cause**: Trying to add a dependency that already exists.

**Handling**:
1. If you need to update the version, first remove it with `yaoxiang rm`
2. Or simply use the existing dependency

---

### E0104: Invalid Manifest Format

```
Error: Invalid yaoxiang.toml format: <details>
```

**Cause**: The `yaoxiang.toml` file format is incorrect.

**Handling**:
1. Check if TOML syntax is correct
2. Ensure all required fields are present
3. Check for syntax errors (such as missing quotes, commas, etc.)

---

### E0105: IO Error

```
Error: IO error: <details>
```

**Cause**: File read/write operation failed.

**Common causes**:
- Insufficient disk space
- Insufficient permissions
- File is locked by another program

**Handling**:
1. Check disk space
2. Check file permissions
3. Close other programs that may be locking the file

---

### E0106: TOML Parse Error

```
Error: TOML parse error: <details>
```

**Cause**: TOML file format error.

**Handling**:
1. Validate TOML syntax
2. Check if special characters are properly escaped

---

## FAQ

### Q: What to do if dependency installation fails?

1. Check network connection
2. Verify dependency name and version are correct
3. Try using `yaoxiang update` to refresh

### Q: What to do if version conflicts occur?

Check if there are incompatible dependency version requirements in `yaoxiang.toml`.

### Q: What if the vendor directory is corrupted?

Delete the `vendor` directory and run `yaoxiang install` again.
