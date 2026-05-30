# yaoxiang check

对 YaoXiang 源码进行静态检查（类型检查、所有权检查），不生成任何代码。

## 用法

```
yaoxiang check [OPTIONS] [PATH]...
```

## 参数

| 参数 | 说明 |
|------|------|
| `PATH` | 一个或多个文件或目录路径。不指定时检查当前项目。 |

## 选项

| 选项 | 说明 | 默认值 |
|------|------|--------|
| `--json` | 以 JSON 格式输出诊断信息 | 否 |
| `-w`, `--watch` | 监视文件变化并自动重新检查 | 否 |
| `--color <MODE>` | 颜色输出模式：`auto`、`always`、`never` | `auto` |
| `--exclude <PATH>` | 排除指定路径（可多次使用） | 无 |
| `--no-progress` | 抑制进度和摘要消息 | 否 |

## 退出码

| 退出码 | 说明 |
|--------|------|
| `0` | 无错误 |
| `1` | 检查发现错误 |
| `2` | 未找到 `.yx` 文件 |

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
yaoxiang check

# 检查指定文件
yaoxiang check src/main.yx

# 检查目录并输出 JSON
yaoxiang check src/ --json

# 监视模式
yaoxiang check --watch

# CI 模式（无颜色、无进度）
yaoxiang check --color never --no-progress

# 排除测试目录
yaoxiang check src/ --exclude tests/
```

## 与 CI 集成

```yaml
# GitHub Actions
- name: Type check
  run: yaoxiang check --color never --no-progress
```

## 另请参阅

- [`yaoxiang fmt`](./format-command.md) -- 代码格式化
- [错误码参考](./error-codes.md) -- 完整错误码列表
