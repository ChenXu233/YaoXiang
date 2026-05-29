---
title: RFC インデックス
---

# YaoXiang RFC（リクエスト・コメント）インデックス

> RFC（Request for Comments）は、YaoXiang言語の機能設計提案の正式な提出フォーマットです。

## 目次

- [テンプレート](#テンプレート)
- [草案RFC](#草案rfc)
- [審査中RFC](#審査中rfc)
- [承認済みRFC](#承認済みrfc)
- [拒否済みRFC](#拒否済みrfc)

---

## テンプレート

| ファイル | 説明 |
|------|------|
| [RFC_TEMPLATE.md](RFC_TEMPLATE.md) | RFC標準テンプレート |
| [EXAMPLE_full_feature_proposal.md](EXAMPLE_full_feature_proposal.md) | 完全示例（パターン照合の拡張） |

---

## 草案RFC

| 番号 | タイトル | 著者 | 作成日 | ステータス |
|------|------|------|----------|------|
| RFC-016 | [量子ネイティブサポートとマルチバックエンド統合](./draft/016-quantum-native-support.md) | 晨煦 | 2026-02-12 | 草案 |
| RFC-018 | [LLVM AOT コンパイラとランタイムスケジューラ統合設計](./draft/018-llvm-aot-compiler.md) | 晨煦 | 2026-02-15 | 草案 |
| RFC-019 | [型レベル同像性 (Typed Homoiconicity)](./draft/019-typed-homoiconicity.md) | 晨煦 | 2026-02-20 | 永久草案 ⚠️ |
| RFC-020 | [動的モジュール、FFI 統合とコンテキスト感知スケジューリング強化](./draft/020-dynamic-modules-ffi.md) | 晨煦 | 2026-02-25 | 草案 |
| RFC-021 | [ライブラリ駆動型FFI拡張と異言語間呼び出しサポート](./draft/021-library-driven-ffi-extension.md) | 晨煦 | 2026-03-14 | 草案 |
| RFC-022 | [オプションのホーア論理静的検証（仕様注釈と仕様型）](./draft/022-hoare-logic-static-verification.md) | 晨煦 | 2026-03-16 | 草案 |

---

## 審査中RFC

| 番号 | タイトル | 著者 | 作成日 | ステータス |
|------|------|------|----------|------|
| RFC-003 | [バージョニング計画と実装提案](./review/003-version-planning.md) | 晨煦 | 2025-01-05 | 審査中 |

---

## 承認済みRFC

| 番号 | タイトル | 著者 | 作成日 | ステータス |
|------|------|------|----------|------|
| RFC-001 | [並作モデルとエラー処理システム](./accepted/001-concurrent-model-error-handling.md) | 晨煦 | 2025-01-05 | 承認済み |
| RFC-004 | [カリー化メソッドの位置自由的共同束縛設計](./accepted/004-curry-multi-position-binding.md) | 晨煦 | 2025-01-05 | 承認済み |
| RFC-006 | [ドキュメントサイト構築と最適化方案](./accepted/006-documentation-site-optimization.md) | 晨煦 | 2025-01-05 | 承認済み |
| RFC-007 | [関数定義構文統一方案](./accepted/007-function-syntax-unification.md) | 晨煦 | 2025-01-05 | 承認済み |
| RFC-008 | [Runtime 並行モデルとスケジューラ分離設計](./accepted/008-runtime-concurrency-model.md) | 晨煦 | 2025-01-05 | 承認済み |
| RFC-009 | [所有権モデル v7](./accepted/009-ownership-model.md) | 晨煦 | 2025-01-05 | 承認済み |
| RFC-011 | [汎用システム設計 - ゼロコスト抽象化とマクロ代替](./accepted/011-generic-type-system.md) | 晨煦 | 2025-01-25 | 承認済み |
| RFC-012 | [F-String テンプレート文字列](./accepted/012-f-string-template-strings.md) | 晨煦 | 2025-01-27 | 承認済み |
| RFC-013 | [エラーコード仕様設計](./accepted/013-error-code-specification.md) | 晨煦 | 2025-01-30 | 承認済み |
| RFC-014 | [パッケージ管理システム設計](./accepted/014-package-manager.md) | 晨煦 | 2026-02-12 | 承認済み |
| RFC-015 | [YaoXiang 設定システム設計](./accepted/015-configuration-system.md) | 晨煦 | 2026-02-12 | 承認済み |
| RFC-017 | [言語サーバープロトコル（LSP）サポート設計](./review/017-lsp-support.md) | 晨煦 | 2026-02-15 | 承認済み |


---

## 拒否済みRFC

| 番号 | タイトル | 著者 | 作成日 | ステータス |
|------|------|------|----------|------|
| RFC-002 | [クロスプラットフォームI/Oとlibuv統合](./rejected/002-cross-platform-io-libuv.md) | 晨煦 | 2025-01-05 | 拒否済み |
| RFC-005 | [自動化CVEセキュリティスキャンシステム](./rejected/005-automated-cve-scanning.md) | 晨煦 | 2025-01-05 | 拒否済み |

---

## RFCライフサイクル

```
┌─────────────┐
│   草案      │  ← 作成者が作成
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  審査中     │  ← コミュニティでの議論とフィードバックを開始
└──────┬──────┘
       │
       ├──────────────────┐
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│  承認済み   │    │  拒否済み   │
└──────┬──────┘    └──────┬──────┘
       │                  │
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│  accepted/  │    │  rejected/  │
│ (正式設計)  │    │ (拒否)      │
└─────────────┘    └─────────────┘
```

### ステータス説明

| ステータス | 場所 | 説明 |
|------|------|------|
| **草案** | `docs/reference/rfc/draft/` | 作成者の草稿、審査待ち |
| **審査中** | `docs/reference/rfc/review/` | コミュニティでの議論とフィードバックを開始 |
| **承認済み** | `docs/reference/rfc/accepted/` | 正式設計文書となり、実装段階に入る |
| **拒否済み** | `docs/reference/rfc/rejected/` | 拒否されたRFC文書 |

---

## RFCの提出

1. [RFC_TEMPLATE.md](RFC_TEMPLATE.md) を読んで形式要件を理解する
2. [EXAMPLE_full_feature_proposal.md](EXAMPLE_full_feature_proposal.md) を参照して書き方を学ぶ
3. 新規ファイルを作成し、名前を `番号-記述的タイトル.md` とする
4. ファイルを `docs/reference/rfc/draft/` ディレクトリに配置する
5. このインデックスファイルを更新して、新しいRFCエントリを追加する
6. PRを提出して審査プロセスに進む

---

## 寄稿ガイドライン

寄稿ガイドラインについては [CONTRIBUTING.md](../../../../CONTRIBUTING.md) を参照してください。