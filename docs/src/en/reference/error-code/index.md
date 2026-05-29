# Error Code Reference

> Auto-generated from `src/util/diagnostic/codes/`

The YaoXiang compiler uses a unified error code system, where each error code includes:
- **Code**: Error identifier (e.g., `E1001`)
- **Category**: Error phase
- **Title**: Short error description
- **Message**: Detailed error message
- **Help**: Possible solutions

## Error Code List

| Prefix | Category | Description |
|--------|----------|-------------|
| E0xxx | Lexer/Parser | Lexical and syntax analysis errors |
| E1xxx | TypeCheck | Type checking errors |
| E2xxx | Semantic | Semantic analysis errors |
| E4xxx | Generic | Generics and trait errors |
| E5xxx | Module | Module and import errors |
| E6xxx | Runtime | Runtime errors |
| E7xxx | I/O | I/O and system errors |
| E8xxx | Internal | Internal compiler errors |

## Usage

### CLI Commands

Use the `yaoxiang explain` command to view error details:

```bash
# View error details
yaoxiang explain E1001

# JSON format output
yaoxiang explain E1001 --json
```

### In Code

```rust
use yaoxiang::util::diagnostic::{ErrorCodeDefinition, I18nRegistry};

// Find error codes and retrieve titles and help information via I18nRegistry
let i18n = I18nRegistry::default();

if let Some(code) = ErrorCodeDefinition::find("E1001") {
    let title = i18n.get_title(&code);
    println!("Title: {}", title);

    if let Some(help) = i18n.get_help(&code) {
        println!("Help: {}", help);
    }
}
```