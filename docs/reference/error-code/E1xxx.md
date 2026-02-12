# E1xxx：类型检查

> 自动生成自 `src/util/diagnostic/codes/`

## 错误列表

## E1001：Unknown variable

**类别**: TypeCheck

**消息**: Referenced variable is not defined

**帮助**: Check if the variable name is spelled correctly, or define it first

---

## E1002：Type mismatch

**类别**: TypeCheck

**消息**: Expected type does not match actual type

**帮助**: Use the correct type or add a type conversion

---

## E1003：Unknown type

**类别**: TypeCheck

**消息**: Referenced type does not exist

**帮助**: Check if the type name is spelled correctly

---

## E1010：Parameter count mismatch

**类别**: TypeCheck

**消息**: Function call arguments do not match definition

**帮助**: Check the number of arguments in the function call

---

## E1011：Parameter type mismatch

**类别**: TypeCheck

**消息**: Parameter type check failed

**帮助**: Ensure the argument types match the function signature

---

## E1012：Return type mismatch

**类别**: TypeCheck

**消息**: Function return type is incorrect

**帮助**: Ensure the return value matches the declared return type

---

## E1013：Function not found

**类别**: TypeCheck

**消息**: Calling an undefined function

**帮助**: Check if the function name is spelled correctly

---

## E1020：Cannot infer type

**类别**: TypeCheck

**消息**: Context cannot infer the type

**帮助**: Add a type annotation or explicit type arguments

---

## E1021：Type inference conflict

**类别**: TypeCheck

**消息**: Multiple constraints lead to type contradiction

**帮助**: Check type annotations for consistency

---

## E1030：Pattern non-exhaustive

**类别**: TypeCheck

**消息**: Match expression does not cover all cases

**帮助**: Add missing patterns to the match expression

---

## E1031：Unreachable pattern

**类别**: TypeCheck

**消息**: Pattern that can never match

**帮助**: Remove or modify the unreachable pattern

---

## E1040：Operation not supported

**类别**: TypeCheck

**消息**: Type does not support the operation

**帮助**: Check the supported operations for this type

---

## E1041：Index out of bounds

**类别**: TypeCheck

**消息**: Array/list index is out of range

**帮助**: Ensure the index is within the bounds of the collection

---

## E1042：Field not found

**类别**: TypeCheck

**消息**: Accessing a non-existent struct field

**帮助**: Check the available fields of the struct

---

