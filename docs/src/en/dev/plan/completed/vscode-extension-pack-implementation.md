# VS Code Extension Pack Implementation Plan

> **Task**: Implement YaoXiang VS Code Extension Pack
> **Goal**: Auto-enable YaoXiang language support in VS Code
> **Date**: 2026-02-23
> **Status**: Pending
> **Prerequisite**: LSP server implementation complete

---

## Overview

This plan breaks down the VS Code extension pack implementation into 4 steps, including implementation goals, acceptance criteria, and test items.

### Relationship with LSP

```
┌─────────────────────────────────────────────────────┐
│              VS Code (Built-in LSP Client)         │
└──────────────────────┬──────────────────────────────┘
                       │ Communicates with LSP server via stdio
                       ▼
┌─────────────────────────────────────────────────────┐
│            YaoXiang LSP Server (Implemented)        │
│  - Code completion ✓                                │
│  - Go to definition ✓                              │
│  - Real-time diagnostics ✓                         │
└─────────────────────────────────────────────────────┘
                       ▲
                       │ Dependency
┌──────────────────────┴──────────────────────────────┐
│           VS Code Extension Pack (This Plan)        │
│  - Syntax highlighting                              │
│  - Language configuration                           │
│  - LSP auto-discovery configuration                 │
└─────────────────────────────────────────────────────┘
```

---

## Step 1: Create Extension Pack Project Structure

**Goal**:
- Create `vscode-extension/` directory in project root
- Establish standard VS Code extension pack structure

**Directory Structure**:
```
vscode-extension/
├── package.json                    # Extension configuration
├── language-configuration.json     # Language configuration
├── syntaxes/
│   └── yaoxiang.tmLanguage.json    # Syntax highlighting (optional)
└── README.md                       # Installation instructions
```

**Acceptance Criteria**:
- [ ] `vscode-extension/` directory created
- [ ] Directory structure conforms to VS Code extension pack standard

**Test Items**:
- [ ] Directory creation verification

---

## Step 2: Create package.json

**Goal**:
- Define YaoXiang language ID
- Associate file extension `.yx`
- Configure built-in LSP client support

**Core Configuration**:
```json
{
  "name": "yaoxiang",
  "displayName": "YaoXiang Language",
  "description": "YaoXiang programming language support",
  "languages": [{
    "id": "yaoxiang",
    "aliases": ["YaoXiang", "yx"],
    "extensions": [".yx"]
  }],
  "grammars": {
    "language": "yaoxiang",
    "scopeName": "source.yaoxiang"
  }
}
```

**Acceptance Criteria**:
- [ ] package.json contains correct language ID configuration
- [ ] File extension `.yx` is associated
- [ ] Language display name is "YaoXiang"

**Test Items**:
- [ ] package.json syntax validation
- [ ] Configuration completeness check

---

## Step 3: Create language-configuration.json

**Goal**:
- Configure line comment format `//`
- Configure block comment format `/* */`
- Configure bracket matching rules
- Configure auto-indentation rules

**Core Configuration**:
```json
{
  "comments": {
    "lineComment": "//",
    "blockComment": ["/*", "*/"]
  },
  "brackets": [
    ["{", "}"],
    ["[", "]"],
    ["(", ")"]
  ],
  "indentationRules": {
    "increaseIndentPattern": "^.*\\{[^}]*$",
    "decreaseIndentPattern": "^\\s*\\}"
  }
}
```

**Acceptance Criteria**:
- [ ] Line comments use `//`
- [ ] Block comments use `/* */`
- [ ] Bracket matching works correctly

**Test Items**:
- [ ] Open .yx file in VS Code, verify comment shortcuts (Ctrl+/) work
- [ ] Verify bracket matching highlight

---

## Step 4: (Optional) Create Syntax Highlighting

**Goal**:
- Create TextMate syntax definition based on YaoXiang keywords
- Support keyword, string, number, and comment coloring

**YaoXiang Keyword List**:
- Control flow: `if`, `elif`, `else`, `match`, `while`, `for`, `in`, `return`, `break`, `continue`
- Declaration: `pub`, `use`, `spawn`, `ref`, `mut`
- Type: `as`, `unsafe`

**TextMate Syntax Structure**:
```json
{
  "name": "YaoXiang",
  "patterns": [
    {
      "include": "#keywords"
    },
    {
      "include": "#strings"
    },
    {
      "include": "#numbers"
    },
    {
      "include": "#comments"
    }
  ]
}
```

**Acceptance Criteria**:
- [ ] Keywords colored correctly
- [ ] Strings colored correctly
- [ ] Numbers colored correctly
- [ ] Comments colored correctly

**Test Items**:
- [ ] Open .yx file, verify syntax highlighting effect
- [ ] Check coloring correctness for various token types

---

## Step 5: Create README.md

**Goal**:
- Provide extension pack installation instructions
- Explain LSP server configuration method

**Acceptance Criteria**:
- [ ] README contains installation steps
- [ ] README contains LSP configuration instructions

---

## Acceptance Criteria Summary

| Step | Acceptance Item | Status |
|------|-----------------|--------|
| 1 | Directory structure created | ⬜ |
| 2 | package.json configuration | ⬜ |
| 3 | language-configuration.json | ⬜ |
| 4 | Syntax highlighting (optional) | ⬜ |
| 5 | README.md | ⬜ |

---

## Test Items Summary

### Manual Testing

1. **Step 2**: Validate package.json syntax
2. **Step 3**:
   - Open .yx file, press Ctrl+/ to verify commenting works
   - Type brackets to verify matching highlight
3. **Step 4**: Verify coloring for keywords, strings, numbers, and comments
4. **Step 5**: Verify documentation readability

---

## Future Extensions

When LSP server implementation is complete, the following features can be expanded:

1. **Auto LSP Discovery**: Extension pack automatically detects if `yaoxiang-lsp` is in PATH
2. **Status Bar Integration**: Display LSP connection status
3. **Debug Integration**: DAP-based debugging entry point
4. **Project Templates**: One-click YaoXiang project creation

---

## References

- [VS Code Extension Guidelines](https://code.visualstudio.com/api)
- [Language Extension Overview](https://code.visualstudio.com/api/language-extensions)
- [Syntax Highlight Guide](https://code.visualstudio.com/api/language-extensions/syntax-highlight-guide)