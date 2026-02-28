# YaoXiang Extension Pack

VS Code 扩展包，提供 YaoXiang 编程语言的完整支持。

## 包含内容

此扩展包包含两个子扩展：

1. **YaoXiang Language Pack** (`yaoxiang-language-pack`)
   - 语法高亮
   - 语言配置（自动缩进、括号匹配、注释快捷键）

2. **YaoXiang Language Server** (`yaoxiang-language-server`)
   - LSP 服务器集成
   - 代码补全
   - 跳转定义
   - 实时诊断
   - 悬停提示
   - 引用查找

## 前置条件

### 安装 YaoXiang

1. 确保已安装 Rust 工具链
2. 安装 YaoXiang 编译器：

```bash
cargo install yaoxiang
```

3. 确保 `yaoxiang` 命令在系统 PATH 中

## 安装

### 方式一：本地安装（开发中）

```bash
# 克隆仓库
git clone https://github.com/yaoxiang-lang/yaoxiang.git

# 复制扩展包到 VS Code 扩展目录
# Windows
copy vscode-extension\extension %USERPROFILE%\.vscode\extensions\yaoxiang-extension-pack

# Linux/Mac
cp -r vscode-extension/extension ~/.vscode/extensions/yaoxiang-extension-pack
```

### 方式二：VS Code 调试

1. 在 VS Code 中打开项目
2. 按 `F5` 启动调试
3. 选择 "启动扩展开发主机"

## 故障排除

### LSP 服务器未启动

如果 LSP 功能未自动启用，请检查：

1. `yaoxiang` 命令是否在 PATH 中：
   ```bash
   yaoxiang --version
   ```

2. 手动配置 LSP（可选）：
   在用户设置 (`settings.json`) 中添加：
   ```json
   {
     "languageserver": {
       "yaoxiang": {
         "command": "yaoxiang",
         "args": ["lsp"],
         "filetypes": ["yaoxiang"]
       }
     }
   }
   ```

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
https://github.com/yaoxiang-lang/yaoxiang/issues

## 相关链接

- [YaoXiang 官方网站](https://yaoxiang.dev)
- [LSP 服务器实现](https://github.com/yaoxiang-lang/yaoxiang/tree/main/src/lsp)
- [RFC-017: LSP 支持设计](https://github.com/yaoxiang-lang/yaoxiang/blob/main/docs/src/design/rfc/accepted/017-lsp-support.md)
