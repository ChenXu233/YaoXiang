# yx format

格式化 YaoXiang 源码，统一代码风格。

## 用法

```
yx format [OPTIONS] <PATH>
```

## 参数

| 参数    | 说明                       |
| ------- | -------------------------- |
| `PATH`  | 要格式化的源文件或目录     |

## 选项

| 选项               | 说明                                             | 默认值 |
| ------------------ | ------------------------------------------------ | ------ |
| `-n`, `--dry-run`  | 试运行：显示格式化差异，不写回                   | 否     |
| `-w`, `--write`    | 就地写回格式化结果                               | 否     |
| `--stdout`         | 输出到标准输出（未指定 `-n`/`-w` 时的默认行为）  | --     |
| `--no-verify`      | 跳过格式化后的校验（性能优化）                   | 否     |
| `--indent <N>`     | 覆盖缩进宽度                                     | 配置值 |
| `--line-width <N>` | 覆盖最大行宽                                     | 配置值 |
| `--use-tabs`       | 使用 Tab 缩进                                    | 配置值 |
| `--single-quote`   | 字符串使用单引号                                 | 配置值 |

## 配置优先级

格式化选项按以下优先级生效（高者优先）：

1. 命令行旗标
2. 项目配置 `yaoxiang.toml` 的格式化配置
3. 用户级配置文件

配置项与默认值详见[格式化配置设计](../design/formatter/configuration.md)。

## 退出码

| 退出码 | 说明                                              |
| ------ | ------------------------------------------------- |
| `0`    | 成功（含无需格式化的情况）                        |
| `2`    | `--dry-run` 下发现待格式化文件，或未找到 `.yx` 文件 |
| `1`    | 其他错误                                          |

## 示例

```bash
# 格式化并输出到标准输出（默认）
yx format src/main.yx

# 就地写回
yx format -w src/

# CI 检查：存在待格式化文件时以退出码 2 失败
yx format -n .

# 临时覆盖缩进与行宽
yx format --indent 4 --line-width 100 main.yx
```

## 与 CI 集成

```yaml
# GitHub Actions
- name: Format check
  run: yx format --dry-run .
```

退出码 `2` 表示存在待格式化文件。详细 CI 配置请参阅 [CI 集成指南](../guide/ci-integration.md)。

## 另请参阅

- [`yx check`](./check-command.md) -- 静态检查
- [`yx test`](./test-command.md) -- 运行测试
- [格式化规则总览](../design/formatter/formatting-rules/index.md) -- 各项格式化规则
- [CI 集成指南](../guide/ci-integration.md) -- CI/CD 集成
