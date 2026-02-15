# E1xxx: Type Checking

> Type system related errors.

## E1001: Unknown variable

Referenced variable is not defined.

```yaoxiang
x = 10;
print(y);  # y is not defined
```

```
error[E1001]: Unknown variable: y
  --> example.yx:2:8
   |
 2 | print(y);
   |        ^ not found in this scope
```

**Fix**: Check for typos or define the variable first.

## E1002: Type mismatch

Expected type does not match actual type.

```yaoxiang
x: Int = "hello";
```

```
error[E1002]: Type mismatch: expected `Int`, found `String`
  --> example.yx:1:12
   |
 1 | x: Int = "hello";
   |            ^^^^^^^^ expected `Int`, found `String`
```

**Fix**: Use the correct type or add type conversion.

## E1003: Unknown type

Referenced type does not exist.

```yaoxiang
x: MyType = 10;
```

```
error[E1003]: Unknown type: MyType
  --> example.yx:1:10
   |
 1 | x: MyType = 10;
   |          ^^^^^^ not defined
```

## E1010: Parameter count mismatch

Function call parameter count does not match definition.

```yaoxiang
add: (a: Int, b: Int) -> Int = a + b;
result = add(10);  # Only 1 argument passed
```

```
error[E1010]: Parameter count mismatch: expected 2, found 1
  --> example.yx:2:16
   |
 2 | result = add(10);
   |                ^^^ expected 2 arguments
```

## E1011: Parameter type mismatch

Parameter type check failed.

```yaoxiang
greet: (name: String) -> Void = print(name);
greet(123);  # Passed Int instead of String
```

```
error[E1011]: Parameter type mismatch: expected `String`, found `Int`
  --> example.yx:2:8
   |
 2 | greet(123);
   |        ^^^ expected `String`, found `Int`
```

## E1012: Return type mismatch

Function return value type error.

```yaoxiang
getNumber: () -> Int = "hello";
```

```
error[E1012]: Return type mismatch: expected `Int`, found `String`
  --> example.yx:1:22
   |
 1 | getNumber: () -> Int = "hello";
   |                      ^ expected `Int`, found `String`
```

## E1013: Function not found

Calling an undefined function.

```yaoxiang
result = my_function(10);
```

```
error[E1013]: Function not found: my_function
  --> example.yx:1:13
   |
 1 | result = my_function(10);
   |              ^^^^^^^^^^^ not defined
```

## E1020: Cannot infer type

Context cannot infer type.

```yaoxiang
identity: [T](x: T) -> T = x;  # Generic function needs type parameter
```

```
error[E1020]: Cannot infer type for parameter `x`
  --> example.yx:1:13
   |
 1 | identity(x) = x;
   |             ^ parameter must have type annotation
```

## E1021: Type inference conflict

Multiple constraints lead to type contradiction.

```yaoxiang
x = 10;
y: String = x;  # Int cannot convert to String
```

```
error[E1021]: Type inference conflict
  --> example.yx:2:14
   |
 2 | y: String = x;
   |              ^ cannot convert `Int` to `String`
```

## E1030: Pattern non-exhaustive

Match expression does not cover all cases.

```yaoxiang
result: Result[Int, String] = ok(42);
match result {
    ok(value) => print(value),
    # missing err arm
};
```

```
error[E1030]: Pattern non-exhaustive: missing `err` pattern
  --> example.yx:3:1
   |
 3 | match result {
   | ^ not all patterns are covered
```

## E1031: Unreachable pattern

Pattern that can never match.

```yaoxiang
x: Int = 10;
match x {
    0 => print("zero"),
    1 => print("one"),
    10 => print("ten"),  # This arm can never be reached
    _ => print("other"),
};
```

```
warning[E1031]: Unreachable pattern
  --> example.yx:4:5
   |
 4 |     10 => print("ten"),
   |     ^^ pattern can never match
```

## E1040: Operation not supported

Type does not support the operation.

```yaoxiang
a = "hello";
b = "world";
c = a * b;  # String does not support * operation
```

```
error[E1040]: Operation not supported: `String * String`
  --> example.yx:3:13
   |
 3 | c = a * b;
   |             ^ operator '*' is not defined for `String`
```

## E1041: Index out of bounds

Array/list index out of range.

```yaoxiang
arr = [1, 2, 3];
x = arr[10];  # Index exceeds array size
```

```
error[E1041]: Index out of bounds: index 10, size 3
  --> example.yx:2:13
   |
 2 | x = arr[10];
   |             ^^ index 10 out of bounds for array of size 3
```

## E1042: Field not found

Accessing non-existent struct field.

```yaoxiang
type Point = { x: Int, y: Int };
p = Point(10, 20);  # Constructor syntax
z = p.z;  # Point has no z field
```

```
error[E1042]: Field not found: `z` in `Point`
  --> example.yx:3:11
   |
 3 | z = p.z;
   |           ^ field `z` does not exist in `Point`
```

## Related

- [E0xxx: Lexer & Parser](./E0xxx.md)
- [E2xxx: Semantic Analysis](./E2xxx.md)
- [Error Code Index](./index.md)
