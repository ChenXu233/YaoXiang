# E7xxx: I/O & System Errors

> File, network, and other system operation errors.

## E7001: File not found

Attempting to read a non-existent file.

```yaoxiang
main: () -> Void = {
    content = read_file("nonexistent.txt");
}
```

```
error[E7001]: File not found: nonexistent.txt
  --> example.yx:2:26
   |
 2 |     content = read_file("nonexistent.txt");
   |                          ^^^^^^^^^^^^^^^^^^^ file does not exist
```

## E7002: Permission denied

Insufficient file permissions.

```yaoxiang
main: () -> Void = {
    content = read_file("/root/secret.txt");
}
```

```
error[E7002]: Permission denied: /root/secret.txt
  --> example.yx:2:26
   |
 2 |     content = read_file("/root/secret.txt");
   |                          ^^^^^^^^^^^^^^^^^^^^^^^ permission denied
```

## E7003: I/O error

Generic I/O error.

```yaoxiang
main: () -> Void = {
    content = read_file("/dev/full");
}
```

```
error[E7003]: I/O error: device full
  --> example.yx:2:26
   |
 2 |     content = read_file("/dev/full");
   |                          ^^^^^^^^^^^^^^^ I/O error: device full
```

## E7004: Network error

Network operation failed.

```yaoxiang
main: () -> Void = {
    response = HTTP.get("https://invalid.example.com");
}
```

```
error[E7004]: Network error: connection refused
  --> example.yx:2:31
   |
 2 |     response = HTTP.get("https://invalid.example.com");
   |                                   ^^^^^^^^^^^^^^^^^^^^^^^^ network error
```

## Related

- [E6xxx: Runtime Errors](./E6xxx.md)
- [E8xxx: Internal Compiler Errors](./E8xxx.md)
- [Error Code Index](./index.md)
