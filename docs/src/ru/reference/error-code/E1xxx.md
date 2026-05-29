# E1xxx：Проверка типов

> Автоматически сгенерировано из `src/util/diagnostic/codes/`

## Список ошибок

## E1001：Unknown variable

**Категория**: TypeCheck

**Сообщение**: Referenced variable is not defined

**Помощь**: Check if the variable name is spelled correctly, or define it first

---

## E1002：Type mismatch

**Категория**: TypeCheck

**Сообщение**: Expected type does not match actual type

**Помощь**: Use the correct type or add a type conversion

---

## E1003：Unknown type

**Категория**: TypeCheck

**Сообщение**: Referenced type does not exist

**Помощь**: Check if the type name is spelled correctly

---

## E1010：Parameter count mismatch

**Категория**: TypeCheck

**Сообщение**: Function call arguments do not match definition

**Помощь**: Check the number of arguments in the function call

---

## E1011：Parameter type mismatch

**Категория**: TypeCheck

**Сообщение**: Parameter type check failed

**Помощь**: Ensure the argument types match the function signature

---

## E1012：Return type mismatch

**Категория**: TypeCheck

**Сообщение**: Function return type is incorrect

**Помощь**: Ensure the return value matches the declared return type

---

## E1013：Function not found

**Категория**: TypeCheck

**Сообщение**: Calling an undefined function

**Помощь**: Check if the function name is spelled correctly

---

## E1020：Cannot infer type

**Категория**: TypeCheck

**Сообщение**: Context cannot infer the type

**Помощь**: Add a type annotation or explicit type arguments

---

## E1021：Type inference conflict

**Категория**: TypeCheck

**Сообщение**: Multiple constraints lead to type contradiction

**Помощь**: Check type annotations for consistency

---

## E1030：Pattern non-exhaustive

**Категория**: TypeCheck

**Сообщение**: Match expression does not cover all cases

**Помощь**: Add missing patterns to the match expression

---

## E1031：Unreachable pattern

**Категория**: TypeCheck

**Сообщение**: Pattern that can never match

**Помощь**: Remove or modify the unreachable pattern

---

## E1040：Operation not supported

**Категория**: TypeCheck

**Сообщение**: Type does not support the operation

**Помощь**: Check the supported operations for this type

---

## E1041：Index out of bounds

**Категория**: TypeCheck

**Сообщение**: Array/list index is out of range

**Помощь**: Ensure the index is within the bounds of the collection

---

## E1042：Field not found

**Категория**: TypeCheck

**Сообщение**: Accessing a non-existent struct field

**Помощь**: Check the available fields of the struct

---