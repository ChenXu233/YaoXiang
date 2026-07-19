title: "yaoxiang format 命令行用法"
description: 格式化工具的命令行参数和使用方法
---

# 命令行用法

---

## A. 命令行用法

```bash
# 格式化文件（输出到 stdout）
yaoxiang format file.yx

# 检查文件是否已格式化
yaoxiang format --dry-run file.yx

# 格式化并写入文件
yaoxiang format -w file.yx

# 格式化目录下所有 .yx 文件
yaoxiang format -w src/
```

---

## B. CLI 参数

| 参数 | 说明 | 默认值 |
|------|------|--------|
| `--dry-run` | 检查模式，不修改文件 | false |
| `-w`, `--write` | 写入模式，修改文件 | false |
| `--stdout` | 输出到 stdout | false |
| `--indent-width` | 缩进宽度 | 4 |
| `--line-width` | 最大行宽 | 120 |
| `--use-tabs` | 使用 tab 缩进 | false |
| `--single-quote` | 使用单引号 | false |

---

## C. 参考资料

- [Issue #13: 实现 yaoxiang format 代码格式化工具](https://github.com/ChenXu233/YaoXiang/issues/13)
- [Rustfmt 风格指南](https://rust-lang.github.io/rustfmt/)
- [测试编写规范](../test-specification.md)
