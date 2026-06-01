---
title: "RFC インデックス"
---

# YaoXiang RFC（リクエスト・フォー・コメント）インデックス

> RFC（Request for Comments）は、YaoXiang言語の特性設計提案の正式な提出フォーマットです。

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
| [EXAMPLE_full_feature_proposal.md](EXAMPLE_full_feature_proposal.md) | 完全な示例（パターン照合の拡張） |

---

## 草案RFC

| 番号 | タイトル | 著者 | 作成日 | 状態 |
|------|------|------|----------|------|
| RFC-016 | [量子ネイティブサポートとマルチバックエンド統合](./draft/016-quantum-native-support.md) | 晨煦 | 2026-02-12 | 草案 |
| RFC-019 | [タイプレベル同図性 (Typed Homoiconicity)](./draft/019-typed-homoiconicity.md) | 晨煦 | 2026-02-20 | 永久草案 ⚠️ |
| RFC-020 | [動的モジュール、FFI統合とコンテキスト感知スケジューリング拡張](./draft/020-dynamic-modules-ffi.md) | 晨煦 | 2026-02-25 | 草案 |

---

## 審査中RFC

| 番号 | タイトル | 著者 | 作成日 | 状態 |
|------|------|------|----------|------|
| RFC-003 | [バージョン計画と実装提案](./review/003-version-planning.md) | 晨煦 | 2025-01-05 | 審査中 |
| RFC-018 | [LLVM AOTコンパイラとランタイムスケジューラ統合設計](./review/018-llvm-aot-compiler.md) | 晨煦 | 2026-02-15 | 審査中 |
| RFC-021 | [ライブラリ駆動FFI拡張と跨言語呼び出しサポート](./review/021-library-driven-ffi-extension.md) | 晨煦 | 2026-03-14 | 審査中 |
| RFC-022 | [オプションのホーア論理静的検証（仕様注釈と仕様タイプ）](./review/022-hoare-logic-static-verification.md) | 晨煦 | 2026-03-16 | 審査中 |

---

## 承認済みRFC

| 番号 | タイトル | 著者 | 作成日 | 状態 |
|------|------|------|----------|------|
| RFC-001 | [並作モデルとエラー処理システム](./accepted/001-concurrent-model-error-handling.md) | 晨煦 | 2025-01-05 | 承認済み |
| RFC-004 | [カリー化メソッドのマルチポジション共同バインディング設計](./accepted/004-curry-multi-position-binding.md) | 晨煦 | 2025-01-05 | 承認済み |
| RFC-006 | [ドキュメントサイト構築と最適化方案](./accepted/006-documentation-site-optimization.md) | 晨煦 | 2025-01-05 | 承認済み |
| RFC-007 | [関数定義構文統一方案](./accepted/007-function-syntax-unification.md) | 晨煦 | 2025-01-05 | 承認済み |
| RFC-008 | [Runtime並発モデルとスケジューラ分離設計](./accepted/008-runtime-concurrency-model.md) | 晨煦 | 2025-01-05 | 承認済み |
| RFC-009 | [所有権モデル v7](./accepted/009-ownership-model.md) | 晨煦 | 2025-01-05 | 承認済み |
| RFC-011 | [ジェネリックシステム設計 - ゼロコスト抽象化とマクロ代替](./accepted/011-generic-type-system.md) | 晨煦 | 2025-01-25 | 承認済み |
| RFC-012 | [F-Stringテンプレート文字列](./accepted/012-f-string-template-strings.md) | 晨煦 | 2025-01-27 | 承認済み |
| RFC-013 | [エラーコード仕様設計](./accepted/013-error-code-specification.md) | 晨煦 | 2025-01-30 | 承認済み |
| RFC-014 | [パッケージ管理システム設計](./accepted/014-package-manager.md) | 晨煦 | 2026-02-12 | 承認済み |
| RFC-015 | [YaoXiang設定システム設計](./accepted/015-configuration-system.md) | 晨煦 | 2026-02-12 | 承認済み |
| RFC-017 | [言語サーバプロトコル（LSP）サポート設計](./review/017-lsp-support.md) | 晨煦 | 2026-02-15 | 承認済み |
| RFC-023 | [クロージャ捕獲モデル](./accepted/023-closure-capture-model.md) | 晨煦 | 2026-05-29 | 承認済み |


---

## 拒否済みRFC

| 番号 | タイトル | 著者 | 作成日 | 状態 |
|------|------|------|----------|------|
| RFC-002 | [跨プラットフォームI/Oとlibuv統合](./rejected/002-cross-platform-io-libuv.md) | 晨煦 | 2025-01-05 | 拒否済み |
| RFC-005 | [自動化CVEセキュリティ検査システム](./rejected/005-automated-cve-scanning.md) | 晨煦 | 2025-01-05 | 拒否済み |

---

## RFCライフサイクル

```
┌─────────────┐
│   草案      │  ←  著者が作成
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  審査中     │  ←  コミュニティでの議論とフィードバックを募集中
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

### 状態の説明

| 状態 | 場所 | 説明 |
|------|------|------|
| **草案** | `docs/reference/rfc/draft/` | 著者の草稿。審査への提出待ち |
| **審査中** | `docs/reference/rfc/review/` | コミュニティでの議論とフィードバックを募集中 |
| **承認済み** | `docs/reference/rfc/accepted/` | 正式な設計ドキュメントとなり、実装段階に入る |
| **拒否済み** | `docs/reference/rfc/rejected/` | 拒否されたRFCドキュメント |

---

## RFCの提交

1. [RFC_TEMPLATE.md](RFC_TEMPLATE.md) を読んでフォーマット要件を理解する
2. [EXAMPLE_full_feature_proposal.md](EXAMPLE_full_feature_proposal.md) を參考に書き方を学ぶ
3. 新規ファイルを作成し、`番号-記述的タイトル.md` と命名する
4. `docs/reference/rfc/draft/` ディレクトリにファイルを配置する
5. このインデックスファイルを更新して、新しいRFCエントリを追加する
6. 審査プロセスに入るためにPRを提交する

---

## 寄稿ガイドライン

寄稿ガイドラインについては [CONTRIBUTING.md](../../../../CONTRIBUTING.md) を参照してください。