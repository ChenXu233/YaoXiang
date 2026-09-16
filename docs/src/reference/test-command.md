# yx test

运行 YaoXiang 测试文件。测试文件是普通的 `.yx` 源文件，通过 `std.test` 标准测试模块声明断言与期望。

## 用法

```
yx test [OPTIONS] [PATH]...
```

## 测试发现

不指定 `PATH` 时，按以下顺序确定测试范围：

1. `./yaoxiang.toml` 中 `[tool.test]` 的 `patterns` 配置
2. 未配置时，默认发现 `tests/**/*.yx`

指定 `PATH` 时只运行显式给出的路径（不读取 patterns 配置），之后仍应用配置中的 exclude 模式与 `--filter`。

每个测试文件按其声明的期望分为四类（schema 稳定，供 CI 消费）：

| 类别            | 判定方式                                                                 |
| --------------- | ------------------------------------------------------------------------ |
| `behavior`      | 运行文件，退出码为 0 即通过                                              |
| `compile-error` | `check` 非零退出且声明的错误码全部出现即通过（不运行文件）               |
| `runtime-error` | `check` 必须通过且运行必须失败，声明的错误码全部出现即通过               |
| `invalid`       | 文件声明不合法（如期望码与类别矛盾），不计入通过也不计入失败             |

期望声明文法见 [RFC-036](../design/rfc/accepted/036-test-framework.md)。

## 选项

| 选项              | 说明                                                     | 默认值 |
| ----------------- | -------------------------------------------------------- | ------ |
| `--filter <NAME>` | 只运行文件名包含该子串的测试文件                         | 无     |
| `--fail-fast`     | 首个失败的测试文件运行完成后停止                         | 否     |
| `-v`, `--verbose` | 显示每个测试文件捕获的 stdout/stderr                     | 否     |
| `--list`          | 仅列出发现的测试文件，不运行                             | 否     |
| `--no-progress`   | 抑制进度输出（标题与 PASS 行）；失败与摘要始终显示       | 否     |
| `--json`          | 输出 JSON 报告代替人类可读文本                           | 否     |
| `--parallel`      | 并行运行测试文件（每个 CPU 核一个 worker）               | 否     |

## 退出码

| 退出码 | 说明                           |
| ------ | ------------------------------ |
| `0`    | 全部通过（或未发现测试文件）   |
| `1`    | 存在失败，或运行出错           |

## JSON 输出格式

使用 `--json` 时，输出格式为：

```json
{
  "summary": {
    "total": 3,
    "passed": 2,
    "failed": 1,
    "skipped": 0,
    "by_kind": { "behavior": 2, "compile-error": 0, "runtime-error": 1, "invalid": 0 },
    "time_secs": 0.512
  },
  "files": [
    {
      "file": "tests/div_zero_err.yx",
      "kind": "runtime-error",
      "passed": false,
      "time_secs": 0.103,
      "exit_code": 1,
      "stderr": "..."
    }
  ]
}
```

- 失败文件附 `exit_code` 与 `stderr`（CI 取证）；`--verbose` 时全部文件附 `stdout`/`stderr`
- `files` 按路径排序，输出稳定
- `by_kind` 固定四键（`behavior` / `compile-error` / `runtime-error` / `invalid`），零计数也输出

## 示例

```bash
# 运行项目全部测试
yx test

# 运行指定目录
yx test tests/yaoxiang/

# 只运行文件名包含 parser 的测试文件
yx test --filter parser

# 首个失败后停止，并显示捕获输出
yx test --fail-fast -v

# 仅列出发现的测试文件
yx test --list

# CI 模式：并行、无进度
yx test --parallel --no-progress

# 输出 JSON 报告
yx test --json > report.json
```

## 与 CI 集成

```yaml
# GitHub Actions
- name: Test
  run: yx test --parallel --no-progress
```

详细 CI 配置请参阅 [CI 集成指南](../guide/ci-integration.md)。

## 另请参阅

- [`yx check`](./check-command.md) -- 静态检查
- [`yx format`](./format-command.md) -- 代码格式化
- [RFC-036: std.test 测试框架](../design/rfc/accepted/036-test-framework.md) -- 测试框架设计
- [CI 集成指南](../guide/ci-integration.md) -- CI/CD 集成
