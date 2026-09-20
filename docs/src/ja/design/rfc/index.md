---
title: 'RFC インデックス'
---

# YaoXiang RFC（Request for Comments）インデックス

> RFC（Request for Comments）はYaoXiang言語の特性設計提案の正式な提出フォーマットです。

## 目次

- [テンプレート](#テンプレート)
- [ドラフトRFC](#ドラフトrfc)
- [レビュー中RFC](#レビュー中rfc)
- [承認済みRFC](#承認済みrfc)
- [廃止RFC](#廃止rfc)
- [却下RFC](#却下rfc)
- [ドキュメント改訂ルール](#ドキュメント改訂ルール)

---

## テンプレート

| ファイル                                                             | 説明                                       |
| -------------------------------------------------------------------- | ------------------------------------------ |
| [RFC_TEMPLATE.md](RFC_TEMPLATE.md)                                   | RFC標準テンプレート                        |
| [EXAMPLE_full_feature_proposal.md](EXAMPLE_full_feature_proposal.md) | 完全なサンプル（パターンマッチングの強化） |

---

## ドラフトRFC

| 番号     | タイトル                                                                                                 | 著者 | 作成日     | ステータス |
| -------- | -------------------------------------------------------------------------------------------------------- | ---- | ---------- | ---------- |
| RFC-002  | [RFC-002：libuv ベースのリソース型 IO 実装層](./draft/002-cross-platform-io-libuv.md)                    | 晨煦 | 2026-01-05 | ドラフト   |
| RFC-019  | [RFC-019: 型レベル同像性 (Typed Homoiconicity) - 構文即ち型](./draft/019-typed-homoiconicity.md)         | 晨煦 | 2026-02-20 | ドラフト   |
| RFC-028  | [RFC-028：JIT コンパイラ — VM 内複数レベル実行エンジン](./draft/028-jit-compiler.md)                     | 晨煦 | 2026-06-11 | ドラフト   |
| RFC-031  | [RFC-031：最適化レベルと Pass マネージャー](./draft/031-optimization-levels.md)                          | 晨煦 | 2026-06-16 | ドラフト   |
| RFC-033  | [RFC-033: `^^` 反射演算子](./draft/033-reflection-operator.md)                                           | 晨煦 | 2026-06-16 | レビュー中 |
| RFC-034  | [RFC-034: 統合デバッグツールチェーン](./draft/034-debug-toolchain.md)                                    | 晨煦 | 2026-07-06 | ドラフト   |
| RFC-035  | [RFC-035: MCP Server サポート（AI Agent 統合）](./draft/035-mcp-server.md)                               | 晨煦 | 2026-07-11 | ドラフト   |
| RFC-039  | [RFC-039: パターンマッチング完全化](./draft/039-pattern-matching-completeness.md)                        | 晨煦 | 2026-09-03 | ドラフト   |
| RFC-027a | [RFC-027a: 停止検査の証明関数フォールバック](./draft/027a-termination-proof-fallback.md)                 | 晨煦 | 2026-09-14 | ドラフト   |
| RFC-029a | [RFC-029a: モジュールキャッシュとインクリメンタル再コンパイル](./draft/029a-module-cache-incremental.md) | 晨煦 | 2026-09-07 | ドラフト   |

---

## レビュー中RFC

| 番号    | タイトル                                                                                           | 著者 | 作成日     | ステータス |
| ------- | -------------------------------------------------------------------------------------------------- | ---- | ---------- | ---------- |
| RFC-032 | [RFC-032: spawn 統一式修飾 — spawn for 特殊ケースの解消](./review/032-spawn-unified-expression.md) | 晨煦 | 2026-06-16 | レビュー中 |

---

## 承認済みRFC

| 番号       | タイトル                                                                                                                       | 著者      | 作成日     | ステータス         |
| ---------- | ------------------------------------------------------------------------------------------------------------------------------ | --------- | ---------- | ------------------ |
| RFC-004    | [RFC-004: カリー化メソッドの複数位置ユニオン束縛設計](./accepted/004-curry-multi-position-binding.md)                          | 晨煦      | 2025-01-05 | 承認済み           |
| RFC-006    | [RFC-006: ドキュメントサイト構築](./accepted/006-documentation-site-optimization.md)                                           | 晨煦      | 2025-01-05 | 承認済み           |
| RFC-007    | [RFC-007: 関数定義構文の統一方案](./accepted/007-function-syntax-unification.md)                                               | 沫郁酱    | 2025-01-05 | 承認済み           |
| RFC-008    | [RFC-008：Runtime 並行モデルとスケジューラの疎結合設計](./accepted/008-runtime-concurrency-model.md)                           | 晨煦      | 2025-01-05 | 承認済み           |
| RFC-009    | [RFC-009: 所有権モデル設計](./accepted/009-ownership-model.md)                                                                 | 晨煦      | 2025-01-08 | 承認済み           |
| ↳ RFC-009a | [RFC-009a: トークンライフタイム解析——ホーア証明パイプラインに基づく](./accepted/009a-borrow-proof-pipeline.md)                 | 晨煦      | 2026-06-13 | 承認済み           |
| RFC-010    | [RFC-010: 統一型構文 - name: type = value モデル](./accepted/010-unified-type-syntax.md)                                       | 晨煦      |            | 承認済み           |
| ↳ RFC-010a | [RFC-010a: 末尾式評価と return セマンティクス](./accepted/010a-tail-expression-and-return.md)                                  | 晨煦      | 2026-09-15 | 承認済み           |
| RFC-011    | [RFC-011: ジェネリクスシステム設計 - ゼロコスト抽象とマクロ代替](./accepted/011-generic-type-system.md)                        | 晨煦      |            | 承認済み           |
| ↳ RFC-011a | [RFC-011a: インターフェース実装と動的ディスパッチ](./accepted/011a-interface-implementation.md)                                | 晨煦      | 2026-06-14 | 承認済み           |
| ↳ RFC-011b | [RFC-011b: 演算子オーバーロードとインターフェース駆動演算子](./draft/011b-operator-overloading.md)                             | 晨煦      | 2026-09-15 | ドラフトRFC        |
| RFC-012    | [RFC 012: F-String テンプレート文字列](./accepted/012-f-string-template-strings.md)                                            | Chen Xu   | 2025-01-27 | 承認済み           |
| RFC-013    | [RFC 013: エラーコード規範](./accepted/013-error-code-specification.md)                                                        | 晨煦      | 2026-02-02 | 承認済み           |
| RFC-014    | [RFC-014: パッケージ管理システム設計](./accepted/014-package-manager.md)                                                       | 晨煦      | 2026-02-12 | 承認済み           |
| ↳ RFC-014a | [RFC-014a: Registry プロトコル規範](./review/014a-registry-protocol.md)                                                        | 晨煦      | 2026-06-11 | レビュー中RFC      |
| ↳ RFC-014b | [RFC-014b: ビルドシステムとバイナリ配布](./review/014b-build-system.md)                                                        | 晨煦      | 2026-06-11 | レビュー中RFC      |
| ↳ RFC-014c | [RFC-014c: ワークスペースサポート](./review/014c-workspace.md)                                                                 | 晨煦      | 2026-06-11 | レビュー中RFC      |
| RFC-015    | [RFC-015: YaoXiang 設定システム設計](./accepted/015-configuration-system.md)                                                   | 晨煦      | 2026-02-12 | 承認済み           |
| RFC-017    | [RFC-017: Language Server Protocol（LSP）サポート設計](./accepted/017-lsp-support.md)                                          | 晨煦      | 2026-02-15 | 実装済み           |
| RFC-018    | [RFC-018：LLVM AOT コンパイラ設計](./accepted/018-llvm-aot-compiler.md)                                                        | 晨煦      | 2026-02-15 | 承認済み           |
| RFC-024    | [RFC-024：spawn ベースの並行ランタイムセマンティクス](./accepted/024-concurrency-model.md)                                     | 晨煦      | 2026-06-05 | 承認済み（改訂版） |
| RFC-026    | [RFC-026：FFI コアメカニズム](./accepted/026-ffi-core-mechanism.md)                                                            | 晨煦      | 2026-07-03 | 承認済み           |
| ↳ RFC-026a | [RFC-026a: 拡張可能 FFI メカニズム体系](./review/026a-extensible-ffi-system.md)                                                | 晨煦      | 2026-06-05 | レビュー中RFC      |
| ↳ RFC-026b | [RFC-026b: yx-bindgen ツールチェーン](./draft/026b-yx-bindgen.md)                                                              | 晨煦      | 2026-06-05 | ドラフトRFC        |
| RFC-027    | [RFC-027：コンパイル時述語と統一静的検証](./accepted/027-compile-time-evaluation-types.md)                                     | 晨煦      | 2026-06-07 | 承認済み           |
| RFC-029    | [RFC-029: モジュールセマンティクスシステム](./accepted/029-module-semantics.md)                                                | 晨煦      | 2026-06-13 | 承認済み           |
| RFC-030    | [RFC-030: assert 断言メカニズム](./accepted/030-assert-mechanism.md)                                                           | 晨煦      | 2026-06-15 | 承認済み           |
| RFC-036    | [RFC-036: std.test テストフレームワークと yaoxiang test コマンド](./accepted/036-test-framework.md)                            | 晨煦      | 2026-07-26 | 承認済み           |
| RFC-037    | [RFC-037: 工業的配布方案 — cargo-dist ベースのコンパイラ/ツールチェーンパッケージング](./accepted/037-industrial-packaging.md) | ChenXu233 | 2026-07-26 | 承認済み           |
| RFC-038    | [RFC-038: 文の終端と改行ルール（Statement Termination & Newline Rules）](./accepted/038-statement-termination.md)              | ChenXu233 | 2026-08-05 | 承認済み           |
| RFC-029f   | [RFC-029f: コンパイルターゲットロールとインポート面セマンティクス](./accepted/029f-target-semantics.md)                        | 晨煦      | 2026-09-12 | 承認済み           |

---

## 廃止RFC

| 番号    | タイトル                                                                                                           | 著者 | 作成日     | ステータス             |
| ------- | ------------------------------------------------------------------------------------------------------------------ | ---- | ---------- | ---------------------- |
| RFC-001 | [RFC-001：spawn モデルとエラーハンドリングシステム](./deprecated/001-concurrent-model-error-handling.md)           | 晨煦 | 2025-01-05 | 廃止（RFC-024 に置換） |
| RFC-020 | [RFC-020：動的モジュールと FFI 統合](./deprecated/020-dynamic-modules-ffi.md)                                      | 晨煦 | 2026-03-14 | 廃止                   |
| RFC-021 | [RFC-021: ライブラリ駆動 FFI 拡張と言語横断呼び出しサポート](./deprecated/021-library-driven-ffi-extension.md)     | 晨煦 | 2026-03-14 | 廃止                   |
| RFC-022 | [RFC 022: ホーア論理静的検証サポート（仕様コメントと仕様型）](./deprecated/022-hoare-logic-static-verification.md) | 晨煦 | 2026-03-16 | 廃止（RFC-027 に置換） |
| RFC-023 | [RFC-023: クロージャキャプチャモデル](./deprecated/023-closure-capture-model.md)                                   | 晨煦 | 2026-05-29 | 廃止                   |

---

## 却下RFC

| 番号    | タイトル                                                                                          | 著者 | 作成日     | ステータス |
| ------- | ------------------------------------------------------------------------------------------------- | ---- | ---------- | ---------- |
| RFC-003 | [RFC-003：バージョン計画](./rejected/003-version-planning.md)                                     | 晨煦 | 2025-01-05 | 却下       |
| RFC-005 | [RFC-005: 自動CVEセキュリティ検査システム](./rejected/005-automated-cve-scanning.md)              | 晨煦 | 2025-01-05 | 却下       |
| RFC-016 | [RFC 016: 量子ネイティブサポートと複数バックエンド統合](./rejected/016-quantum-native-support.md) | 晨煦 | 2026-02-13 | 却下       |
| RFC-025 | [RFC-025: 拡張可能プリミティブ型メカニズム](./rejected/025-primitive-extension.md)                | 晨煦 | 2026-06-05 | 却下       |

---

## RFCライフサイクル

```
ドラフト → レビュー中 → 承認済み → 廃止（置換）
                  ↓
               却下（不承認）
```

### ステータス説明

| ステータス     | 配置場所          | 説明                                         |
| -------------- | ----------------- | -------------------------------------------- |
| **ドラフト**   | `rfc/draft/`      | 著者による草稿、レビュー提出待ち             |
| **レビュー中** | `rfc/review/`     | コミュニティでの議論とフィードバックを公開中 |
| **承認済み**   | `rfc/accepted/`   | 正式な設計ドキュメントとなり、実装段階へ     |
| **廃止**       | `rfc/deprecated/` | 過去に承認されたが、新しい設計に置換された   |
| **却下**       | `rfc/rejected/`   | 却下されたRFCドキュメント                    |

---

## ドキュメント改訂ルール

**RFC ドキュメントには正しい情報のみを含める。**
設計変更時には、原典を直接修正して現在の正しいセマンティクスを表現する。
**誤った内容を残したまま「正誤表」ブロックを追加して修正してはならない。**

「原文 + 正誤表」を残すのは最悪の書き方である：読者は途中でその前がすべて無効だったと気付き、これまでの読書コストが無駄になる。また、廃止された段落を現行のセマンティクスとして引用してしまうリスクもある。正誤表ブロックは一見慎重に

見えますが、実際には整理コストを読者に転嫁しているだけです。

### 正しいやり方

| ケース                                           | やり方                                                                                                 |
| ------------------------------------------------ | ------------------------------------------------------------------------------------------------------ |
| 実装と原文が一致しない、原文が覆された           | その段落を正しい内容に直接書き換え、元表現を削除する                                                   |
| サンプルコードがもはや実行できない               | 実行可能な形態に直接変更し、古いサンプルを残さない                                                     |
| 読者に「以前はどうだったか」を知らせる必要がある | **意図的にエラー対比を行う場合のみ**エラー内容を保持し、直近に「この書き方は誤りである」理由を付記する |
| 設計の変遷を追跡する必要がある                   | Git コミットメッセージや issue に書き、RFC 本文中には書かない                                          |

### 例外

以下のエラー情報は保持してよい：

- **意図的に行う対比教育**：正誤対照として明確にラベル付けされ、エラー側に「なぜ誤りか」の説明が直前にある
- **廃止RFC**（`rfc/deprecated/`）：歴史的記録として保持するが、誰に置換されたかを明記する

### 補助手段

- RFC 上部の `status` / `updated`
  フィールドで最新の改訂時間を反映し、本文中に「今回の改訂内容」を書かない
- 実装ステータスは表の ✅ / ❌ で表現し、本文中に状態の説明を挟まない
- 完全な改訂履歴は `git log -- <ファイル>` で確認する

---

## RFCの提出

1. [RFC_TEMPLATE.md](RFC_TEMPLATE.md) を読んでフォーマット要件を確認する
2. [EXAMPLE_full_feature_proposal.md](EXAMPLE_full_feature_proposal.md) を参考にして書き方を学ぶ
3. 新規ファイルを `番号-説明的なタイトル.md` の名前で作成する
4. ファイルを `docs/reference/rfc/draft/` ディレクトリに配置する
5. 本インデックスファイルを更新し、新規RFCエントリを追加する
6. PRを提出してレビュープロセスに進む

---

## 貢献ガイドライン

貢献ガイドラインについては [CONTRIBUTING.md](../../../../CONTRIBUTING.md) を参照してください。
