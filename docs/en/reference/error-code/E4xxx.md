# E4xxx: Generics & Traits

> Generic parameters and trait constraint errors.

## E4001: Generic parameter mismatch

Generic parameter count or type mismatch.

```yaoxiang
identity: [T](x: T) -> T = x;
result = identity[Int, String](10);  # Too many generic parameters
```

```
error[E4001]: Generic parameter mismatch: expected 1, found 2
  --> example.yx:2:27
   |
 2 | result = identity[Int, String](10);
   |                            ^^^^^^^^ too many generic parameters
```

## E4002: Trait bound violated

Trait constraint not satisfied.

```yaoxiang
double: [T: Addable](x: T) -> T = x + x;
s = double("hello");  # String does not implement Addable
```

```
error[E4002]: Trait bound violated: String does not implement Addable
  --> example.yx:2:18
   |
 2 | s = double("hello");
   |                  ^^^^^ trait bound `Addable` not satisfied
```

## E4003: Associated type error

Associated type definition or usage error.

```yaoxiang
# Using type to define interface (RFC-010 syntax)
type Container = {
    type Item;  # Associated type
    get: () -> Item,
}
```

```
error[E4003]: Associated type error
  --> example.yx:3:5
   |
 3 |     type Item;
   |           ^^^ associated type definition error
```

## E4004: Duplicate trait implementation

Duplicate implementation of the same trait (RFC-010 uses interface composition syntax).

```yaoxiang
# Interface definition
type Printable = {
    print: () -> Void,
}

# Implementing interface (list interface name at end of type definition)
type IntPrinter = {
    value: Int,
    Printable,  # Implement Printable interface
}

type IntPrinter2 = {
    value: Int,
    Printable,  # Duplicate implementation
}
```

```
error[E4004]: Duplicate trait implementation: Printable is already implemented for Int
  --> example.yx:10:5
   |
10 |     Printable,  # Duplicate implementation
   |     ^^^^^^^^^^ conflicting implementation
```

## E4005: Trait not found

Required trait not found.

```yaoxiang
print_all: [T: MyPrintable](items: List[T]) -> Void = {
    for item in items {
        item.print();
    }
}
```

```
error[E4005]: Trait not found: MyPrintable
  --> example.yx:1:16
   |
 1 | print_all: [T: MyPrintable](items: List[T]) -> Void = {
   |                ^^^^^^^^^^^ trait not defined
```

## E4006: Sized bound violated

Sized constraint not satisfied.

```yaoxiang
store: [T](value: T) -> Void = {
    # T must be Sized
}
```

```
error[E4006]: Sized bound violated: T may not be sized
  --> example.yx:1:14
   |
 1 | store: [T](value: T) -> Void = {
   |              ^^^^^^^^^ T does not satisfy `Sized` bound
```

## Related

- [E1xxx: Type Checking](./E1xxx.md)
- [E5xxx: Modules & Imports](./E5xxx.md)
- [Error Code Index](./index.md)
