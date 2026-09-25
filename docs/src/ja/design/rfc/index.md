---
title: 'RFC インデックス'
---

# YaoXiang RFC（リクエスト・コメント）インデックス

> RFC（Request for Comments）は、YaoXiang言語の機能設計提案の正式な提出フォーマットです。

## 目次

- [テンプレート](#テンプレート)
- [草案RFC](#草案rfc)
- [レビュー中RFC](#レビュー中rfc)
- [承認済みRFC](#承認済みrfc)
- [廃止RFC](#廃止rfc)
- [拒否されたRFC](#拒否されたrfc)
- [ドキュメント改訂ルール](#ドキュメント改訂ルール)

---

## テンプレート

| ファイル                                                                   | 説明                     |
| ------------------------------------------------------------------------ | ------------------------ |
| [RFC_TEMPLATE.md](RFC_TEMPLATE.md)                                       | RFC標準テンプレート              |
| [EXAMPLE_full_feature_proposal.md](EXAMPLE_full_feature_proposal.md) | 完全示例（パターンマッチング拡張） |

---

## 草案RFC

| 番号     | タイトル                                                                                           | 著者 | 作成日   | 状態   |
| -------- | ---------------------------------------------------------------------------------------------- | ---- | ---------- | ------ |
| RFC-002  | [RFC-002：libuvベースのリソース型IO実装層](./draft/002-cross-platform-io-libuv.md)             | 晨煦 | 2026-01-05 | 草案   |
| RFC-019  | [RFC-019: 型レベル同図性 (Typed Homoiconicity) - 構文即ち型](./draft/019-typed-homoiconicity.md) | 晨煦 | 2026-02-20 | 草案   |
| RFC-028  | [RFC-028：JITコンパイラ — VM内多レベル実行エンジン](./draft/028-jit-compiler.md)                         | 晨煦 | 2026-06-11 | 草案   |
| RFC-031  | [RFC-031：最適化レベルとPassマネージャー](./draft/031-optimization-levels.md)                          | 晨煦 | 2026-06-16 | 草案   |
| RFC-033  | [RFC-033: `^^` リフレクション演算子](./draft/033-reflection-operator.md)                                 | 晨煦 | 2026-06-16 | レビュー中 |
| RFC-034  | [RFC-034: 統一デバッグツールチェーン](./draft/034-debug-toolchain.md)                                      | 晨煦 | 2026-07-06 | 草案   |
| RFC-035  | [RFC-035: MCPサーバーサポート（AI Agent統合）](./draft/035-mcp-server.md)                         | 晨煦 | 2026-07-11 | 草案   |
| RFC-010b  | [RFC-010b: パターンマッチング完全性](./draft/010b-pattern-matching-completeness.md)                        | 晨煦 | 2026-09-03 | 草案   |
| RFC-027a | [RFC-027a: 終了検査の明示的測度](./review/027a-termination-explicit-measure.md)                  | 晨煦 | 2026-09-14 | レビュー中 |
| RFC-029a | [RFC-029a: モジュールキャッシュと增量再コンパイル](./draft/029a-module-cache-incremental.md)                     | 晨煦 | 2026-09-07 | 草案   |

---

## レビュー中RFC

| 番号    | タイトル                                                                                                | 著者 | 作成日   | 状態   |
| ------- | --------------------------------------------------------------------------------------------------- | ---- | ---------- | ------ |
| RFC-032 | [RFC-032: spawn統一式修飾 — spawn for特殊ケースの除去](./review/032-spawn-unified-expression.md) | 晨煦 | 2026-06-16 | レビュー中 |

---

## 承認済みRFC

| 番号       | タイトル                                                                                                            | 著者      | 作成日   | 状態             |
| ---------- | --------------------------------------------------------------------------------------------------------------- | --------- | ---------- | ---------------- |
| RFC-004    | [RFC-004: キュアリングされたメソッドの複数位置連合バインディング設計](./accepted/004-curry-multi-position-binding.md)                       | 晨煦      | 2025-01-05 | 承認済み           |
| RFC-006    | [RFC-006: ドキュメントサイト構築](./accepted/006-documentation-site-optimization.md)                                      | 晨煦      | 2025-01-05 | 承認済み           |
| RFC-007    | [RFC-007: 関数定義構文統一方案](./accepted/007-function-syntax-unification.md)                                  | 沫郁酱    | 2025-01-05 | 承認済み           |
| RFC-008    | [RFC-008：Runtime並行モデルとスケジューラ分離設計](./accepted/008-runtime-concurrency-model.md)                        | 晨煦      | 2025-01-05 | 承認済み           |
| RFC-009    | [RFC-009: 所有権モデル設計](./accepted/009-ownership-model.md)                                                    | 晨煦      | 2025-01-08 | 承認済み           |
| ↳ RFC-009a | [RFC-009a: トークンライフタイム分析——ホーア証明パイプラインに基づく](./accepted/009a-borrow-proof-pipeline.md)                          | 晨煦      | 2026-06-13 | 承認済み           |
| RFC-010    | [RFC-010: 統一型構文 - name: type = value モデル](./accepted/010-unified-type-syntax.md)                        | 晨煦      |            | 承認済み           |
| ↳ RFC-010a | [RFC-010a: 末尾式評価とreturnセマンティクス](./accepted/010a-tail-expression-and-return.md)                           | 晨煦      | 2026-09-15 | 承認済み           |
| RFC-011    | [RFC-011: ジェネリクスシステム設計 - ゼロコスト抽象化とマクロ代替](./accepted/011-generic-type-system.md)                             | 晨煦      |            | 承認済み           |
| ↳ RFC-011a | [RFC-011a: インターフェース実装と動的ディスパッチ](./accepted/011a-interface-implementation.md)                                     | 晨煦      | 2026-06-14 | 承認済み           |
| ↳ RFC-011b | [RFC-011b: 演算子オーバーロードとインターフェース駆動演算子](./accepted/011b-operator-overloading.md)                                 | 晨煦      | 2026-09-22 | 承認済み           |
| RFC-012    | [RFC 012: F-Stringテンプレート文字列](./accepted/012-f-string-template-strings.md)                                     | Chen Xu   | 2025-01-27 | 承認済み           |
| RFC-013    | [RFC 013: エラーコード仕様](./accepted/013-error-code-specification.md)                                             | 晨煦      | 2026-02-02 | 承認済み           |
| RFC-014    | [RFC-014: パッケージ管理システム設計](./accepted/014-package-manager.md)                                                    | 晨煦      | 2026-02-12 | 承認済み           |
| ↳ RFC-014a | [RFC-014a: Registryプロトコル仕様](./review/014a-registry-protocol.md)                                               | 晨煦      | 2026-06-11 | レビュー中RFC        |
| ↳ RFC-014b | [RFC-014b: ビルドシステムとバイナリ配布](./review/014b-build-system.md)                                                 | 晨煦      | 2026-06-11 | レビュー中RFC        |
| ↳ RFC-014c | [RFC-014c: ワークスペースサポート](./review/014c-workspace.md)                                                            | 晨煦      | 2026-06-11 | レビュー中RFC        |
| RFC-015    | [RFC-015: YaoXiang設定システム設計](./accepted/015-configuration-system.md)                                        | 晨煦      | 2026-02-12 | 承認済み           |
| RFC-017    | [RFC-017: 言語サーバープロトコル（LSP）サポート設計](./accepted/017-lsp-support.md)                                         | 晨煦      | 2026-02-15 | 実装済み           |
| RFC-018    | [RFC-018：LLVM AOTコンパイラ設計](./accepted/018-llvm-aot-compiler.md)                                             | 晨煦      | 2026-02-15 | 承認済み           |
| RFC-024    | [RFC-024：spawnベースの並行Runtimeセマンティクス](./accepted/024-concurrency-model.md)                                     | 晨煦      | 2026-06-05 | 承認済み（改訂版） |
| RFC-026    | [RFC-026：FFIコアメカニズム](./accepted/026-ffi-core-mechanism.md)                                                   | 晨煦      | 2026-07-03 | 承認済み           |
| ↳ RFC-026a | [RFC-026a: 拡張可能FFIメカニズム体系](./review/026a-extensible-ffi-system.md)                                         | 晨煦      | 2026-06-05 | レビュー中RFC        |
| ↳ RFC-026b | [RFC-026b: yx-bindgenツールチェーン](./draft/026b-yx-bindgen.md)                                                       | 晨煦      | 2026-06-05 | 草案RFC          |
| RFC-027    | [RFC-027：コンパイル時述語と統一静的検証](./accepted/027-compile-time-evaluation-types.md)                            | 晨煦      | 2026-06-07 | 承認済み           |
| RFC-029    | [RFC-029: モジュールセマンティクスシステム](./accepted/029-module-semantics.md)                                                     | 晨煦      | 2026-06-13 | 承認済み           |
| RFC-030    | [RFC-030: assertアサートメカニズム](./accepted/030-assert-mechanism.md)                                                  | 晨煦      | 2026-06-15 | 承認済み           |
| RFC-036    | [RFC-036: std.testテストフレームワークとyaoxiang testコマンド](./accepted/036-test-framework.md)                             | 晨煦      | 2026-07-26 | 承認済み           |
| RFC-037    | [RFC-037: 産業化配布方案 — cargo-distに基づくコンパイラ/ツールチェーンパッケージ](./accepted/037-industrial-packaging.md)         | ChenXu233 | 2026-07-26 | 承認済み           |
| RFC-038    | [RFC-038: 文終了と改行ルール（Statement Termination & Newline Rules）](./accepted/038-statement-termination.md) | ChenXu233 | 2026-08-05 | 承認済み           |
| RFC-029f   | [RFC-029f: コンパイルターゲットロールとインポート面セマンティクス](./accepted/029f-target-semantics.md)                                       | 晨煦      | 2026-09-12 | 承認済み           |

---

## 廃止RFC

| 番号    | タイトル                                                                                                       | 著者 | 作成日   | 状態                      |
| ------- | ---------------------------------------------------------------------------------------------------------- | ---- | ---------- | ------------------------- |
| RFC-001 | [RFC-001：spawnモデルとエラー処理システム](./deprecated/001-concurrent-model-error-handling.md)                     | 晨煦 | 2025-01-05 | 廃止（RFC-024に取代） |
| RFC-020 | [RFC-020：動的モジュールとFFI統合](./deprecated/020-dynamic-modules-ffi.md)                                    | 晨煦 | 2026-03-14 | 廃止                    |
| RFC-021 | [RFC-021: ライブラリ駆動FFI拡張と跨言語呼び出しサポート](./deprecated/021-library-driven-ffi-extension.md)               | 晨煦 | 2026-03-14 | 廃止                    |
| RFC-022 | [RFC 022: ホーア論理静的検証サポート（仕様コメントと仕様型）](./deprecated/022-hoare-logic-static-verification.md) | 晨煦 | 2026-03-16 | 廃止（RFC-027に取代） |
| RFC-023 | [RFC-023: クロージャ捕獲モデル](./deprecated/023-closure-capture-model.md)                                         | 晨煦 | 2026-05-29 | 廃止                    |

---

## 拒否されたRFC

| 番号    | タイトル                                                                            | 著者 | 作成日   | 状態   |
| ------- | ------------------------------------------------------------------------------- | ---- | ---------- | ------ |
| RFC-003 | [RFC-003：バージョニング計画](./rejected/003-version-planning.md)                         | 晨煦 | 2025-01-05 | 拒否 |
| RFC-005 | [RFC-005: 自動CVEセキュリティ検査システム](./rejected/005-automated-cve-scanning.md)      | 晨煦 | 2025-01-05 | 拒否 |
| RFC-016 | [RFC 016: 量子ネイティブサポートとマルチバックエンド統合](./rejected/016-quantum-native-support.md) | 晨煦 | 2026-02-13 | 拒否 |
| RFC-025 | [RFC-025: 拡張可能プリミティブ型メカニズム](./rejected/025-primitive-extension.md)            | 晨煦 | 2026-06-05 | 拒否 |

---

## RFCライフサイクル

```
草案 → レビュー中 → 承認済み → 廃止済み（取代됨）
                  ↓
               拒否済み（不通過）
```

### 状態説明

| 状態       | 位置              | 説明                           |
| ---------- | ----------------- | ------------------------------ |
| **草案**   | `rfc/draft/`      | 著者草案、レビュー提出待ち         |
| **レビュー中** | `rfc/review/`     | コミュニティ議論とフィードバック公開             |
| **承認済み** | `rfc/accepted/`   | 正式設計文書、实现段階に移行 |
| **廃止済み** | `rfc/deprecated/` | かつて承認済み、新しい設計に取代         |
| **拒否済み** | `rfc/rejected/`   | 拒否されたRFC文書                |

---

## ドキュメント改訂ルール

**RFC文書は正しい情報のみを含めることができます。** 設計変更時は、原文を直接修正して現在の正しいセマンティクスを反映させる；
**誤り内容を残したまま「正誤表」ブロックで修正することは禁止です。**

「原文 + 正誤表」の保持是最悪の書き方：読者が途中でようやく前半全体が無駄だったことに気づき、読みコストが無駄になり、廃止された段落を現行のセマンティクスとして誤って引用してしまう可能性があります。正誤表ブロックは慎重に見えますが，实际上是把整理コスト转嫁给了读者。

### 正しい做法

| 情形                           | 做法                                                                     |
| ------------------------------ | ------------------------------------------------------------------------ |
| 実現と原文の不一致、原文の否定     | その段落を正しい内容に書き直し、原表述を削除                                     |
| 示例コードが実行不可能になった             | そのまま実行可能な形態に修正、旧示例を残す必要はない                                     |
| 読者に「以前はどうだったか」を知らせる必要がある         | **誤った比較を特意に行う**場合にのみ誤り内容を残し、紧隣に「この写法は誤り」及其理由を标注 |
| 設計の進化を追溯する必要がある               | Gitコミット情報またはissueに記述し、RFC本文には書かない                            |

### 例外

以下の誤り情報は保持できます：

- **特意に行った対比教学**：正誤対照であることを明示し、誤り側に紧隣「なぜ误りか」を説明
- **廃止済みRFC**（`rfc/deprecated/`）：歴史記録としてだが、何に取代されたかを标注

### 補助手段

- RFC上部の `status` / `updated` フィールドが最新改訂時間を反映、正문에「この改訂は何をしたか」を書く必要はない
- 実現状況はテーブルで ✅ / ❌ で表現し、正문에状態叙述を挟まない
- 完全な改訂履歴は `git log -- <ファイル>` で確認

---

## RFCの提交

1. [RFC_TEMPLATE.md](RFC_TEMPLATE.md) を読んでフォーマット要件を理解する
2. [EXAMPLE_full_feature_proposal.md](EXAMPLE_full_feature_proposal.md) を参考にする
3. 新規ファイルを作成し、`番号-記述的タイトル.md` と命名する
4. ファイルを `docs/reference/rfc/draft/` ディレクトリに配置する
5. このインデックスファイルを更新し、新しいRFCエントリを追加する
6. PRを提交してレビュー流程に入る

---

## 貢献ガイド

貢献ガイドについては CONTRIBUTING.md をご覧ください。