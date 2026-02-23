# YaoXiang Language Support

VS Code 扩展，提供 YaoXiang 编程语言的基础支持。

## 功能

- **语法高亮**：关键字、字符串、数字、注释着色
- **语言配置**：自动缩进、括号匹配、注释快捷键
- **LSP 支持**：配合 YaoXiang LSP 服务器使用

## 安装

### 方式一：本地安装（开发中）

```bash
# 克隆仓库
git clone https://github.com/yaoxiang/yaoxiang.git

# 复制扩展到 VS Code 扩展目录
# Windows
copy vscode-extension %USERPROFILE%\.vscode\extensions\yaoxiang-language-support

# Linux/Mac
cp -r vscode-extension ~/.vscode/extensions/yaoxiang-language-support
```

### 方式二：VS Code 调试（开发中）

1. 在 VS Code 中打开项目
2. 按 `F5` 启动调试
3. 选择 "启动扩展开发主机"

## LSP 配置

### 前置条件

1. 安装 YaoXiang LSP 服务器：
   ```bash
   cargo install yaoxiang-lsp
   ```

2. 确保 `yaoxiang-lsp` 在系统 PATH 中

### 配置 LSP

在用户设置 (`settings.json`) 中添加：

```json
{
  "languageserver": {
    "yaoxiang": {
      "command": "yaoxiang-lsp",
      "filetypes": ["yaoxiang"]
    }
  }
}
```

或者使用 VS Code 扩展市场中的 LSP 客户端配置。

## 文件关联

扩展会自动关联 `.yx` 文件扩展名：

- `.yx` - YaoXiang 源文件

## 快捷键

| 功能 | 快捷键 |
|------|--------|
| 注释行 | `Ctrl+/` |
| 格式化代码 | `Shift+Alt+F` |

## 问题反馈

如果遇到问题，请提交 Issue：
https://github.com/yaoxiang/yaoxiang/issues

## 相关链接

- [YaoXiang 官方网站](https://yaoxiang.dev)
- [LSP 服务器实现](https://github.com/yaoxiang/yaoxiang/tree/main/src/lsp)
- [RFC-017: LSP 支持设计](https://github.com/yaoxiang/yaoxiang/blob/main/docs/src/design/rfc/accepted/017-lsp-support.md)
