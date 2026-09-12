---
title: 'Error Codes'
description: Package manager error codes and handling methods
---

# Error Codes

This document lists the errors that the YaoXiang package manager may return and how to handle them.

## Error List

### E0100: Project already exists

```
Error: Project already exists: <path>
```

**Cause**: The project directory you are trying to create already exists.

**Handling**:

1. Choose a different project name
2. Delete or move the existing directory

---

### E0101: Not a valid project

```
Error: Not a YaoXiang project: yaoxiang.toml not found
```

**Cause**: The current directory or the specified directory does not contain a `yaoxiang.toml` file.

**Handling**:

1. Make sure you run the command inside the project directory
2. Use `yx init` to create a new project

---

### E0102: Dependency not found

```
Error: Dependency not found: <name>
```

**Cause**: Attempted to operate on a dependency that does not exist.

**Handling**:

1. Check whether the dependency name is spelled correctly
2. Use `yx list` to view existing dependencies

---

### E0103: Dependency already exists

```
Error: Dependency already exists: <name>
```

**Cause**: Attempted to add a dependency that already exists.

**Handling**:

1. If you need to update the version, first remove it with `yx rm`
2. Or use the existing dependency directly

---

### E0104: Invalid manifest format

```
Error: Invalid yaoxiang.toml format: <details>
```

**Cause**: The `yaoxiang.toml` file is not in the correct format.

**Handling**:

1. Check whether the TOML syntax is correct
2. Make sure all required fields are present
3. Check for syntax errors (such as missing quotes, commas, etc.)

---

### E0105: IO error

```
Error: IO error: <details>
```

**Cause**: A file read or write operation failed.

**Common causes**:

- Insufficient disk space
- Insufficient permissions
- The file is being used by another program

**Handling**:

1. Check the disk space
2. Check the file permissions
3. Close any other programs that may be occupying the file

---

### E0106: TOML parse error

```
Error: TOML parse error: <details>
```

**Cause**: The TOML file has a format error.

**Handling**:

1. Validate the TOML syntax
2. Check that special characters are properly escaped

---

## FAQ

### Q: What should I do if installing dependencies fails?

1. Check the network connection
2. Confirm the dependency name and version are correct
3. Try running `yx update` to refresh

### Q: What should I do when encountering version conflicts?

Check `yaoxiang.toml` for any incompatible dependency version requirements.

### Q: What should I do if the vendor directory is corrupted?

Delete the `vendor` directory and run `yx install` again.
