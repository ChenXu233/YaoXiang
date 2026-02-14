# E2xxx：语义分析

> 自动生成自 `src/util/diagnostic/codes/`

## 错误列表

## E2001：Scope error

**类别**: Semantic

**消息**: Variable is not in current scope

**帮助**: Check the variable's scope and ensure it's accessible here

---

## E2002：Duplicate definition

**类别**: Semantic

**消息**: Variable is defined multiple times in the same scope

**帮助**: Rename or remove the duplicate definition

---

## E2003：Ownership error

**类别**: Semantic

**消息**: Ownership constraint is not satisfied

**帮助**: Check the ownership semantics of the value

---

## E2010：Immutable assignment

**类别**: Semantic

**消息**: Attempting to modify an immutable variable

**帮助**: Use 'mut' to declare a mutable variable

---

## E2011：Uninitialized use

**类别**: Semantic

**消息**: Using an uninitialized variable

**帮助**: Initialize the variable before using it

---

## E2012：Mutability conflict

**类别**: Semantic

**消息**: Using mutable reference in immutable context

**帮助**: Ensure the reference mutability matches the usage context

---

