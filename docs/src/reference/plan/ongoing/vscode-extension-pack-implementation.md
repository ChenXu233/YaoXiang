# VS Code 扩展包实现计划

> **任务**：实现 YaoXiang VS Code 扩展包（Extension Pack）
> **目标**：在 VS Code 中自动启用 YaoXiang 语言支持
> **日期**：2026-02-23
> **状态**：待开始
> **前置依赖**：LSP 服务器实现完成

---

## 概述

本计划将 VS Code 扩展包实现分解为 4 个步骤，包含实现目标、验收标准和测试项目。

### 与 LSP 的关系

```
┌─────────────────────────────────────────────────────┐
│              VS Code (内置 LSP 客户端)              │
└──────────────────────┬──────────────────────────────┘
                       │ 通过 stdio 与 LSP 服务器通信
                       ▼
┌─────────────────────────────────────────────────────┐
│            YaoXiang LSP Server (已实现)             │
│  - 代码补全 ✓                                        │
│  - 跳转定义 ✓                                        │
│  - 实时诊断 ✓                                        │
└─────────────────────────────────────────────────────┘
                       ▲
                       │ 依赖
┌──────────────────────┴──────────────────────────────┐
│           VS Code 扩展包（本计划实现）               │
│  - 语法高亮                                          │
│  - 语言配置                                          │
│  - LSP 自动发现配置                                  │
└─────────────────────────────────────────────────────┘
```

---

## 步骤 1：创建扩展包项目结构

**目标**：
- 在项目根目录创建 `vscode-extension/` 目录
- 建立标准 VS Code 扩展包结构

**目录结构**：
```
vscode-extension/
├── package.json                    # 扩展配置
├── language-configuration.json     # 语言配置
├── syntaxes/
│   └── yaoxiang.tmLanguage.json    # 语法高亮（可选）
└── README.md                       # 安装说明
```

**验收标准**：
- [ ] `vscode-extension/` 目录已创建
- [ ] 目录结构符合 VS Code 扩展包规范

**测试项目**：
- [ ] 目录创建验证

---

## 步骤 2：创建 package.json

**目标**：
- 定义 YaoXiang 语言 ID
- 关联文件扩展名 `.yx`
- 配置内置 LSP 客户端支持

**核心配置**：
```json
{
  "name": "yaoxiang",
  "displayName": "YaoXiang Language",
  "description": "YaoXiang programming language support",
  "languages": [{
    "id": "yaoxiang",
    "aliases": ["YaoXiang", "yx"],
    "extensions": [".yx"]
  }],
  "grammars": {
    "language": "yaoxiang",
    "scopeName": "source.yaoxiang"
  }
}
```

**验收标准**：
- [ ] package.json 包含正确的语言 ID 配置
- [ ] 文件扩展名 `.yx` 已关联
- [ ] 语言显示名称为 "YaoXiang"

**测试项目**：
- [ ] package.json 语法验证
- [ ] 配置完整性检查

---

## 步骤 3：创建 language-configuration.json

**目标**：
- 配置行注释格式 `//`
- 配置块注释格式 `/* */`
- 配置括号匹配规则
- 配置自动缩进规则

**核心配置**：
```json
{
  "comments": {
    "lineComment": "//",
    "blockComment": ["/*", "*/"]
  },
  "brackets": [
    ["{", "}"],
    ["[", "]"],
    ["(", ")"]
  ],
  "indentationRules": {
    "increaseIndentPattern": "^.*\\{[^}]*$",
    "decreaseIndentPattern": "^\\s*\\}"
  }
}
```

**验收标准**：
- [ ] 行注释使用 `//`
- [ ] 块注释使用 `/* */`
- [ ] 括号匹配正常工作

**测试项目**：
- [ ] 在 VS Code 中打开 .yx 文件，验证注释快捷键（Ctrl+/）生效
- [ ] 验证括号匹配高亮

---

## 步骤 4：（可选）创建语法高亮

**目标**：
- 基于 YaoXiang 关键字创建 TextMate 语法定义
- 支持关键字、字符串、数字、注释着色

**YaoXiang 关键字列表**：
- 控制流：`if`, `elif`, `else`, `match`, `while`, `for`, `in`, `return`, `break`, `continue`
- 声明：`pub`, `use`, `spawn`, `ref`, `mut`
- 类型：`as`, `unsafe`

**TextMate 语法结构**：
```json
{
  "name": "YaoXiang",
  "patterns": [
    {
      "include": "#keywords"
    },
    {
      "include": "#strings"
    },
    {
      "include": "#numbers"
    },
    {
      "include": "#comments"
    }
  ]
}
```

**验收标准**：
- [ ] 关键字正确着色
- [ ] 字符串正确着色
- [ ] 数字正确着色
- [ ] 注释正确着色

**测试项目**：
- [ ] 打开 .yx 文件，验证语法高亮效果
- [ ] 检查各种 token 类型的着色是否正确

---

## 步骤 5：创建 README.md

**目标**：
- 提供扩展包安装说明
- 说明 LSP 服务器配置方法

**验收标准**：
- [ ] README 包含安装步骤
- [ ] README 包含 LSP 配置说明

---

## 验收标准汇总

| 步骤 | 验收项 | 状态 |
|------|--------|------|
| 1 | 目录结构创建 | ⬜ |
| 2 | package.json 配置 | ⬜ |
| 3 | language-configuration.json | ⬜ |
| 4 | 语法高亮（可选） | ⬜ |
| 5 | README.md | ⬜ |

---

## 测试项目汇总

### 手动测试

1. **步骤 2**：验证 package.json 语法
2. **步骤 3**：
   - 打开 .yx 文件，按 Ctrl+/ 验证注释生效
   - 输入括号验证匹配高亮
3. **步骤 4**：验证关键字、字符串、数字、注释的着色
4. **步骤 5**：验证文档可读性

---

## 后续扩展

当 LSP 服务器实现完成后，可扩展以下功能：

1. **自动 LSP 发现**：扩展包自动检测 `yaoxiang-lsp` 是否在 PATH 中
2. **状态栏集成**：显示 LSP 连接状态
3. **调试集成**：基于 DAP 的调试入口
4. **项目模板**：一键创建 YaoXiang 项目

---

## 参考资料

- [VS Code Extension Guidelines](https://code.visualstudio.com/api)
- [Language Extension Overview](https://code.visualstudio.com/api/language-extensions)
- [Syntax Highlight Guide](https://code.visualstudio.com/api/language-extensions/syntax-highlight-guide)
