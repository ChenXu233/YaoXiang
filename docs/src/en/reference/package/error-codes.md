---
title: 'Error Codes'
description: 'Package manager error codes and how to handle them'
---

# Error Codes

This document lists the errors that the YaoXiang package manager may return and how to handle them.

## Error List

### E0100: Project Already Exists

```
Error: Project already exists: <path>
```

**Cause**: The project directory you are trying to create already exists.

**How to handle**:

1. Choose a different project name
2. Delete or move the existing directory

---

### E0101: Not a Valid Project

```
Error: Not a YaoXiang project: yaoxiang.toml not found
```

**Cause**: The current directory or the specified directory does not contain a `yaoxiang.toml` file.

**How to handle**:

1. Make sure you run the command inside the project directory
2. Use `yx init` to create a new project

---

### E0102: Dependency Not Found

```
Error: Dependency not found: <name>
```

**Cause**: Trying to operate on a dependency that does not exist.

**How to handle**:

1. Check that the dependency name is spelled correctly
2. Use `yx list` to view existing dependencies

---

### E0103: Dependency Already Exists

```
Error: Dependency already exists: <name>
```

**Cause**: Trying to add a dependency that already exists.

**How to handle**:

1. If you need to update the version, first remove it with `yx rm`
2. Or use the existing dependency directly

---

### E0104: Invalid Manifest Format

```
Error: Invalid yaoxiang.toml format: <details>
```

**Cause**: The `yaoxiang.toml` file format is incorrect.

**How to handle**:

1. Check that the TOML syntax is correct
2. Make sure all required fields are present
3. Check for syntax errors (such as missing quotes, commas, etc.)

---

### E0105: IO Error

```
Error: IO error: <details>
```

**Cause**: A file read or write operation failed.

**Common causes**:

- Insufficient disk space
- Insufficient permissions
- The file is being used by another program

**How to handle**:

1. Check disk space
2. Check file permissions
3. Close other programs that may be using the file

---

### E0106: TOML Parse Error

```
Error: TOML parse error: <details>
```

**Cause**: The TOML file format is incorrect.

**How to handle**:

1. Validate the TOML syntax
2. Check that special characters are properly escaped

---

## FAQ

### Q: What to do when installing a dependency fails?

1. Check the network connection
2. Confirm the dependency name and version are correct
3. Try refreshing with `yx update`

### Q: What to do when encountering a version conflict?

Check `yaoxiang.toml` for incompatible dependency version requirements.

### Q: What to do when the vendor directory is corrupted?

Delete the `vendor` directory and rerun `yx install`.
