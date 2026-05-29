# E2xxx: Semantic Analysis

> Auto-generated from `src/util/diagnostic/codes/`

## Error List

## E2001: Scope error

**Category**: Semantic

**Message**: Variable is not in current scope

**Help**: Check the variable's scope and ensure it's accessible here

---

## E2002: Duplicate definition

**Category**: Semantic

**Message**: Variable is defined multiple times in the same scope

**Help**: Rename or remove the duplicate definition

---

## E2003: Ownership error

**Category**: Semantic

**Message**: Ownership constraint is not satisfied

**Help**: Check the ownership semantics of the value

---

## E2010: Immutable assignment

**Category**: Semantic

**Message**: Attempting to modify an immutable variable

**Help**: Use 'mut' to declare a mutable variable

---

## E2011: Uninitialized use

**Category**: Semantic

**Message**: Using an uninitialized variable

**Help**: Initialize the variable before using it

---

## E2012: Mutability conflict

**Category**: Semantic

**Message**: Using mutable reference in immutable context

**Help**: Ensure the reference mutability matches the usage context