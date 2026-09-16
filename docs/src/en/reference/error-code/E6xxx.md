# E6xxx: Runtime Errors

> Auto-generated from `src/util/diagnostic/codes/`

## Error List

## E6001: Division by zero

**Category**: Runtime

**Message**: Attempted to divide by zero

**Help**: Add a check to prevent division by zero

---

## E6003: Array index out of bounds

**Category**: Runtime

**Message**: Array index is out of bounds at runtime

**Help**: Ensure the index is within the array bounds

---

## E6004: Stack overflow

**Category**: Runtime

**Message**: Recursion depth exceeded stack limit

**Help**: Reduce recursion depth or use iteration

---

## E6005: Assert failed

**Category**: Runtime

**Message**: Assertion failed at runtime

**Help**: Fix the assertion condition or provide valid input

---

## E6006: Function not found (runtime)

**Category**: Runtime

**Message**: Function not found: '{func}'

**Help**: Ensure the function is defined and spelled correctly

---

## E6007: Runtime error

**Category**: Runtime

**Message**: Runtime error: {message}

**Help**: See the error message for details

---

## E6008: Key not found

**Category**: Runtime

**Message**: Key not found

**Help**: Use dict.has to check key existence before indexing

---

> #299 §4: Dict missing key and index out-of-bounds (E6003) are semantically different categories —
> key not existing vs. index exceeding bounds. They are kept as separate codes to preserve
> diagnostic information. For safe access, use `dict.has` to check first, then retrieve.
