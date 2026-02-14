# E5xxx: Modules & Imports

> Module resolution and import/export errors.

## E5001: Module not found

Imported module does not exist.

```yaoxiang
use "./nonexistent";  # Module does not exist
```

```
error[E5001]: Module not found: ./nonexistent
  --> example.yx:1:5
   |
 1 | use "./nonexistent";
   |     ^^^^^^^^^^^^^^^^^ file not found
```

## E5002: Cyclic import

Circular dependency between modules.

```yaoxiang
# a.yx
import "./b";

a_func: () -> Void = b.b_func();
```

```yaoxiang
# b.yx
import "./a";

b_func: () -> Void = a.a_func();
```

```
error[E5002]: Cyclic import detected
  --> a.yx:1:1
   |
 1 | import "./b";
   | ^^^^^^^^^^^ circular dependency: a -> b -> a
```

## E5003: Symbol not exported

Attempting to access non-exported symbol.

```yaoxiang
# my_module.yx
internal = 10;  # Not exported

# main.yx
import "./my_module";
x = my_module.internal;
```

```
error[E5003]: Symbol not exported: internal
  --> main.yx:3:18
   |
 3 | x = my_module.internal;
   |                  ^^^^^^^^ `internal` is not exported by `./my_module`
```

## E5004: Invalid module path

Module path format error.

```yaoxiang
import "invalid/path/../to/module";  # Path contains ..
```

```
error[E5004]: Invalid module path
  --> example.yx:1:8
   |
 1 | import "invalid/path/../to/module";
   |        ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ invalid path format
```

## E5005: Private access

Accessing private symbol.

```yaoxiang
# other.yx
private_value = 42;

# main.yx
import "./other";
x = other.private_value;
```

```
error[E5005]: Private access: private_value is private
  --> main.yx:3:18
   |
 3 | x = other.private_value;
   |                  ^^^^^^^^^^^ cannot access private symbol
```

## Related

- [E1xxx: Type Checking](./E1xxx.md)
- [E6xxx: Runtime Errors](./E6xxx.md)
- [Error Code Index](./index.md)
