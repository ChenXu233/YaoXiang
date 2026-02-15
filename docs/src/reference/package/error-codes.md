---
title: 错误码
description: 包管理器错误码及处理方式
---

# 错误码

本文档列出 YaoXiang 包管理器可能返回的错误及其处理方式。

## 错误列表

### E0100: 项目已存在

```
Error: Project already exists: <path>
```

**原因**：尝试创建的项目目录已存在。

**处理方式**：
1. 选择不同的项目名称
2. 删除或移动已存在的目录

---

### E0101: 不是有效项目

```
Error: Not a YaoXiang project: yaoxiang.toml not found
```

**原因**：当前目录或指定目录不包含 `yaoxiang.toml` 文件。

**处理方式**：
1. 确保在项目目录内执行命令
2. 使用 `yaoxiang init` 创建新项目

---

### E0102: 依赖不存在

```
Error: Dependency not found: <name>
```

**原因**：尝试操作一个不存在的依赖。

**处理方式**：
1. 检查依赖名称拼写是否正确
2. 使用 `yaoxiang list` 查看现有依赖

---

### E0103: 依赖已存在

```
Error: Dependency already exists: <name>
```

**原因**：尝试添加已存在的依赖。

**处理方式**：
1. 如果需要更新版本，先用 `yaoxiang rm` 移除
2. 或者直接使用现有依赖

---

### E0104: 无效的 manifest 格式

```
Error: Invalid yaoxiang.toml format: <details>
```

**原因**：`yaoxiang.toml` 文件格式不正确。

**处理方式**：
1. 检查 TOML 语法是否正确
2. 确保所有必需字段存在
3. 检查是否有语法错误（如缺少引号、逗号等）

---

### E0105: IO 错误

```
Error: IO error: <details>
```

**原因**：文件读写操作失败。

**常见原因**：
- 磁盘空间不足
- 权限不足
- 文件被其他程序占用

**处理方式**：
1. 检查磁盘空间
2. 检查文件权限
3. 关闭可能占用文件的其他程序

---

### E0106: TOML 解析错误

```
Error: TOML parse error: <details>
```

**原因**：TOML 文件格式错误。

**处理方式**：
1. 验证 TOML 语法
2. 检查特殊字符是否正确转义

---

## 常见问题

### Q: 安装依赖失败怎么办？

1. 检查网络连接
2. 确认依赖名称和版本正确
3. 尝试使用 `yaoxiang update` 刷新

### Q: 遇到版本冲突怎么办？

检查 `yaoxiang.toml` 中是否有不兼容的依赖版本要求。

### Q: vendor 目录损坏怎么办？

删除 `vendor` 目录后重新运行 `yaoxiang install`。
