---
title: 'RFC インデックス'
---

# YaoXiang RFC（リクエスト・フォー・コメント）インデックス

> RFC（Request for Comments）はYaoXiang言語の特性設計提案の正式な提出形式です。

## ディレクトリ

- [テンプレート](#テンプレート)
- [草案RFC](#草案rfc)
- [審査中RFC](#審査中rfc)
- [承認済みRFC](#承認済みrfc)
- [廃棄済みRFC](#廃棄済みrfc)
- [拒否済みRFC](#拒否済みrfc)

---

## テンプレート

| ファイル                                                                 | 説明                     |
| -------------------------------------------------------------------- | ------------------------ |
| [RFC_TEMPLATE.md](RFC_TEMPLATE.md)                                   | RFC標準テンプレート              |
| [EXAMPLE_full_feature_proposal.md](EXAMPLE_full_feature_proposal.md) | 完全例（パターン照合強化） |

---

## 草案RFC

| 番号     | タイトル                                                                                                 | 作成者      | 作成日   | ステータス             |
| -------- | ---------------------------------------------------------------------------------------------------- | --------- | ---------- | ---------------- |
| RFC-019  | [RFC-019: 型レベル同語性 (Typed Homoiconicity) - 構文即ち型](./draft/019-typed-homoiconicity.md)       | 晨煦      | 2026-02-20 | 草案             |
| RFC-028  | [RFC-028：JIT コンパイラ — VM 内マルチレベル実行エンジン](./draft/028-jit-compiler.md)                               | 晨煦      | 2026-06-11 | 草案             |
| RFC-029  | [RFC-029: モジュール意味論システム](./draft/029-module-semantics.md)                                             | 晨煦      | 2026-06-13 | 草案             |
| RFC-031  | [RFC-031：最適化レベルと Pass 管理](./draft/031-optimization-levels.md)                                | 晨煦      | 2026-06-16 | 草案             |
| RFC-002  | [RFC-002：libuvに基づくリソース型 IO 実装層](./draft/002-cross-platform-io-libuv.md)                   | 晨煦      | 2025-01-05 | 草案（再審査） |
| RFC-026b | [RFC-026b: yx-bindgen ツールチェーン](./draft/026b-yx-bindgen.md)                                            | 晨煦      | 2026-07-03 | 草案             |
| RFC-034  | [RFC-034: 統合デバッグツールチェーン](./draft/034-debug-toolchain.md)                                            | 晨煦      | 2026-07-06 | 草案             |
| RFC-035  | [RFC-035: MCP Server サポート（AI Agent 統合）](./draft/035-mcp-server.md)                               | Chen Xu   | 2026-07-11 | 草案             |
| RFC-036  | [RFC-036: std.test テストフレームワークと yaoxiang test コマンド](./draft/036-test-framework.md)                     | 晨煦      | 2026-07-25 | 草案             |
| RFC-037  | [RFC-037: 産業化配布方案 — cargo-distに基づくコンパイラ/ツールチェインパッケ](./draft/037-industrial-packaging.md) | ChenXu233 | 2026-07-26 | 草案             |

---

## 審査中RFC

| 番号     | タイトル                                                                                                | 作成者 | 作成日   | ステータス   |
| -------- | --------------------------------------------------------------------------------------------------- | ---- | ---------- | ------ |
| RFC-026a | [RFC-026a: 拡張可能な FFI 機構体系](./review/026a-extensible-ffi-system.md)                             | 晨煦 | 2026-07-03 | 審査中 |
| RFC-032  | [RFC-032: spawn 統合式修飾 — spawn for 特殊情况の解消](./review/032-spawn-unified-expression.md) | 晨煦 | 2026-06-16 | 審査中 |

---

## 承認済みRFC

| 番号       | タイトル                                                                                      | 作成者    | 作成日   | ステータス      |
| ---------- | ----------------------------------------------------------------------------------------- | ------- | ---------- | --------- |
| RFC-004    | [RFC-004: カリー化法の多位置統合束縛設計](./accepted/004-curry-multi-position-binding.md) | 晨煦    | 2025-01-05 | 承認済み    |
| RFC-006    | [RFC-006: ドキュメンテーションサイト構築](./accepted/006-documentation-site-optimization.md)                | 晨煦    | 2025-01-05 | 承認済み    |
| RFC-007    | [RFC-007: 関数定義構文統合方案](./accepted/007-function-syntax-unification.md)            | 沫郁酱  | 2025-01-05 | 承認済み    |
| RFC-008    | [RFC-008：Runtime 並行モデルとスケジューラ分離設計](./accepted/008-runtime-concurrency-model.md)  | 晨煦    | 2025-01-05 | 承認済み    |
| RFC-009    | [RFC-009: 所有権モデル設計](./accepted/009-ownership-model.md)                              | 晨煦    | 2025-01-08 | 承認済み    |
| ↳ RFC-009a | [RFC-009a: トークン生涯解析——ホーア証明パイプラインに基づく](./accepted/009a-borrow-proof-pipeline.md)    | 晨煦    | 2026-06-13 | 承認済み    |
| RFC-010    | [RFC-010: 統合型構文 - name: type = value モデル](./accepted/010-unified-type-syntax.md)  | 晨煦    | 2025-01-20 | 承認済み    |
| RFC-011    | [RFC-011:  型システム設計 - ゼロコスト抽象化とマクロ代替](./accepted/011-generic-type-system.md)       | 晨煦    | 2025-01-25 | 承認済み    |
| ↳ RFC-011a | [RFC-011a: インターフェース実装と動的ディスパッチ](./review/011a-interface-implementation.md)                 | 晨煦    | 2026-06-14 | 審査中    |
| RFC-012    | [RFC 012: F-String テンプレート文字列](./accepted/012-f-string-template-strings.md)               | Chen Xu | 2025-01-27 | 承認済み    |
| RFC-013    | [RFC 013: エラーコード仕様](./accepted/013-error-code-specification.md)                       | 晨煦    | 2026-02-02 | 承認済み    |
| RFC-014    | [RFC-014: パッケージ管理システム設計](./accepted/014-package-manager.md)                              | 晨煦    | 2026-02-12 | 承認済み    |
| ↳ RFC-014a | [RFC-014a: Registry プロトコル仕様](./review/014a-registry-protocol.md)                         | 晨煦    | 2026-06-11 | 審査中RFC |
| ↳ RFC-014b | [RFC-014b: ビルドシステムとバイナリ配布](./review/014b-build-system.md)                           | 晨煦    | 2026-06-11 | 審査中RFC |
| ↳ RFC-014c | [RFC-014c: ワークスペースサポート](./review/014c-workspace.md)                                      | 晨煦    | 2026-06-11 | 審査中RFC |
| RFC-015    | [RFC-015: YaoXiang 設定システム設計](./accepted/015-configuration-system.md)                  | 晨煦    | 2026-02-12 | 承認済み    |
| RFC-017    | [RFC-017: 言語サーバプロトコル（LSP）支援設計](./accepted/017-lsp-support.md)                   | 晨煦    | 2026-02-15 | 審査中    |
| RFC-018    | [RFC-018：LLVM AOT コンパイラ設計](./accepted/018-llvm-aot-compiler.md)                       | 晨煦    | 2026-02-15 | 承認済み    |
| RFC-024    | [RFC-024：spawn ブロックに基づく並行モデル](./accepted/024-concurrency-model.md)                   | 晨煦    | 2026-06-05 | 承認済み    |
| RFC-026    | [RFC-026: FFI コア機構](./accepted/026-ffi-core-mechanism.md)                             | 晨煦    | 2026-06-05 | 承認済み    |
| RFC-027    | [RFC-027：コンパイル時述語と統合静的検証](./accepted/027-compile-time-evaluation-types.md)      | 晨煦    | 2026-06-07 | 承認済み    |
| RFC-030    | [RFC-030: assert アサーション機構](./accepted/030-assert-mechanism.md)                            | 晨煦    | 2026-06-15 | 承認済み    |

---

## 廃棄済みRFC

| 番号    | タイトル                                                                                                       | 作成者 | 作成日   | ステータス                      |
| ------- | ---------------------------------------------------------------------------------------------------------- | ---- | ---------- | ------------------------- |
| RFC-001 | [RFC-001：spawn モデルとエラー処理システム](./deprecated/001-concurrent-model-error-handling.md)                     | 晨煦 | 2025-01-05 | 廃棄済み（RFC-024に置換） |
| RFC-020 | [RFC-020：動的モジュールと FFI 統合](./deprecated/020-dynamic-modules-ffi.md)                                    | 晨煦 | 2026-03-14 | 廃棄済み                    |
| RFC-021 | [RFC-021: ライブラリ駆動型 FFI 拡張とクロス言語呼び出し支援](./deprecated/021-library-driven-ffi-extension.md)               | 晨煦 | 2026-03-14 | 廃棄済み                    |
| RFC-022 | [RFC 022: ホーア論理静的検証支援（仕様コメントと仕様型）](./deprecated/022-hoare-logic-static-verification.md) | 晨煦 | 2026-03-16 | 廃棄済み（RFC-027に置換） |
| RFC-023 | [RFC-023: クロージャ捕獲モデル](./deprecated/023-closure-capture-model.md)                                         | 晨煦 | 2026-05-29 | 廃棄済み                    |

---

## 拒否済みRFC

| 番号    | タイトル                                                                            | 作成者 | 作成日   | ステータス                                |
| ------- | ------------------------------------------------------------------------------- | ---- | ---------- | ----------------------------------- |
| RFC-003 | [RFC-003：バージョ計画](./rejected/003-version-planning.md)                         | 晨煦 | 2025-01-05 | 拒否済み                              |
| RFC-005 | [RFC-005: 自動化 CVE セキュリティ検査システム](./rejected/005-automated-cve-scanning.md)      | 晨煦 | 2025-01-05 | 拒否済み                              |
| RFC-016 | [RFC 016: 量子ネイティブ支援とマルチバックエンド統合](./rejected/016-quantum-native-support.md) | 晨煦 | 2026-02-13 | 拒否済み                              |
| RFC-025 | [RFC-025: 拡張可能なプリミティブ型機構](./rejected/025-primitive-extension.md)            | 晨煦 | 2026-06-05 | 拒否済み（RFC-026 の不透明ハンドルがカバー） |

---

## RFC ライフサイクル

```
草案 → 審査中 → 承認済み → 廃棄済み（置換）
                  ↓
               拒否済み（不通過）
```

### ステータス説明

| ステータス       | 位置          | 説明                   |
| ---------- | ------------- | ---------------------- |
| **草案**   | `rfc/draft/`  | 作成者草稿、提出待ち |
| **審査中** | `rfc/review/` | コミュニティ議論中     |

| **廃棄済み** | `rfc/deprecated/` | 承認後新品に取代 | | **拒否済み** | `rfc/rejected/` | 却下された RFC 文書 |

---

## RFC の提出

1. [RFC_TEMPLATE.md](RFC_TEMPLATE.md) を読んで形式を確認
2. [EXAMPLE_full_feature_proposal.md](EXAMPLE_full_feature_proposal.md) を参照して執筆方法を学ぶ
3. 新規ファイルを作成し、`番号-記述的タイトル.md` と命名
4. ファイルを `docs/src/design/rfc/draft/` ディレクトリに配置
5. 本索引ファイルを更新し、新規 RFC 条目を追加
6. 審査プロセスに PR を提出

---

## 寄稿ガイド

寄稿ガイドについては [CONTRIBUTING.md](../../../../CONTRIBUTING.md) を参照してください。
