# E0xxx：词法和语法分析

> 自动生成自 `src/util/diagnostic/codes/`

## 错误列表

## E0001：Invalid character

**类别**: Lexer

**消息**: Source contains illegal character

**帮助**: Remove the illegal character

---

## E0002：Invalid number literal

**类别**: Lexer

**消息**: Number literal format is incorrect

**帮助**: Check the format of the number literal

---

## E0003：Unterminated string

**类别**: Lexer

**消息**: Multi-line string is missing closing quote

**帮助**: Add the closing quote for the string

---

## E0004：Invalid character literal

**类别**: Lexer

**消息**: Character literal is incorrect

**帮助**: Character literals must contain exactly one character

---

## E0010：Expected token

**类别**: Parser

**消息**: Parser expected a specific token

**帮助**: Check the syntax and add the expected token

---

## E0011：Unexpected token

**类别**: Parser

**消息**: Encountered an unexpected token

**帮助**: Remove or replace the unexpected token

---

## E0012：Invalid syntax

**类别**: Parser

**消息**: Expression/statement has syntax error

**帮助**: Check the syntax of the expression or statement

---

## E0013：Mismatched brackets

**类别**: Parser

**消息**: Parentheses, brackets, or braces are mismatched

**帮助**: Ensure all brackets are properly closed

---

## E0014：Missing semicolon

**类别**: Parser

**消息**: Statement is missing a semicolon

**帮助**: Add a semicolon at the end of the statement

---

