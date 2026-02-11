# E7xxx：I/O 与系统错误

> 文件、网络等系统操作相关错误。

## E7001：File not found

尝试读取不存在的文件。

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

## E7002：Permission denied

文件权限不足。

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

## E7003：I/O error

通用 I/O 错误。

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

## E7004：Network error

网络操作失败。

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

## 相关章节

- [E6xxx：运行时错误](./E6xxx.md)
- [E8xxx：内部编译器错误](./E8xxx.md)
- [错误码总索引](./index.md)
