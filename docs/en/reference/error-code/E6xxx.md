# E6xxx: Runtime Errors

> Errors occurring during execution.

## E6001: Division by zero

Integer division by zero.

```yaoxiang
main: () -> Void = {
    x = 10 / 0;
}
```

```
error[E6001]: Division by zero
  --> example.yx:2:17
   |
 2 |     x = 10 / 0;
   |                 ^ division by zero
```

## E6002: Assertion failed

assert! macro failed.

```yaoxiang
main: () -> Void = {
    x = 10;
    assert!(x > 100, "x must be greater than 100");
}
```

```
error[E6002]: Assertion failed: x must be greater than 100
  --> example.yx:3:5
   |
 3 |     assert!(x > 100, "x must be greater than 100");
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ assertion failed
```

## E6003: Arithmetic overflow

Arithmetic operation overflow.

```yaoxiang
main: () -> Void = {
    max_int = 9223372036854775807;  # Max Int value
    overflow = max_int + 1;  # Overflow
}
```

```
error[E6003]: Arithmetic overflow
  --> example.yx:3:21
   |
 3 |     overflow = max_int + 1;
   |                     ^^^^^^^^^^^ arithmetic overflow
```

## E6004: Stack overflow

Stack space exhausted (excessive recursion depth).

```yaoxiang
fibonacci: (n: Int) -> Int = {
    if n <= 1 {
        return n;
    }
    return fibonacci(n - 1) + fibonacci(n - 2);
}

main: () -> Void = {
    result = fibonacci(10000);
}
```

```
error[E6004]: Stack overflow: maximum recursion depth exceeded
  --> example.yx:7:22
   |
 7 |     result = fibonacci(10000);
   |                      ^^^^^^^^^^^^ recursion depth too large
```

## E6005: Heap allocation failed

Memory allocation failure.

```yaoxiang
main: () -> Void = {
    huge_list = List::new(1_000_000_000_000);
}
```

```
error[E6005]: Heap allocation failed
  --> example.yx:2:19
   |
 2 |     huge_list = List::new(1_000_000_000_000);
   |                   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ memory allocation failed
```

## E6006: Runtime index out of bounds

Index out of bounds at runtime.

```yaoxiang
main: () -> Void = {
    arr = [1, 2, 3];  # Array
    index = get_index();  # Dynamic index
    value = arr[index];
}
```

```
error[E6006]: Runtime index out of bounds
  --> example.yx:4:18
   |
 4 |     value = arr[index];
   |                  ^^^^^^^^^^ index out of bounds at runtime
```

## E6007: Type cast failed

Attempting to cast type to incompatible type.

```yaoxiang
main: () -> Void = {
    value: Any = 42;
    string_value = value as String;
}
```

```
error[E6007]: Type cast failed
  --> example.yx:3:22
   |
 3 |     string_value = value as String;
   |                      ^^^^^^^^^^^^^^^^ cannot cast `Int` to `String`
```

## Related

- [E5xxx: Modules & Imports](./E5xxx.md)
- [E7xxx: I/O & System Errors](./E7xxx.md)
- [Error Code Index](./index.md)
