```yaml
---
title: "RFC インデックス"
---

# YaoXiang RFC（Request for Comments）インデックス

> RFC（Request for Comments）は、YaoXiang言語の特性設計提案の正式な提出フォーマットです。

## 目次

- [テンプレート](#テンプレート)
- [ドラフトRFC](#ドラフトrfc)
- [レビュー中RFC](#レビュー中rfc)
- [承認済みRFC](#承認済みrfc)
- [非推奨RFC](#非推奨rfc)
- [却下RFC](#却下rfc)

---

## テンプレート

| ファイル | 説明 |
|------|------|
| [RFC_TEMPLATE.md](RFC_TEMPLATE.md) | RFC標準テンプレート |
| [EXAMPLE_full_feature_proposal.md](EXAMPLE_full_feature_proposal.md) | 完全なサンプル（pattern matching機能拡張） |

---

## ドラフトRFC

| 番号 | タイトル | 著者 | 作成日 | ステータス |
|------|------|------|----------|------|
| RFC-019 | [RFC-019: 型レベル同像性 (Typed Homoiconicity) - 構文即型](./draft/019-typed-homoiconicity.md) | 晨煦 | 2026-02-20 | ドラフト |
| RFC-028 | [RFC-028: JITコンパイラ — VM内多段実行エンジン](./draft/028-jit-compiler.md) | 晨煦 | 2026-06-11 | ドラフト |

---

## レビュー中RFC

| 番号 | タイトル | 著者 | 作成日 | ステータス |
|------|------|------|----------|------|
| RFC-025 | [RFC-025: 拡張可能primitive type機構](./review/025-primitive-extension.md) | 晨煦 | 2026-06-05 | レビュー中 |
| RFC-026 | [RFC-026：FFIコア機構](./review/026-ffi-core-mechanism.md) | 晨煦 | 2026-06-05 | レビュー中 |

---

## 承認済みRFC

| 番号 | タイトル | 著者 | 作成日 | ステータス |
|------|------|------|----------|------|
| RFC-004 | [RFC-004: カリー化メソッドの複数位置共同バインディング設計](./accepted/004-curry-multi-position-binding.md) | 晨煦 | 2025-01-05 | 承認済み |
| RFC-006 | [RFC-006: ドキュメントサイト構築](./accepted/006-documentation-site-optimization.md) | 晨煦 | 2025-01-05 | 承認済み |
| RFC-007 | [RFC-007: 関数定義構文統一方案](./accepted/007-function-syntax-unification.md) | 沫郁酱 | 2025-01-05 | 承認済み |
| RFC-008 | [RFC-008：Runtime並行モデルとスケジューラの疎結合設計](./accepted/008-runtime-concurrency-model.md) | 晨煦 | 2025-01-05 | 承認済み |
| RFC-009 | [RFC-009: ownershipモデル設計](./accepted/009-ownership-model.md) | 晨煦 | 2025-01-08 | 承認済み |
| RFC-010 | [RFC-010: 統一型構文 - name: type = valueモデル](./accepted/010-unified-type-syntax.md) | 晨煦 | 2025-01-20 | 承認済み |
| RFC-011 | [RFC-011: genericsシステム設計 - ゼロコスト抽象化とマクロ代替](./accepted/011-generic-type-system.md) | 晨煦 | 2025-01-25 | 承認済み |
| RFC-012 | [RFC 012: F-Stringテンプレート文字列](./accepted/012-f-string-template-strings.md) | Chen Xu | 2025-01-27 | 承認済み |
| RFC-013 | [RFC 013: エラーコード規範](./accepted/013-error-code-specification.md) | 晨煦 | 2026-02-02 | 承認済み |
| RFC-014 | [RFC-014: パッケージ管理システム設計](./accepted/014-package-manager.md) | 晨煦 | 2026-02-12 | 承認済み |
| RFC-015 | [RFC-015: YaoXiang設定システム設計](./accepted/015-configuration-system.md) | 晨煦 | 2026-02-12 | 承認済み |
| RFC-017 | [RFC-017: 言語サーバープロトコル（LSP）サポート設計](./accepted/017-lsp-support.md) | 晨煦 | 2026-02-15 | レビュー中 |
| RFC-018 | [RFC-018：LLVM AOTコンパイラ設計](./accepted/018-llvm-aot-compiler.md) | 晨煦 | 2026-02-15 | 承認済み |
| RFC-023 | [RFC-023: クロージャキャプチャモデル](./accepted/023-closure-capture-model.md) | 晨煦 | 2026-05-29 | 承認済み |
| RFC-024 | [RFC-024：spawnブロックベースの並行モデル](./accepted/024-concurrency-model.md) | 晨煦 | 2026-06-05 | 承認済み |
| RFC-027 | [RFC-027：compile-time述語と統一静的検証](./accepted/027-compile-time-evaluation-types.md) | 晨煦 | 2026-06-07 | 承認済み |

---

## 非推奨RFC

| 番号 | タイトル | 著者 | 作成日 | ステータス |
|------|------|------|----------|------|
| RFC-001 | [RFC-001：spawnモデルとエラーハンドリングシステム](./deprecated/001-concurrent-model-error-handling.md) | 晨煦 | 2025-01-05 | 非推奨（RFC-024に置き換え） |
| RFC-020 | [RFC-020：動的モジュールとFFI統合](./deprecated/020-dynamic-modules-ffi.md) | 晨煦 (コミュニティとの議論に基づく整理) | 2026-03-14 | 非推奨 |
| RFC-021 | [RFC-021: ライブラリ駆動FFI拡張と多言語呼び出しサポート](./deprecated/021-library-driven-ffi-extension.md) | 晨煦 | 2026-03-14 | 非推奨 |
| RFC-022 | [RFC 022: ホーア論理静的検証サポート（仕様注釈と仕様型）](./deprecated/022-hoare-logic-static-verification.md) | 晨煦 | 2026-03-16 | 非推奨（RFC-027に置き換え） |

---

## 却下RFC

| 番号 | タイトル | 著者 | 作成日 | ステータス |
|------|------|------|----------|------|
| RFC-002 | [RFC-002：クロスプラットフォームI/Oとlibuv統合](./rejected/002-cross-platform-io-libuv.md) | 晨煦 | 2025-01-05 | 却下 |
| RFC-003 | [RFC-003：バージョン計画](./rejected/003-version-planning.md) | 晨煦 | 2025-01-05 | 却下 |
| RFC-005 | [RFC-005: 自動CVEセキュリティチェックシステム](./rejected/005-automated-cve-scanning.md) | 晨煦 | 2025-01-05 | 却下 |
| RFC-016 | [RFC 016: 量子ネイティブサポートと多バックエンド統合](./rejected/016-quantum-native-support.md) | 晨煦 | 2026-02-13 | 却下 |

---

## RFCライフサイクル

```
ドラフト → レビュー中 → 承認済み → 非推奨（置き換え）
                  ↓
               却下（不承認）
