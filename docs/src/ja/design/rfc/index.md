---
title: 'RFC インデックス'
---

# YaoXiang RFC（Request for Comments）インデックス

> RFC（Request for Comments）は YaoXiang 言語機能設計提案の正式な提出フォーマットです。

## 目次

- [テンプレート](#テンプレート)
- [ドラフト RFC](#ドラフトrfc)
- [レビュー中 RFC](#レビュー中rfc)
- [採用済み RFC](#採用済みrfc)
- [廃止 RFC](#廃止rfc)
- [却下 RFC](#却下rfc)

---

## テンプレート

| ファイル                                                             | 説明                                    |
| -------------------------------------------------------------------- | --------------------------------------- |
| [RFC_TEMPLATE.md](RFC_TEMPLATE.md)                                   | RFC 標準テンプレート                    |
| [EXAMPLE_full_feature_proposal.md](EXAMPLE_full_feature_proposal.md) | 完全なサンプル（pattern matching 強化） |

---

## ドラフト RFC

| 番号     | タイトル                                                                                                                              | 著者      | 作成日     | ステータス             |
| -------- | ------------------------------------------------------------------------------------------------------------------------------------- | --------- | ---------- | ---------------------- |
| RFC-019  | [RFC-019: タイプレベル同像性 (Typed Homoiconicity) - 構文はタイプそのもの](./draft/019-typed-homoiconicity.md)                        | 晨煦      | 2026-02-20 | ドラフト               |
| RFC-028  | [RFC-028：JIT コンパイラ — VM 内マルチレベル実行エンジン](./draft/028-jit-compiler.md)                                                | 晨煦      | 2026-06-11 | ドラフト               |
| RFC-029  | [RFC-029: モジュール意味論システム](./draft/029-module-semantics.md)                                                                  | 晨煦      | 2026-06-13 | ドラフト               |
| RFC-031  | [RFC-031：最適化レベルと Pass マネージャ](./draft/031-optimization-levels.md)                                                         | 晨煦      | 2026-06-16 | ドラフト               |
| RFC-002  | [RFC-002：libuv ベースのリソース型 IO 実装層](./draft/002-cross-platform-io-libuv.md)                                                 | 晨煦      | 2025-01-05 | ドラフト（再レビュー） |
| RFC-026b | [RFC-026b: yx-bindgen ツールチェーン](./draft/026b-yx-bindgen.md)                                                                     | 晨煦      | 2026-07-03 | ドラフト               |
| RFC-034  | [RFC-034: 統一デバッグツールチェーン](./draft/034-debug-toolchain.md)                                                                 | 晨煦      | 2026-07-06 | ドラフト               |
| RFC-035  | [RFC-035: MCP Server サポート（AI Agent 統合）](./draft/035-mcp-server.md)                                                            | Chen Xu   | 2026-07-11 | ドラフト               |
| RFC-036  | [RFC-036: std.test テストフレームワークと yaoxiang test コマンド](./draft/036-test-framework.md)                                      | 晨煦      | 2026-07-25 | ドラフト               |
| RFC-037  | [RFC-037: 産業的配布ソリューション — cargo-dist ベースのコンパイラ/ツールチェーンパッケージング](./draft/037-industrial-packaging.md) | ChenXu233 | 2026-07-26 | ドラフト               |

---

## レビュー中 RFC

| 番号     | タイトル                                                                                           | 著者 | 作成日     | ステータス |
| -------- | -------------------------------------------------------------------------------------------------- | ---- | ---------- | ---------- |
| RFC-026a | [RFC-026a: 拡張可能 FFI メカニズム体系](./review/026a-extensible-ffi-system.md)                    | 晨煦 | 2026-07-03 | レビュー中 |
| RFC-032  | [RFC-032: spawn 統一式修飾 — spawn for 特殊ケースの解消](./review/032-spawn-unified-expression.md) | 晨煦 | 2026-06-16 | レビュー中 |

---

## 採用済み RFC

| 番号       | タイトル                                                                                                        | 著者    | 作成日     | ステータス |
| ---------- | --------------------------------------------------------------------------------------------------------------- | ------- | ---------- | ---------- |
| RFC-004    | [RFC-004: カリー化メソッドの複数位置ユニオン束縛設計](./accepted/004-curry-multi-position-binding.md)           | 晨煦    | 2025-01-05 | 採用済み   |
| RFC-006    | [RFC-006: ドキュメントサイト構築](./accepted/006-documentation-site-optimization.md)                            | 晨煦    | 2025-01-05 | 採用済み   |
| RFC-007    | [RFC-007: 関数定義構文統一案](./accepted/007-function-syntax-unification.md)                                    | 沫郁酱  | 2025-01-05 | 採用済み   |
| RFC-008    | [RFC-008：Runtime 並行性モデルとスケジューラの疎結合設計](./accepted/008-runtime-concurrency-model.md)          | 晨煦    | 2025-01-05 | 採用済み   |
| RFC-009    | [RFC-009: ownership モデル設計](./accepted/009-ownership-model.md)                                              | 晨煦    | 2025-01-08 | 採用済み   |
| ↳ RFC-009a | [RFC-009a: トークンライフタイム解析 — ホーア証明パイプラインに基づく](./accepted/009a-borrow-proof-pipeline.md) | 晨煦    | 2026-06-13 | 採用済み   |
| RFC-010    | [RFC-010: 統一型構文 - name: type = value モデル](./accepted/010-unified-type-syntax.md)                        | 晨煦    | 2025-01-20 | 採用済み   |
| RFC-011    | [RFC-011: generics システム設計 - ゼロコスト抽象とマクロ代替](./accepted/011-generic-type-system.md)            | 晨煦    | 2025-01-25 | 採用済み   |
| ↳ RFC-011a | [RFC-011a: インターフェース実装と動的ディスパッチ](./review/011a-interface-implementation.md)                   | 晨煦    | 2026-06-14 | レビュー中 |
| RFC-012    | [RFC 012: F-String テンプレート文字列](./accepted/012-f-string-template-strings.md)                             | Chen Xu | 2025-01-27 | 採用済み   |
| RFC-013    | [RFC 013: エラーコード規範](./accepted/013-error-code-specification.md)                                         | 晨煦    | 2026-02-02 | 採用済み   |
| RFC-014    | [RFC-014: パッケージ管理システム設計](./accepted/014-package-manager.md)                                        | 晨煦    | 2026-02-12 | 採用済み   |
| ↳ RFC-014a | [RFC-014a: Registry プロトコル規範](./review/014a-registry-protocol.md)                                         | 晨煦    | 2026-06-11 | レビュー中 |
| ↳ RFC-014b | [RFC-014b: ビルドシステムとバイナリ配布](./review/014b-build-system.md)                                         | 晨煦    | 2026-06-11 | レビュー中 |
| ↳ RFC-014c | [RFC-014c: ワークスペースサポート](./review/014c-workspace.md)                                                  | 晨煦    | 2026-06-11 | レビュー中 |
| RFC-015    | [RFC-015: YaoXiang 設定システム設計](./accepted/015-configuration-system.md)                                    | 晨煦    | 2026-02-12 | 採用済み   |
| RFC-017    | [RFC-017: 言語サーバープロトコル（LSP）サポート設計](./accepted/017-lsp-support.md)                             | 晨煦    | 2026-02-15 | レビュー中 |
| RFC-018    | [RFC-018：LLVM AOT コンパイラ設計](./accepted/018-llvm-aot-compiler.md)                                         | 晨煦    | 2026-02-15 | 採用済み   |
| RFC-024    | [RFC-024：spawn ブロックベースの並行性モデル](./accepted/024-concurrency-model.md)                              | 晨煦    | 2026-06-05 | 採用済み   |
| RFC-026    | [RFC-026: FFI コアメカニズム](./accepted/026-ffi-core-mechanism.md)                                             | 晨煦    | 2026-06-05 | 採用済み   |
| RFC-027    | [RFC-027：compile-time 述語と統一静的検証](./accepted/027-compile-time-evaluation-types.md)                     | 晨煦    | 2026-06-07 | 採用済み   |
| RFC-030    | [RFC-030: assert メカニズム](./accepted/030-assert-mechanism.md)                                                | 晨煦    | 2026-06-15 | 採用済み   |

---

## 廃止 RFC

| 番号    | タイトル                                                                                                           | 著者 | 作成日     | ステータス                 |
| ------- | ------------------------------------------------------------------------------------------------------------------ | ---- | ---------- | -------------------------- |
| RFC-001 | [RFC-001：spawn モデルとエラーハンドリングシステム](./deprecated/001-concurrent-model-error-handling.md)           | 晨煦 | 2025-01-05 | 廃止（RFC-024 に置き換え） |
| RFC-020 | [RFC-020：動的モジュールと FFI 統合](./deprecated/020-dynamic-modules-ffi.md)                                      | 晨煦 | 2026-03-14 | 廃止                       |
| RFC-021 | [RFC-021: ライブラリ駆動 FFI 拡張とクロス言語呼び出しサポート](./deprecated/021-library-driven-ffi-extension.md)   | 晨煦 | 2026-03-14 | 廃止                       |
| RFC-022 | [RFC 022: ホーアロジック静的検証サポート（仕様注釈と仕様型）](./deprecated/022-hoare-logic-static-verification.md) | 晨煦 | 2026-03-16 | 廃止（RFC-027 に置き換え） |
| RFC-023 | [RFC-023: クロージャキャプチャモデル](./deprecated/023-closure-capture-model.md)                                   | 晨煦 | 2026-05-29 | 廃止                       |

---

## 却下 RFC

| 番号    | タイトル                                                                                            | 著者 | 作成日     | ステータス                               |
| ------- | --------------------------------------------------------------------------------------------------- | ---- | ---------- | ---------------------------------------- |
| RFC-003 | [RFC-003：バージョン計画](./rejected/003-version-planning.md)                                       | 晨煦 | 2025-01-05 | 却下                                     |
| RFC-005 | [RFC-005: 自動 CVE セキュリティチェックシステム](./rejected/005-automated-cve-scanning.md)          | 晨煦 | 2025-01-05 | 却下                                     |
| RFC-016 | [RFC 016: 量子ネイティブサポートとマルチバックエンド統合](./rejected/016-quantum-native-support.md) | 晨煦 | 2026-02-13 | 却下                                     |
| RFC-025 | [RFC-025: 拡張可能プリミティブ型メカニズム](./rejected/025-primitive-extension.md)                  | 晨煦 | 2026-06-05 | 却下（RFC-026 不透明ハンドルにより対応） |

---

## RFC ライフサイクル

```
ドラフト → レビュー中 → 採用済み → 廃止（置き換え）
                          ↓
                       却下（不承認）
```

### ステータス説明

| ステータス     | 場所          | 説明                                       |
| -------------- | ------------- | ------------------------------------------ |
| **ドラフト**   | `rfc/draft/`  | 著者のドラフト、レビュー提出待ち           |
| **レビュー中** | `rfc/review/` | コミュニティでの議論とフィードバック募集中 |

| **廃止** | `rfc/deprecated/` | 過去に採用されたが、新しい設計に置き換えられた | | **却下** |
`rfc/rejected/` | 却下された RFC ドキュメント |

---

## RFC の提出

1. [RFC_TEMPLATE.md](RFC_TEMPLATE.md) を読んでフォーマット要件を確認する
2. [EXAMPLE_full_feature_proposal.md](EXAMPLE_full_feature_proposal.md) を参考に書き方を学ぶ
3. 新規ファイルを作成し、`番号-説明的なタイトル.md` という名前を付ける
4. ファイルを `docs/src/design/rfc/draft/` ディレクトリに配置する
5. 本インデックスファイルを更新して新しい RFC エントリを追加する
6. PR を提出してレビュープロセスに進む

---

## 貢献ガイドライン

貢献ガイドラインについては [CONTRIBUTING.md](../../../../CONTRIBUTING.md) を参照してください。
