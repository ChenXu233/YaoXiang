# 多文件项目 Fixtures

这些项目模拟真实开发场景——多个 `.yx` 文件通过 `use` 互相引用。

## 目录约定

每个项目是一个独立目录，结构对齐 `yaoxiang new` 的项目骨架：

```
<project-name>/
├── src/
│   ├── lib.yx       # 类型定义
│   ├── utils.yx     # 工具函数
│   └── main.yx      # 入口
├── yaoxiang.toml    # 项目配置
├── yaoxiang.lock    # 锁文件
├── .gitignore       # git 忽略规则
└── README.md        # 项目说明
```
