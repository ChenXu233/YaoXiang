---
title: "RFC インデックス"
---

# YaoXiang RFC（リクエスト・フォー・コメント）インデックス

> RFC（Request for Comments）は、YaoXiang言語の仕様設計提案の正式な提出フォーマットです。

## ディレクトリ

- [テンプレート](#テンプレート)
- [草案RFC](#草案rfc)
- [レビュー中RFC](#レビュー中rfc)
- [採用済みRFC](#採用済みrfc)
- [廃止RFC](#廃止rfc)
- [却下RFC](#却下rfc)

---

## テンプレート

| ファイル | 説明 |
|------|------|
| [RFC_TEMPLATE.md](RFC_TEMPLATE.md) | RFC標準テンプレート |
| [EXAMPLE_full_feature_proposal.md](EXAMPLE_full_feature_proposal.md) | 完全示例（パターンマッチングの強化） |

---

## 草案RFC

| 番号 | タイトル | 著者 | 作成日 | 状態 |
|------|------|------|----------|------|
| RFC-019 | [RFC-019: 型レベル同像性 (Typed Homoiconicity) - 構文は型である](./draft/019-typed-homoiconicity.md) | 晨煦 | 2026-02-20 | 草案 |
| RFC-025 | [RFC-025: 拡張可能な原語型メカニズム](./draft/025-primitive-extension.md) | 晨煦 | 2026-06-05 | 草案 |

---

## レビュー中RFC

| 番号 | タイトル | 著者 | 作成日 | 状態 |
|------|------|------|----------|------|
| RFC-018 | [RFC-018：LLVM AOT コンパイラ設計](./review/018-llvm-aot-compiler.md) | 晨煦 | 2026-02-15 | レビュー中 |
| RFC-026 | [RFC-026：FFI コアメカニズム](./review/026-ffi-core-mechanism.md) | 晨煦 | 2026-06-05 | レビュー中 |
| RFC-027 | [RFC-027：コンパイル時評価型と統合静的検証](./review/027-compile-time-evaluation-types.md) | 晨煦 | 2026-06-07 | レビュー中 |

---

## 採用済みRFC

| 番号 | タイトル | 著者 | 作成日 | 状態 |
|------|------|------|----------|------|
| RFC-004 | [RFC-004: カーリ化メソッドの位置間統合バインディング設計](./accepted/004-curry-multi-position-binding.md) | 晨煦 | 2025-01-05 | 採用済み |
| RFC-006 | [RFC-006: ドキュメントサイト構築](./accepted/006-documentation-site-optimization.md) | 晨煦 | 2025-01-05 | 採用済み |
| RFC-007 | [RFC-007: 関数定義構文の統一](./accepted/007-function-syntax-unification.md) | 沫郁酱 | 2025-01-05 | 採用済み |
| RFC-008 | [RFC-008：Runtime 並行モデルとスケジューラの疎結合設計](./accepted/008-runtime-concurrency-model.md) | 晨煦 | 2025-01-05 | 採用済み |
| RFC-009 | [RFC-009: 所有権モデル設計](./accepted/009-ownership-model.md) | 晨煦 | 2025-01-08 | 採用済み |
| RFC-010 | [RFC-010: 統合型構文 - name: type = value モデル](./accepted/010-unified-type-syntax.md) | 晨煦 | 2025-01-20 | 採用済み |
| RFC-011 | [RFC-011: ジェネリクス型システム設計 - ゼロコスト抽象化とマクロ代替](./accepted/011-generic-type-system.md) | 晨煦 | 2025-01-25 | 採用済み |
| RFC-012 | [RFC 012: F-String テンプレート文字列](./accepted/012-f-string-template-strings.md) | Chen Xu | 2025-01-27 | 採用済み |
| RFC-013 | [RFC 013: エラーコード仕様](./accepted/013-error-code-specification.md) | 晨煦 | 2026-02-02 | 採用済み |
| RFC-014 | [RFC-014: パッケージ管理システム設計](./accepted/014-package-manager.md) | 晨煦 | 2026-02-12 | 採用済み |
| RFC-015 | [RFC-015: YaoXiang 設定システム設計](./accepted/015-configuration-system.md) | 晨煦 | 2026-02-12 | 採用済み |
| RFC-017 | [RFC-017: Language Server Protocol（LSP）サポート設計](./accepted/017-lsp-support.md) | 晨煦 | 2026-02-15 | レビュー中 |
| RFC-023 | [RFC-023: クロージャ捕獲モデル](./accepted/023-closure-capture-model.md) | 晨煦 | 2026-05-29 | 採用済み |
| RFC-024 | [RFC-024：spawn ブロックベースの並行モデル](./accepted/024-concurrency-model.md) | 晨煦 | 2026-06-05 | 採用済み |

---

## 廃止RFC

| 番号 | タイトル | 著者 | 作成日 | 状態 |
|------|------|------|----------|------|
| RFC-001 | [RFC-001：並行モデルとエラー処理システム](./deprecated/001-concurrent-model-error-handling.md) | 晨煦 | 2025-01-05 | 廃止（RFC-024に置き換え） |
| RFC-020 | [RFC-020：動的モジュールとFFI統合](./deprecated/020-dynamic-modules-ffi.md) | 晨煦（コミュニティ議論に基づく整理） | 2026-03-14 | 廃止 |
| RFC-021 | [RFC-021: ライブラリ駆動型FFI拡張と異言語間呼び出しサポート](./deprecated/021-library-driven-ffi-extension.md) | 晨煦 | 2026-03-14 | 廃止 |
| RFC-022 | [RFC-022: ホア論理静的検証サポート（仕様コメントと仕様型）](./deprecated/022-hoare-logic-static-verification.md) | 晨煦 | 2026-03-16 | 廃止（RFC-027に置き換え） |

---

## 却下RFC

| 番号 | タイトル | 著者 | 作成日 | 状態 |
|------|------|------|----------|------|
| RFC-002 | [RFC-002：クロスプラットフォームI/Oとlibuv統合](./rejected/002-cross-platform-io-libuv.md) | 晨煦 | 2025-01-05 | 却下 |
| RFC-003 | [RFC-003：バージョニング計画](./rejected/003-version-planning.md) | 晨煦 | 2025-01-05 | 却下 |
| RFC-005 | [RFC-005: 自動化CVEセキュリティスキャンシステム](./rejected/005-automated-cve-scanning.md) | 晨煦 | 2025-01-05 | 却下 |
| RFC-016 | [RFC 016: 量子ネイティブサポートとマルチバックエンド統合](./rejected/016-quantum-native-support.md) | 晨煦 | 2026-02-13 | 却下 |

---

## RFCライフサイクル

```
草案 → レビュー中 → 採用済み → 廃止（置き換え）
                  ↓
               却下（不採用）
```

### 状態の説明

| 状態 | 場所 | 説明 |
|------|------|------|
| **草案** | `rfc/draft/` | 作成者による下書き、レビュー提出待ち |
| **レビュー中** | `rfc/review/` | コミュニティでの議論とフィードバックが開放中 |
| **採用済み** | `rfc/accepted/` | 正式な設計ドキュメントとなり、実装フェーズへ |
| **廃止** | `rfc/deprecated/` | かつて採用され、新しい設計に置き換えられた |
| **却下** | `rfc/rejected/` | 却下されたRFCドキュメント |

---

## RFCの提出

1. [RFC_TEMPLATE.md](RFC_TEMPLATE.md) を読んでフォーマット要件を理解する
2. [EXAMPLE_full_feature_proposal.md](EXAMPLE_full_feature_proposal.md) を参照して書き方を学ぶ
3. 新しいファイルを作成し、`番号-説明的なタイトル.md` という名前をつける
4. ファイルを `docs/reference/rfc/draft/` ディレクトリに配置する
5. このインデックスファイルを更新し、新しいRFCエントリを追加する
6. PRを提交してレビュー流程に進む

---

## コントリビューションガイド

コントリビューションガイドについては、[CONTRIBUTING.md](../../../../CONTRIBUTING.md) を参照してください。