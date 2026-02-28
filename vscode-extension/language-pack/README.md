# YaoXiang Language Pack

VS Code 扩展，提供 YaoXiang 编程语言的基础支持。

## 功能

- **语法高亮**：关键字、字符串、数字、注释着色
- **语言配置**：自动缩进、括号匹配、注释快捷键

## 文件关联

扩展会自动关联 `.yx` 文件扩展名：

- `.yx` - YaoXiang 源文件

## 快捷键

| 功能 | 快捷键 |
|------|--------|
| 注释行 | `Ctrl+/` |
| 格式化代码 | `Shift+Alt+F` |

## LSP 服务器

此语言包需要配合 **YaoXiang Language Server** 子扩展使用，以启用完整的 LSP 功能：

- 代码补全
- 跳转定义
- 实时诊断
- 悬停提示
- 引用查找

安装 YaoXiang Extension Pack 后，LSP 服务器将自动启用（如果 `yaoxiang` 命令在 PATH 中）。

## 问题反馈

如果遇到问题，请提交 Issue：
https://github.com/yaoxiang-lang/yaoxiang/issues
