```markdown
---
title: RFC インデックス
---

# YaoXiang RFC（リクエストフォコメント）インデックス

> RFC（Request for Comments）は、YaoXiang言語の機能設計提案の正式な提出フォーマットです。

## 目次

- [テンプレート](#テンプレート)
- [下書きRFC](#下書きrfc)
- [レビュー中RFC](#レビュー中rfc)
- [採用済みRFC](#採用済みrfc)
- [却下済みRFC](#却下済みrfc)

---

## テンプレート

| ファイル | 説明 |
|------|------|
| [RFC_TEMPLATE.md](RFC_TEMPLATE.md) | RFC標準テンプレート |
| [EXAMPLE_full_feature_proposal.md](EXAMPLE_full_feature_proposal.md) | 完全示例（パターン照合の強化） |

---

## 下書きRFC

| 番号 | タイトル | 著者 | 作成日 | 状態 |
|------|------|------|----------|------|
| RFC-016 | [量子ネイティブサポートとマルチバックエンド統合](./draft/016-quantum-native-support.md) | 晨煦 | 2026-02-12 | 下書き |
| RFC-018 | [LLVM AOT コンパイラとランタイムスケジューラ統合設計](./draft/018-llvm-aot-compiler.md) | 晨煦 | 2026-02-15 | 下書き |
| RFC-019 | [型レベル同図性 (Typed Homoiconicity)](./draft/019-typed-homoiconicity.md) | 晨煦 | 2026-02-20 | 永久下書き ⚠️ |
| RFC-020 | [動的モジュール、FFI統合とコンテキスト認識スケジューリング強化](./draft/020-dynamic-modules-ffi.md) | 晨煦 | 2026-02-25 | 下書き |
| RFC-021 | [ライブラリ駆動型FFI拡張とクロス言語呼び出しサポート](./draft/021-library-driven-ffi-extension.md) | 晨煦 | 2026-03-14 | 下書き |
| RFC-022 | [オプションのホア論理静的検証（仕様コメントと仕様型）](./draft/022-hoare-logic-static-verification.md) | 晨煦 | 2026-03-16 | 下書き |

---

## レビュー中RFC

| 番号 | タイトル | 著者 | 作成日 | 状態 |
|------|------|------|----------|------|
| RFC-003 | [バージョニング計画と実装提案](./review/003-version-planning.md) | 晨煦 | 2025-01-05 | レビュー中 |

---

## 採用済みRFC

| 番号 | タイトル | 著者 | 作成日 | 状態 |
|------|------|------|----------|------|
| RFC-001 | [スポーンモデルとエラー処理システム](./accepted/001-concurrent-model-error-handling.md) | 晨煦 | 2025-01-05 | 採用済み |
| RFC-004 | [カリー化メソッドのマルチポジション共用結合設計](./accepted/004-curry-multi-position-binding.md) | 晨煦 | 2025-01-05 | 採用済み |
| RFC-006 | [ドキュメンテーションサイト構築と最適化方案](./accepted/006-documentation-site-optimization.md) | 晨煦 | 2025-01-05 | 採用済み |
| RFC-007 | [関数定義構文統一方案](./accepted/007-function-syntax-unification.md) | 晨煦 | 2025-01-05 | 採用済み |
| RFC-008 | [Runtime 並行モデルとスケジューラデカップ設計](./accepted/008-runtime-concurrency-model.md) | 晨煦 | 2025-01-05 | 採用済み |
| RFC-009 | [所有権モデル v7](./accepted/009-ownership-model.md) | 晨煦 | 2025-01-05 | 採用済み |
| RFC-011 | [ジェネリクスシステム設計 - ゼロコスト抽象化とマクロ代替](./accepted/011-generic-type-system.md) | 晨煦 | 2025-01-25 | 採用済み |
| RFC-012 | [F-String テンプレート文字列](./accepted/012-f-string-template-strings.md) | 晨煦 | 2025-01-27 | 採用済み |
| RFC-013 | [エラーコード仕様設計](./accepted/013-error-code-specification.md) | 晨煦 | 2025-01-30 | 採用済み |
| RFC-014 | [パッケージ管理システム設計](./accepted/014-package-manager.md) | 晨煦 | 2026-02-12 | 採用済み |
| RFC-015 | [YaoXiang 設定システム設計](./accepted/015-configuration-system.md) | 晨煦 | 2026-02-12 | 採用済み |
| RFC-017 | [言語サーバプロトコル（LSP）サポート設計](./review/017-lsp-support.md) | 晨煦 | 2026-02-15 | 採用済み |


---

## 却下済みRFC

| 番号 | タイトル | 著者 | 作成日 | 状態 |
|------|------|------|----------|------|
| RFC-002 | [クロスプラットフォームI/Oとlibuv統合](./rejected/002-cross-platform-io-libuv.md) | 晨煦 | 2025-01-05 | 却下済み |
| RFC-005 | [自動化されたCVEセキュリティ検査システム](./rejected/005-automated-cve-scanning.md) | 晨煦 | 2025-01-05 | 却下済み |

---

## RFCライフサイクル

```
┌─────────────┐
│   下書き    │  ← 作成者が作成
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  レビュー中 │  ← コミュニティ議論とフィードバックを開放
└──────┬──────┘
       │
       ├──────────────────┐
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│  採用済み   │    │  却下済み   │
└──────┬──────┘    └──────┬──────┘
       │                  │
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│  accepted/  │    │  rejected/  │
│ (正式設計)  │    │ (却下)      │
└─────────────┘    └─────────────┘
```

### 状態説明

| 状態 | 場所 | 説明 |
|------|------|------|
| **下書き** | `docs/reference/rfc/draft/` | 作成者の下書き、レビュー提出待ち |
| **レビュー中** | `docs/reference/rfc/review/` | コミュニティ議論とフィードバックを開放 |
| **採用済み** | `docs/reference/rfc/accepted/` | 正式設計ドキュメントとなり、実装段階へ |
| **却下済み** | `docs/reference/rfc/rejected/` | 却下されたRFCドキュメント |

---

## RFCの提交

1. [RFC_TEMPLATE.md](RFC_TEMPLATE.md) を読んでフォーマット要件を理解する
2. [EXAMPLE_full_feature_proposal.md](EXAMPLE_full_feature_proposal.md) を参考にする
3. 新規ファイルを作成、`番号-記述的タイトル.md` と命名する
4. `docs/reference/rfc/draft/` ディレクトリにファイルを配置する
5. このインデックスファイルを更新し、新しいRFCエントリを追加する
6. PRを提交してレビュー流程に入る

---

## 貢献ガイド

貢献ガイドについては [CONTRIBUTING.md](../../../../CONTRIBUTING.md) をご覧ください。
```