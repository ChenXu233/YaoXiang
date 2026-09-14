# yx check

对 YaoXiang 源码进行静态检查（类型检查、所有权检查），不生成任何代码。

## 用法

```
yx check [OPTIONS] [PATH]...
```

## 参数

| 参数   | 说明                                             |
| ------ | ------------------------------------------------ |
| `PATH` | 一个或多个文件或目录路径。不指定时检查当前项目。 |

## 选项

| 选项               | 说明                                             | 默认值 |
| ------------------ | ------------------------------------------------ | ------ |
| `--json`           | 以 JSON 格式输出诊断信息                         | 否     |
| `--color <MODE>`   | 颜色输出模式：`auto`、`always`、`never`          | `auto` |
| `--exclude <PATH>` | 排除指定路径（可多次使用）                       | 无     |
| `--no-progress`    | 抑制进度和摘要消息                               | 否     |
| `--deny-warnings`  | 将警告视为错误：存在警告时以非零码退出           | 否     |

## 退出码

| 退出码 | 说明                                                  |
| ------ | ----------------------------------------------------- |
| `0`    | 无错误                                                |
| `1`    | 检查发现错误；或使用 `--deny-warnings` 时存在警告     |
| `2`    | 未找到 `.yx` 文件                                     |

## 跨文件分析

`yx check` 支持跨文件类型检查。当检查多个文件时：

1. 并行解析所有 `.yx` 文件
2. 构建模块依赖图
3. 检测循环依赖（报错）
4. 按拓扑排序顺序检查
5. 使用共享类型环境，正确检测跨文件引用

```bash
# 检查整个项目（自动检测跨文件引用）
yx check src/

# 检查指定文件
yx check src/main.yx src/lib.yx
```

## JSON 输出格式

使用 `--json` 时，输出格式为：

```json
{
  "error_count": 0,
  "warning_count": 0,
  "diagnostics": [
    {
      "file": "src/main.yx",
      "severity": "error",
      "code": "E1001",
      "message": "Unknown variable: 'x'",
      "line": 5,
      "column": 3,
      "end_line": 5,
      "end_column": 4,
      "lsp": { ... }
    }
  ]
}
```

## 示例

```bash
# 检查当前项目
yx check

# 检查指定文件
yx check src/main.yx

# 检查目录并输出 JSON
yx check src/ --json

# CI 模式（无颜色、无进度）
yx check --color never --no-progress

# CI 严格模式（警告也导致失败）
yx check --deny-warnings

# 排除测试目录
yx check src/ --exclude tests/
```

## 与 CI 集成

```yaml
# GitHub Actions
- name: Type check
  run: yx check --color never --no-progress
```

详细 CI 配置请参阅 [CI 集成指南](../guide/ci-integration.md)。

## 另请参阅

- [`yx format`](./format-command.md) -- 代码格式化
- [`yx test`](./test-command.md) -- 运行测试
- [错误码参考](./error-codes.md) -- 完整错误码列表
- [CI 集成指南](../guide/ci-integration.md) -- CI/CD 集成
- [诊断系统设计](../design/check/diagnostic-system.md) -- 架构设计文档
