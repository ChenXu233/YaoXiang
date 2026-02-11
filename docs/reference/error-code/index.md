# 错误码参考

> YaoXiang 编译器错误码完整参考，按分类组织。

## 快速导航

| 类别 | 范围 | 描述 |
|------|------|------|
| [词法与语法分析](./E0xxx.md) | E0xxx | 词法和解析阶段错误 |
| [类型检查](./E1xxx.md) | E1xxx | 类型系统相关错误 |
| [语义分析](./E2xxx.md) | E2xxx | 作用域、生命周期等 |
| [泛型与特质](./E4xxx.md) | E4xxx | 泛型约束、特质实现 |
| [模块与导入](./E5xxx.md) | E5xxx | 模块解析、导入导出 |
| [运行时错误](./E6xxx.md) | E6xxx | 执行期间错误 |
| [I/O 与系统错误](./E7xxx.md) | E7xxx | 文件、网络等系统错误 |
| [内部编译器错误](./E8xxx.md) | E8xxx | 编译器内部错误 |

## 错误消息格式

```
error[E0001]: Type mismatch: expected `Int`, found `String`
  --> file.yx:10:5
   |
10 | x: Int = "hello";
   |            ^^^^^^^^ expected `Int`, found `String`
   |
   = help: Consider using `.to_int()` method
```

## 使用 `yaoxiang explain`

```bash
# 查看错误详情
yaoxiang explain E0001

# 指定语言
yaoxiang explain E0001 --lang zh

# JSON 格式（供 IDE/LSP 使用）
yaoxiang explain E0001 --json
```

## 相关资源

- [错误系统设计文档](../../old/guides/error-system-design.md)
- [RFC-013: 错误代码规范](../../old/design/accepted/013-error-code-specification.md)
