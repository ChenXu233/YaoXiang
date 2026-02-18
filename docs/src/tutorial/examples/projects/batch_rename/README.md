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
    param = "NEW_"   # Parameter for the operation
    preview = true   # Set to false to actually rename files
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

Processing files...

Preview (first 5 files):
  Example files after rename:
  prefix 'NEW_' would add prefix to each filename

Summary:
  Files found: 29

Run with preview = false to apply changes.
```

## Language Features Demonstrated

- Function definition and calls
- String concatenation (`+`)
- Conditional statements (`if/elif`)
- Pattern matching (`match`)
- Module import (`use std.io`, `use std.os`, `use std.string`, `use std.list`)
- File system operations (`os.read_dir`, `os.rename`, `os.is_dir`)
- String operations (`string.split`, `string.substring`, `string.index_of`, `string.replace`, `string.len`, `string.is_empty`)
- List operations (`list.len`, `list.get`)

## Known Issues

See [LANG_ISSUES.md](LANG_ISSUES.md) for a list of known language issues encountered during development.

## Notes

- The tool runs in preview mode by default for safety
- Set `preview = false` to actually rename files
- Full file iteration requires list handling improvements in the language
