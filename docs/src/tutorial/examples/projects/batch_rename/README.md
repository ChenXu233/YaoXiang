# File Batch Rename Tool

A file batch renaming utility written in YaoXiang.

## Usage

```bash
yx main.yx
```

## Configuration

Edit the `main` function in `main.yx` to configure:

```yaoxiang
main: () -> Void = {
    dir = "."        # Target directory
    op = "prefix"    # Operation: "prefix", "suffix", "replace"
    param = "NEW_"  # Parameter for the operation
    preview = true   # Set to false to actually rename files
    ...
}
```

## Operations

| Operation | Description | Example |
|-----------|-------------|---------|
| `prefix` | Add text at beginning | `photo.jpg` → `NEW_photo.jpg` |
| `suffix` | Add text before extension | `photo.jpg` → `photo_NEW.jpg` |
| `replace` | Replace text in filename | (requires implementation) |

## Running

```bash
# Preview mode (default, safe)
yx main.yx

# To actually rename files, set preview = false in main.yx
```

## Example Output

```
=== File Batch Rename Tool ===
Directory: .
Operation: prefix
Parameter: NEW_
Mode: PREVIEW (no changes will be made)

Files in directory:
.cargo
.claude
.git
...

Note: Full file iteration requires string parsing.

Summary:
  Operation: prefix
  Run with preview = false to apply changes.
```

## Language Features Demonstrated

- Function definition and calls
- String concatenation (`+`)
- Conditional statements (`if/elif`)
- Pattern matching (`match`)
- Module import (`use std.io`, `use std.os`)
- File system operations (`os.read_dir`, `os.rename`, `os.is_dir`)

## Notes

- The tool runs in preview mode by default for safety
- Set `preview = false` to actually rename files
- Current implementation shows files in directory; full iteration requires string parsing
