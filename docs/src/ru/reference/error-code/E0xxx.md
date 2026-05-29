# E0xxx：词法和语法分析

> 自动生成自 `src/util/diagnostic/codes/`

## 错误列表

## E0001：Invalid character（无效字符）

**Категория**: Lexer

**Сообщение**: Source contains illegal character

**Помощь**: Remove the illegal character

---

## E0002：Invalid number literal（无效数字字面量）

**Категория**: Lexer

**Сообщение**: Number literal format is incorrect

**Помощь**: Check the format of the number literal

---

## E0003：Unterminated string（未终止的字符串）

**Категория**: Lexer

**Сообщение**: Multi-line string is missing closing quote

**Помощь**: Add the closing quote for the string

---

## E0004：Invalid character literal（无效字符字面量）

**Категория**: Lexer

**Сообщение**: Character literal is incorrect

**Помощь**: Character literals must contain exactly one character

---

## E0010：Expected token（期望的标记）

**Категория**: Parser

**Сообщение**: Parser expected a specific token

**Помощь**: Check the syntax and add the expected token

---

## E0011：Unexpected token（意外的标记）

**Категория**: Parser

**Сообщение**: Encountered an unexpected token

**Помощь**: Remove or replace the unexpected token

---

## E0012：Invalid syntax（无效语法）

**Категория**: Parser

**Сообщение**: Expression/statement has syntax error

**Помощь**: Check the syntax of the expression or statement

---

## E0013：Mismatched brackets（不匹配的括号）

**Категория**: Parser

**Сообщение**: Parentheses, brackets, or braces are mismatched

**Помощь**: Ensure all brackets are properly closed

---

## E0014：Missing semicolon（缺少分号）

**Категория**: Parser

**Сообщение**: Statement is missing a semicolon

**Помощь**: Add a semicolon at the end of the statement