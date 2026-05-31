# E1xxx: Type Check

> Auto-generated from `src/util/diagnostic/codes/`

## Error List

## E1001: Unknown variable

**Category**: TypeCheck

**Message**: Referenced variable is not defined

**Help**: Check if the variable name is spelled correctly, or define it first

---

## E1002: Type mismatch

**Category**: TypeCheck

**Message**: Expected type does not match actual type

**Help**: Use the correct type or add a type conversion

---

## E1003: Unknown type

**Category**: TypeCheck

**Message**: Referenced type does not exist

**Help**: Check if the type name is spelled correctly

---

## E1010: Parameter count mismatch

**Category**: TypeCheck

**Message**: Function call arguments do not match definition

**Help**: Check the number of arguments in the function call

---

## E1011: Parameter type mismatch

**Category**: TypeCheck

**Message**: Parameter type check failed

**Help**: Ensure the argument types match the function signature

---

## E1012: Return type mismatch

**Category**: TypeCheck

**Message**: Function return type is incorrect

**Help**: Ensure the return value matches the declared return type

---

## E1013: Function not found

**Category**: TypeCheck

**Message**: Calling an undefined function

**Help**: Check if the function name is spelled correctly

---

## E1020: Cannot infer type

**Category**: TypeCheck

**Message**: Context cannot infer the type

**Help**: Add a type annotation or explicit type arguments

---

## E1021: Type inference conflict

**Category**: TypeCheck

**Message**: Multiple constraints lead to type contradiction

**Help**: Check type annotations for consistency

---

## E1030: Pattern non-exhaustive

**Category**: TypeCheck

**Message**: Match expression does not cover all cases

**Help**: Add missing patterns to the match expression

---

## E1031: Unreachable pattern

**Category**: TypeCheck

**Message**: Pattern that can never match

**Help**: Remove or modify the unreachable pattern

---

## E1040: Operation not supported

**Category**: TypeCheck

**Message**: Type does not support the operation

**Help**: Check the supported operations for this type

---

## E1041: Index out of bounds

**Category**: TypeCheck

**Message**: Array/list index is out of range

**Help**: Ensure the index is within the bounds of the collection

---

## E1042: Field not found

**Category**: TypeCheck

**Message**: Accessing a non-existent struct field

**Help**: Check the available fields of the struct