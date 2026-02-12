# 错误码参考

> 自动生成自 `src/util/diagnostic/codes/`

YaoXiang 编译器使用统一的错误码系统，每个错误码包含：
- **代码**: 错误标识符 (如 `E1001`)
- **类别**: 错误所属阶段
- **标题**: 错误简短描述
- **消息**: 详细错误消息
- **帮助**: 可能的解决方案

## 错误码列表

| 前缀 | 类别 | 描述 |
|------|------|------|
| E0xxx | Lexer/Parser | 词法和语法分析错误 |
| E1xxx | TypeCheck | 类型检查错误 |
| E2xxx | Semantic | 语义分析错误 |
| E4xxx | Generic | 泛型与特质错误 |
| E5xxx | Module | 模块与导入错误 |
| E6xxx | Runtime | 运行时错误 |
| E7xxx | I/O | I/O与系统错误 |
| E8xxx | Internal | 内部编译器错误 |

## 使用说明

### CLI 命令

使用 `yaoxiang explain` 命令查看错误详情：

```bash
# 查看错误详情
yaoxiang explain E1001

# JSON 格式输出
yaoxiang explain E1001 --json
```

### 在代码中

```rust
use yaoxiang::util::diagnostic::ErrorCodeDefinition;

// 查找错误码
if let Some(code) = ErrorCodeDefinition::find("E1001") {
    println!("Title: {}", code.title);
    println!("Help: {:?}", code.help);
}
```