```

### ステータス説明

| ステータス | 場所 | 説明 |
|------|------|------|
| **ドラフト** | `rfc/draft/` | 著者の草稿、レビュー提出待ち |
| **レビュー中** | `rfc/review/` | コミュニティでの議論とフィードバック募集中 |
| **承認済み** | `rfc/accepted/` | 正式な設計ドキュメントとなり、実装段階へ |
| **非推奨** | `rfc/deprecated/` | 一度承認されたが、新しい設計に置き換えられた |
| **却下** | `rfc/rejected/` | 却下されたRFCドキュメント |

---

## RFCの提出

1. [RFC_TEMPLATE.md](RFC_TEMPLATE.md) を読んでフォーマット要件を確認
2. [EXAMPLE_full_feature_proposal.md](EXAMPLE_full_feature_proposal.md) を参考に書き方を学ぶ
3. 新規ファイルを `番号-説明的なタイトル.md` という名前で作成
4. ファイルを `docs/reference/rfc/draft/` ディレクトリに配置
5. このインデックスファイルを更新し、新しいRFCエントリを追加
6. PRを提出してレビュープロセスに入る

---

## 貢献ガイドライン

貢献ガイドラインについては [CONTRIBUTING.md](../../../../CONTRIBUTING.md) を参照してください。
```