---
title: 'Data Structure Formatting Rules'
description: 'Formatting rules for literals, lists and dictionaries, and match expressions'
---

# Data Structure Formatting Rules

---

## §8 Literals

**§8.1 Integer literals.** Integer literals are output directly.

```
// ✅ Correct
let x = 42;
```

**§8.2 Float literals.** Float literals must include a decimal point.

```
// ✅ Correct
let x = 3.14;
let y = 42.0;  // Must have a decimal point

// ❌ Incorrect
let y = 42;    // Integer, not a float
```

**§8.3 String literals.** Double quotes are used by default. When `single_quote = true`, single
quotes are used.

```
// Default (double quotes)
let s = "hello";

// single_quote = true
let s = 'hello';
```

**§8.4 Boolean literals.** Boolean literals use lowercase.

```
// ✅ Correct
let x = true;
let y = false;

// ❌ Incorrect
let x = True;
let y = FALSE;
```

---

## §10 Lists and Dictionaries

**§10.1 List format.** Lists are enclosed in `[]`, with elements separated by commas.

```
// ✅ Correct
let x = [1, 2, 3];

// ❌ Incorrect
let x = [1,2,3];
```

**§10.2 Dictionary format.** Dictionaries are enclosed in `{}`, using the `key: value` format for
key-value pairs.

```
// ✅ Correct
let x = {"a": 1, "b": 2};

// ❌ Incorrect
let x = {"a":1, "b":2};
```

**§10.3 List comprehension.** List comprehensions use the `[expr for var in iterable]` format.

```
// ✅ Correct
let x = [i * 2 for i in range(10)];

// With condition
let x = [i for i in range(10) if i > 5];
```

---

## §11 Match Expressions

**§11.1 Match format.** A space separates the `match` keyword and the expression.

```
// ✅ Correct
match x { ... }

// ❌ Incorrect
match(x) { ... }
```

**§11.2 Pattern alignment.** Multiple patterns should be aligned, padded with spaces.

```
// ✅ Aligned
match x {
    1    => "one",
    2    => "two",
    100  => "hundred",
    _    => "other",
}
```

**§11.3 Pattern line break for long patterns.** When a pattern is too long, break the pattern onto a
new line, with `=>` aligned with the body.

```
// ✅ Line break
match x {
    VeryLongPatternName { field1, field2 }
        => handle_case(field1, field2),
    _ => default_case(),
}
```

---

## §11.4 Tuples

**§11.4.1 Tuple format.** Tuples are enclosed in `()`, with elements separated by commas.

```
// ✅ Correct
let t = (1, "hello", true);
let t = (1,);  // Single-element tuple

// ❌ Incorrect
let t = (1, "hello", true);  // Missing space after comma
let t = (1,"hello",true);  // Missing space after comma
```

**§11.4.2 Empty tuple.** An empty tuple is represented as `()`.

```
// ✅ Correct
let t = ();
```

---

## §11.5 Index Access

**§11.5.1 Index format.** Index access uses the `expr[index]` format.

```
// ✅ Correct
let x = arr[0];
let y = matrix[i][j];

// ❌ Incorrect
let x = arr [0];  // Extra spaces
let y = matrix[ i ][ j ];  // Extra spaces
```

---

## §11.6 Field Access

**§11.6.1 Field access format.** Field access uses the `expr.field` format.

```
// ✅ Correct
let x = obj.field;
let y = obj.method();

// ❌ Incorrect
let x = obj . field;  // Extra spaces
let y = obj. field;  // Extra spaces
```

**§11.6.2 Chained field access.** When chained field access exceeds the line width, place one method
call per line.

```
// When exceeding line width
let result = object.method1().method2().method3().method4();

// After formatting
let result = object
    .method1()
    .method2()
    .method3()
    .method4();
```
