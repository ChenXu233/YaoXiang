# File Batch Rename Tool

A file batch renaming utility written in YaoXiang.

## Current Status

**Note**: Due to some language limitations, this tool is currently in demo mode. It shows the interactive UI and reads directory contents, but cannot perform actual batch renaming.

## Running

```bash
cd docs/src/tutorial/examples/projects/batch_rename
cargo run -- run main.yx
```

## Features Demonstrated

- Interactive CLI interface
- Directory selection
- Operation menu (prefix/suffix)
- User confirmation
- Directory content display

## Language Features Used

- Module import (`use std.io`)
- Namespace access (`std.io.read_line()`, `std.os.read_dir()`)
- String comparison (`==`, `!=`)
- Conditional statements (`if/elif`)
- Function definitions

## Known Issues

See [LANG_ISSUES.md](LANG_ISSUES.md) for a list of known language issues.

## Output Example

```
===========================================
        File Batch Rename Tool
===========================================

Welcome to File Batch Rename Tool!

Enter target directory: .
Select: 1=prefix, 2=suffix, 0=exit:

Files:
.cargo
.claude
.git
...

Done
```
