# E0xxx: Lexical and Syntax Analysis

> Automatically generated from `src/util/diagnostic/codes/`

## Error List

## E0001: Invalid character

**Category**: Lexer

**Message**: Source contains illegal character

**Help**: Remove the illegal character

---

## E0002: Invalid number literal

**Category**: Lexer

**Message**: Number literal format is incorrect

**Help**: Check the format of the number literal

---

## E0003: Unterminated string

**Category**: Lexer

**Message**: Multi-line string is missing closing quote

**Help**: Add the closing quote for the string

---

## E0004: Invalid character literal

**Category**: Lexer

**Message**: Character literal is incorrect

**Help**: Character literals must contain exactly one character

---

## E0010: Expected token

**Category**: Parser

**Message**: Parser expected a specific token

**Help**: Check the syntax and add the expected token

---

## E0011: Unexpected token

**Category**: Parser

**Message**: Encountered an unexpected token

**Help**: Remove or replace the unexpected token

---

## E0012: Invalid syntax

**Category**: Parser

**Message**: Expression/statement has syntax error

**Help**: Check the syntax of the expression or statement

---

## E0013: Mismatched brackets

**Category**: Parser

**Message**: Parentheses, brackets, or braces are mismatched

**Help**: Ensure all brackets are properly closed

---

## E0014: Missing semicolon

**Category**: Parser

**Message**: Statement is missing a semicolon

**Help**: Add a semicolon at the end of the statement

---