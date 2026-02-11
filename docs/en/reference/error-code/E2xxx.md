# E2xxx: Semantic Analysis

> Scope, lifetime, and other semantic-related errors.

## E2001: Scope error

Variable is not in current scope.

```yaoxiang
outer: () -> Int = {
    x = 10;
    inner: () -> Int = {
        return x;  # x is not in inner's scope
    };
    return inner();
}
```

```
error[E2001]: Scope error: x is not in scope
  --> example.yx:4:16
   |
 4 |         return x;
   |                ^ not defined in this scope
```

## E2002: Duplicate definition

Duplicate definition in the same scope.

```yaoxiang
x = 10;
x = 20;  # Duplicate definition of x
```

```
error[E2002]: Duplicate definition: x is already defined
  --> example.yx:2:5
   |
 2 | x = 20;
   |     ^ previously defined here
```

## E2003: Ownership error

Ownership constraint not satisfied (YaoXiang uses Move semantics, not borrowing).

```yaoxiang
dangling: () -> Int = {
    x = 10;
    return x;  # x's ownership is transferred on return
}
```

## E2010: Immutable assignment

Attempting to modify immutable variable.

```yaoxiang
x = 10;
x = 20;  # x is immutable
```

```
error[E2010]: Immutable assignment: cannot assign to immutable variable
  --> example.yx:2:3
   |
 2 | x = 20;
   | ^ variable `x` is immutable
```

**Fix**: Use `mut` to declare mutable variable.

## E2011: Uninitialized use

Using uninitialized variable.

```yaoxiang
x: Int;
print(x);  # x is not initialized
```

```
error[E2011]: Uninitialized use: x may be used here
  --> example.yx:2:8
   |
 2 | print(x);
   |        ^ variable `x` is not initialized
```

## E2012: Mutability conflict

Using mutable reference in immutable context.

```yaoxiang
mut x = 10;
ref y = x;  # ref creates shared reference
x = 20;  # y is a shared reference, cannot modify through it
```

```
error[E2012]: Mutability conflict
  --> example.yx:3:3
   |
 3 | x = 20;
   | ^ cannot mutate through shared reference
```

## Related

- [E1xxx: Type Checking](./E1xxx.md)
- [E4xxx: Generics & Traits](./E4xxx.md)
- [Error Code Index](./index.md)
