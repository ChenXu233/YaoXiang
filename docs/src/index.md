---
layout: page
is_home: true
sidebar: false

hero:
  name: YaoXiang // 爻象
  text: 一个面向未来的编程语言
  tagline: 万物并作，吾以观复
  actions:
    - theme: brand
      text: 🚀 快速开始
      link: /getting-started
    - theme: alt
      text: 教程
      link: /tutorial/
    - theme: brand
      text: 下载本体
      link: /download
    - theme: alt
      text: GitHub ⇗
      link: https://github.com/yaoxiang-lang/yaoxiang

tracks:
  track01:
    trackLabel: TRACK 01
    rfc: RFC-010
    title: 统一语法
    description: "极简主义哲学。从变量到函数，所有声明都遵循 name: type = value 模式，学习成本更低，代码更一致。"
    features:
      - 语法声明极致统一
      - 类型是一等公民
  track02:
    rfc: RFC-011
    title: 零成本泛型
    description: "泛型特化在编译期完成，类型抽象不带来任何运行时开销。编译期单态化。死代码消除。类型系统即宏。"
  track03:
    rfc: RFC-009
    title: 所有权模型
    description: "告别 GC 卡顿。爻象用基于作用域的所有权模型，内存安全在编译期就确定，没有意外。"
    features:
      - 共享引用
      - 可预测
      - 无 GC 卡顿
      - 无生命周期
  track04:
    trackLabel: TRACK 04
    title: 解耦调度器
    description: "从单片机到高性能服务器，运行时自适应环境。不同场景选择不同调度策略，性能与资源兼得。"
    steps:
      - label: Embedded
        sub: "完全同步(Sync)"
      - label: Standard
        sub: "基于有向无环图(DAG)和惰性求值的自动化并发管理"
      - label: Full
        sub: "工作窃取机制(WorkSteal)"
  track05:
    title: 语言规范 v1.6
    description: "拒绝语法糖轰炸。18 个关键字覆盖所有特性，没有复杂的语法糖，只有纯粹的表达力。"

---
