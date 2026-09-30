---
title: 'RFC インデックス'
---

# YaoXiang RFC（コメント募集）インデックス

> RFC（Request for Comments）はYaoXiang言語機能設計提案の正式な提出形式です。

## 目次

- [テンプレート](#テンプレート)
- [ドラフトrfc](#ドラフトrfc)
- [レビュー中rfc](#レビュー中rfc)
- [承認済みrfc](#承認済みrfc)
- [廃止済みrfc](#廃止済みrfc)
- [拒否済みrfc](#拒否済みrfc)
- [ドキュメント改訂ルール](#ドキュメント改訂ルール)

---

## テンプレート

| ファイル                                                             | 説明                               |
| -------------------------------------------------------------------- | ---------------------------------- |
| [RFC_TEMPLATE.md](RFC_TEMPLATE.md)                                   | RFC 標準テンプレート               |
| [EXAMPLE_full_feature_proposal.md](EXAMPLE_full_feature_proposal.md) | 完全な例（パターンマッチング強化） |

---

## ドラフトRFC

| 番号     | タイトル                                                                                       | 著者 | 作成日     | ステータス |
| -------- | ---------------------------------------------------------------------------------------------- | ---- | ---------- | ---------- |
| RFC-002  | [RFC-002：基于 libuv 的资源类型 IO 实现层](./draft/002-cross-platform-io-libuv.md)             | 晨煦 | 2026-01-05 | ドラフト   |
| RFC-019  | [RFC-019: 类型级同像性 (Typed Homoiconicity) - 语法即类型](./draft/019-typed-homoiconicity.md) | 晨煦 | 2026-02-20 | ドラフト   |
| RFC-028  | [RFC-028：JIT 编译器 — VM 内多级执行引擎](./draft/028-jit-compiler.md)                         | 晨煦 | 2026-06-11 | ドラフト   |
| RFC-031  | [RFC-031：优化级别与 Pass 管理器](./draft/031-optimization-levels.md)                          | 晨煦 | 2026-06-16 | ドラフト   |
| RFC-033  | [RFC-033: `^^` 反射运算符](./draft/033-reflection-operator.md)                                 | 晨煦 | 2026-06-16 | レビュー中 |
| RFC-034  | [RFC-034: 统一调试工具链](./draft/034-debug-toolchain.md)                                      | 晨煦 | 2026-07-06 | ドラフト   |
| RFC-035  | [RFC-035: MCP Server 支持（AI Agent 集成）](./draft/035-mcp-server.md)                         | 晨煦 | 2026-07-11 | ドラフト   |
| RFC-027a | [RFC-027a: 终止检查的显式测度](./review/027a-termination-explicit-measure.md)                  | 晨煦 | 2026-09-14 | レビュー中 |
| RFC-029a | [RFC-029a: 模块缓存与增量重编译](./draft/029a-module-cache-incremental.md)                     | 晨煦 | 2026-09-07 | ドラフト   |

---

## レビュー中RFC

| 番号    | タイトル                                                                                            | 著者 | 作成日     | ステータス |
| ------- | --------------------------------------------------------------------------------------------------- | ---- | ---------- | ---------- |
| RFC-032 | [RFC-032: spawn 统一表达式修饰 — 消除 spawn for 特殊情况](./review/032-spawn-unified-expression.md) | 晨煦 | 2026-06-16 | レビュー中 |

---

## 承認済みRFC

| 番号       | タイトル                                                                                                        | 著者      | 作成日     | ステータス         |
| ---------- | --------------------------------------------------------------------------------------------------------------- | --------- | ---------- | ------------------ |
| RFC-004    | [RFC-004: 柯里化方法的多位置联合绑定设计](./accepted/004-curry-multi-position-binding.md)                       | 晨煦      | 2025-01-05 | 承認済み           |
| RFC-006    | [RFC-006: 文档站点建设](./accepted/006-documentation-site-optimization.md)                                      | 晨煦      | 2025-01-05 | 承認済み           |
| RFC-007    | [RFC-007: 函数定义语法统一方案](./accepted/007-function-syntax-unification.md)                                  | 沫郁酱    | 2025-01-05 | 承認済み           |
| RFC-008    | [RFC-008：Runtime 并发模型与调度器脱耦设计](./accepted/008-runtime-concurrency-model.md)                        | 晨煦      | 2025-01-05 | 承認済み           |
| RFC-009    | [RFC-009: 所有权模型设计](./accepted/009-ownership-model.md)                                                    | 晨煦      | 2025-01-08 | 承認済み           |
| ↳ RFC-009a | [RFC-009a: 令牌生命期分析——基于霍尔证明管道](./accepted/009a-borrow-proof-pipeline.md)                          | 晨煦      | 2026-06-13 | 承認済み           |
| RFC-010    | [RFC-010: 统一类型语法 - name: type = value 模型](./accepted/010-unified-type-syntax.md)                        | 晨煦      |            | 承認済み           |
| ↳ RFC-010a | [RFC-010a: 尾表达式求值与 return 语义](./accepted/010a-tail-expression-and-return.md)                           | 晨煦      | 2026-09-15 | 承認済み           |
| ↳ RFC-010b | [RFC-010b: 模式匹配完备化（变体解构与穷尽性）](./accepted/010b-pattern-matching-completeness.md)                | 晨煦      | 2026-09-03 | 承認済み           |
| RFC-011    | [RFC-011: 泛型系统设计 - 零成本抽象与宏替代](./accepted/011-generic-type-system.md)                             | 晨煦      |            | 承認済み           |
| ↳ RFC-011a | [RFC-011a: 接口实现与动态分发](./accepted/011a-interface-implementation.md)                                     | 晨煦      | 2026-06-14 | 承認済み           |
| ↳ RFC-011b | [RFC-011b: 运算符重载与接口驱动运算符](./accepted/011b-operator-overloading.md)                                 | 晨煦      | 2026-09-22 | 承認済み           |
| RFC-012    | [RFC 012: F-String 模板字符串](./accepted/012-f-string-template-strings.md)                                     | Chen Xu   | 2025-01-27 | 承認済み           |
| RFC-013    | [RFC 013: 错误代码规范](./accepted/013-error-code-specification.md)                                             | 晨煦      | 2026-02-02 | 承認済み           |
| RFC-014    | [RFC-014: 包管理系统设计](./accepted/014-package-manager.md)                                                    | 晨煦      | 2026-02-12 | 承認済み           |
| ↳ RFC-014a | [RFC-014a: Registry 协议规范](./accepted/014a-registry-protocol.md)                                               | 晨煦      | 2026-06-11 | 承認済み     |
| ↳ RFC-014b | [RFC-014b: 构建系统与二进制分发](./accepted/014b-build-system.md)                                                 | 晨煦      | 2026-06-11 | 承認済み     |
| ↳ RFC-014c | [RFC-014c: 工作空间支持](./accepted/014c-workspace.md)                                                            | 晨煦      | 2026-06-11 | 承認済み     |
| RFC-015    | [RFC-015: YaoXiang 配置系统设计](./accepted/015-configuration-system.md)                                        | 晨煦      | 2026-02-12 | 承認済み           |
| RFC-017    | [RFC-017: 语言服务器协议（LSP）支持设计](./accepted/017-lsp-support.md)                                         | 晨煦      | 2026-02-15 | 実装済み           |
| RFC-018    | [RFC-018：LLVM AOT 编译器设计](./accepted/018-llvm-aot-compiler.md)                                             | 晨煦      | 2026-02-15 | 承認済み           |
| RFC-024    | [RFC-024：基于 spawn 的并发运行时语义](./accepted/024-concurrency-model.md)                                     | 晨煦      | 2026-06-05 | 承認済み（改訂版） |
| RFC-026    | [RFC-026：FFI 核心机制](./accepted/026-ffi-core-mechanism.md)                                                   | 晨煦      | 2026-07-03 | 承認済み           |
| ↳ RFC-026a | [RFC-026a: 可扩展 FFI 机制体系](./review/026a-extensible-ffi-system.md)                                         | 晨煦      | 2026-06-05 | レビュー中 RFC     |
| ↳ RFC-026b | [RFC-026b: yx-bindgen 工具链](./draft/026b-yx-bindgen.md)                                                       | 晨煦      | 2026-06-05 | ドラフト RFC       |
| RFC-027    | [RFC-027：编译期谓词与统一静态验证](./accepted/027-compile-time-evaluation-types.md)                            | 晨煦      | 2026-06-07 | 承認済み           |
| RFC-029    | [RFC-029: 模块语义系统](./accepted/029-module-semantics.md)                                                     | 晨煦      | 2026-06-13 | 承認済み           |
| RFC-030    | [RFC-030: assert 断言机制](./accepted/030-assert-mechanism.md)                                                  | 晨煦      | 2026-06-15 | 承認済み           |
| RFC-036    | [RFC-036: std.test 测试框架与 yaoxiang test 命令](./accepted/036-test-framework.md)                             | 晨煦      | 2026-07-26 | 承認済み           |
| RFC-037    | [RFC-037: 工业化分发方案 — 基于 cargo-dist 的编译器/工具链打包](./accepted/037-industrial-packaging.md)         | ChenXu233 | 2026-07-26 | 承認済み           |
| RFC-038    | [RFC-038: 语句终止与换行规则（Statement Termination & Newline Rules）](./accepted/038-statement-termination.md) | ChenXu233 | 2026-08-05 | 承認済み           |
| RFC-029f   | [RFC-029f: 编译目标角色与导入面语义](./accepted/029f-target-semantics.md)                                       | 晨煦      | 2026-09-12 | 承認済み           |

---

## 廃止済みRFC

| 番号    | タイトル                                                                                                   | 著者 | 作成日     | ステータス                     |
| ------- | ---------------------------------------------------------------------------------------------------------- | ---- | ---------- | ------------------------------ |
| RFC-001 | [RFC-001：并作模型与错误处理系统](./deprecated/001-concurrent-model-error-handling.md)                     | 晨煦 | 2025-01-05 | 廃止済み（RFC-024 に置き換え） |
| RFC-020 | [RFC-020：动态模块与 FFI 集成](./deprecated/020-dynamic-modules-ffi.md)                                    | 晨煦 | 2026-03-14 | 廃止済み                       |
| RFC-021 | [RFC-021: 库驱动 FFI 扩展与跨语言调用支持](./deprecated/021-library-driven-ffi-extension.md)               | 晨煦 | 2026-03-14 | 廃止済み                       |
| RFC-022 | [RFC 022: 霍尔逻辑静态验证支持（规约注释与规约类型）](./deprecated/022-hoare-logic-static-verification.md) | 晨煦 | 2026-03-16 | 廃止済み（RFC-027 に置き換え） |
| RFC-023 | [RFC-023: 闭包捕获模型](./deprecated/023-closure-capture-model.md)                                         | 晨煦 | 2026-05-29 | 廃止済み                       |

---

## 拒否済みRFC

| 番号    | タイトル                                                                        | 著者 | 作成日     | ステータス |
| ------- | ------------------------------------------------------------------------------- | ---- | ---------- | ---------- |
| RFC-003 | [RFC-003：版本规划](./rejected/003-version-planning.md)                         | 晨煦 | 2025-01-05 | 拒否済み   |
| RFC-005 | [RFC-005: 自动化CVE安全检查系统](./rejected/005-automated-cve-scanning.md)      | 晨煦 | 2025-01-05 | 拒否済み   |
| RFC-016 | [RFC 016: 量子原生支持与多重后端集成](./rejected/016-quantum-native-support.md) | 晨煦 | 2026-02-13 | 拒否済み   |
| RFC-025 | [RFC-025: 可扩展原语类型机制](./rejected/025-primitive-extension.md)            | 晨煦 | 2026-06-05 | 拒否済み   |

---

## RFCライフサイクル

```
ドラフト → レビュー中 → 承認済み → 廃止（置き換え）
                              ↓
                          拒否（不承認）
```

### ステータス説明

| ステータス     | 場所              | 説明                                           |
| -------------- | ----------------- | ---------------------------------------------- |
| **ドラフト**   | `rfc/draft/`      | 著者のドラフト、レビュー提出待ち               |
| **レビュー中** | `rfc/review/`     | コミュニティでの議論とフィードバック募集中     |
| **承認済み**   | `rfc/accepted/`   | 正式な設計文書となり、実装段階に入る           |
| **廃止**       | `rfc/deprecated/` | 過去に承認されたが、新しい設計に置き換えられた |
| **拒否**       | `rfc/rejected/`   | 拒否された RFC ドキュメント                    |

---

## ドキュメント改訂ルール

**RFC ドキュメントには正しい情報のみを含める。**
設計が変更された場合は、原典を直接修正して現在の正しいセマンティクスを表現する；
**誤った内容を残し、「正誤表」ブロックで修正することはしない**。

「原文 + 正誤表」を残すのは最悪の書き方である。読者が途中まで読んで初めて前の部分がすべて無効だったことに気づき、前の部分の読むコストが無駄になる。また、廃止された段落を現在のセマンティクスとして誤って引用しやすい。正誤表ブロックは一見慎重に感じるが、実際には整理コストを-readerに転嫁している。

### 正しい方法

| 状況                                                 | 方法                                                                                                           |
| ---------------------------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| 実装と原文が一致しない、原文が覆された               | その段落を直接正しい内容に書き換え、元の表現を削除する                                                         |
| サンプルコードが実行できなくなった                   | 実行可能な形にそのまま変更し、古い例を保持しない                                                               |
| 読者に「以前はどのようだったか」を知らせる必要がある | **意図的に誤った比較を行う**場合にのみ誤った内容を保持し、隣接して「この書き方は誤りである」及び理由を明記する |
| 設計の変遷を遡る必要がある                           | Git コミットメッセージまたは issue に記述し、RFC 本文には書かない                                              |

### 例外

以下のエラー情報は保持できる：

- **意図的に行う比較教育**：正誤対照として明確にマークし、誤り側には「なぜ誤りか」の説明を隣接して記載する
- **廃止された RFC**（`rfc/deprecated/`）：歴史的記録として保持されるが、何に置き換えられたかを明記する

### 補助手段

- RFC 冒頭の `status` / `updated`
  フィールドは最新の改訂時間を反映し、本文に「今回の改訂内容」を書く必要はない
- 実装状態は表の ✅ / ❌ で表現し、本文に状態の説明を挟まない
- 完全な改訂履歴は `git log -- <ファイル>` で確認する

---

## RFCを提出する

1. [RFC_TEMPLATE.md](RFC_TEMPLATE.md) を読んでフォーマット要件を確認する
2. [EXAMPLE_full_feature_proposal.md](EXAMPLE_full_feature_proposal.md) を参考に書き方を学ぶ
3. 新しいファイルを作成し、`番号-説明的なタイトル.md` という名前を付ける
4. ファイルを `docs/reference/rfc/draft/` ディレクトリに配置する
5. 本インデックスファイルを更新し、新しい RFC エントリを追加する
6. PR を提出してレビュープロセスに進む

---

## 貢献ガイド

貢献ガイドについては CONTRIBUTING.md を参照してください。
