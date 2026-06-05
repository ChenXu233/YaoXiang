```markdown
---
title: "RFC インデックス"
---

# YaoXiang RFC（Request for Comments）インデックス

> RFC（Request for Comments）は、YaoXiangの言語機能設計提案の正式な提出フォーマットです。

## 目次

- [テンプレート](#テンプレート)
- [草案RFC](#草案rfc)
- [審査中RFC](#審査中rfc)
- [承認済みRFC](#承認済みrfc)
- [廃止RFC](#廃止rfc)
- [拒否RFC](#拒否rfc)

---

## テンプレート

| ファイル | 説明 |
|------|------|
| [RFC_TEMPLATE.md](RFC_TEMPLATE.md) | RFC標準テンプレート |
| [EXAMPLE_full_feature_proposal.md](EXAMPLE_full_feature_proposal.md) | 完全示例（パターン照合の強化） |

---

## 草案RFC

| 番号 | タイトル | 著者 | 作成日 | 状態 |
|------|------|----------|----------|------|
| RFC-019 | [型レベル同像性 (Typed Homoiconicity)](./draft/019-typed-homoiconicity.md) | 晨煦 | 2026-02-20 | 永久草案 ⚠️ |
| RFC-020 | [動的モジュール、FFI統合とコンテキスト感知スケジューリングの強化](./draft/020-dynamic-modules-ffi.md) | 晨煦 | 2026-02-25 | 草案 |
| RFC-025 | [拡張可能な原語型メカニズム](./draft/025-primitive-extension.md) | 晨煦 | 2026-06-05 | 草案 |

---

## 審査中RFC

| 番号 | タイトル | 著者 | 作成日 | 状態 |
|------|------|----------|----------|------|
| RFC-018 | [LLVM AOTコンパイラとランタイムスケジューラ統合設計](./review/018-llvm-aot-compiler.md) | 晨煦 | 2026-02-15 | 審査中 |
| RFC-021 | [ライブラリ駆動FFI拡張と異言語間呼び出しサポート](./review/021-library-driven-ffi-extension.md) | 晨煦 | 2026-03-14 | 審査中 |
| RFC-022 | [オプションのホーア論理静的検証（仕様コメントと仕様型）](./review/022-hoare-logic-static-verification.md) | 晨煦 | 2026-03-16 | 審査中 |

---

## 承認済みRFC

| 番号 | タイトル | 著者 | 作成日 | 状態 |
|------|------|----------|----------|------|
| RFC-001 | [並作モデルとエラー処理システム](./accepted/001-concurrent-model-error-handling.md) | 晨煦 | 2025-01-05 | 承認済み |
| RFC-004 | [カリー化メソッドの複数位置統合束縛設計](./accepted/004-curry-multi-position-binding.md) | 晨煦 | 2025-01-05 | 承認済み |
| RFC-006 | [ドキュメントサイト構築と最適化方案](./accepted/006-documentation-site-optimization.md) | 晨煦 | 2025-01-05 | 承認済み |
| RFC-007 | [関数定義構文統一方案](./accepted/007-function-syntax-unification.md) | 晨煦 | 2025-01-05 | 承認済み |
| RFC-008 | [Runtime並行モデルとスケジューラ分離設計](./accepted/008-runtime-concurrency-model.md) | 晨煦 | 2025-01-05 | 承認済み |
| RFC-009 | [所有権モデル v7](./accepted/009-ownership-model.md) | 晨煦 | 2025-01-05 | 承認済み |
| RFC-010 | [統一型構文](./accepted/010-unified-type-syntax.md) | 晨煦 | 2025-01-25 | 承認済み |
| RFC-011 | [泛型システム設計 - ゼロコスト抽象化とマクロ代替](./accepted/011-generic-type-system.md) | 晨煦 | 2025-01-25 | 承認済み |
| RFC-012 | [F-Stringテンプレート文字列](./accepted/012-f-string-template-strings.md) | 晨煦 | 2025-01-27 | 承認済み |
| RFC-013 | [エラーコード仕様設計](./accepted/013-error-code-specification.md) | 晨煦 | 2025-01-30 | 承認済み |
| RFC-014 | [パッケージ管理システム設計](./accepted/014-package-manager.md) | 晨煦 | 2026-02-12 | 承認済み |
| RFC-015 | [YaoXiang設定システム設計](./accepted/015-configuration-system.md) | 晨煦 | 2026-02-12 | 承認済み |
| RFC-017 | [言語サーバプロトコル（LSP）サポート設計](./accepted/017-lsp-support.md) | 晨煦 | 2026-02-15 | 承認済み |
| RFC-023 | [クロージャ捕獲モデル](./accepted/023-closure-capture-model.md) | 晨煦 | 2026-05-29 | 承認済み |
| RFC-024 | [spawn ブロックベースの並行処理モデル](./accepted/024-concurrency-model.md) | 晨煦 | 2026-06-05 | 承認済み |


---

## 廃止RFC

| 番号 | タイトル | 著者 | 作成日 | 状態 |
|------|------|----------|----------|------|
| （なし） | | | | |

---

## 拒否RFC

| 番号 | タイトル | 著者 | 作成日 | 状態 |
|------|------|----------|----------|------|
| RFC-002 | [クロスプラットフォームI/Oとlibuv統合](./rejected/002-cross-platform-io-libuv.md) | 晨煦 | 2025-01-05 | 拒否済み |
| RFC-003 | [バージョン計画と実装提案](./rejected/003-version-planning.md) | 晨煦 | 2025-01-05 | 拒否済み |
| RFC-005 | [自動CVEセキュリティ検査システム](./rejected/005-automated-cve-scanning.md) | 晨煦 | 2025-01-05 | 拒否済み |
| RFC-016 | [量子ネイティブサポートとマルチバックエンド統合](./rejected/016-quantum-native-support.md) | 晨煦 | 2026-02-13 | 拒否済み |

---

## RFCライフサイクル

```
草案 → 審査中 → 承認済み → 廃止（置換）
                  ↓
               拒否（不採用）
```

### 状態の説明

| 状態 | 場所 | 説明 |
|------|------|------|
| **草案** | `rfc/draft/` | 著者の下書き、査読提出待ち |
| **審査中** | `rfc/review/` | コミュニティの議論とフィードバック受付 |
| **承認済み** | `rfc/accepted/` | 正式な設計文書となり、実装段階へ |
| **廃止** | `rfc/deprecated/` | かつて承認されたが、新しい設計にを置換 |
| **拒否** | `rfc/rejected/` | 拒否されたRFC文書 |

---

## RFCの提交

1. [RFC_TEMPLATE.md](RFC_TEMPLATE.md) を読んでフォーマット要件を理解する
2. [EXAMPLE_full_feature_proposal.md](EXAMPLE_full_feature_proposal.md) を参照して書き方を学ぶ
3. 新規ファイルを作成し、`番号-記述的タイトル.md` と命名する
4. ファイルを `docs/reference/rfc/draft/` ディレクトリに配置する
5. 本インデックスファイルを更新し、新しいRFCエントリを追加する
6. 査読プロセスにPRを提交する

---

## 貢献ガイド

貢献ガイドについては、[CONTRIBUTING.md](../../../../CONTRIBUTING.md) を参照してください。
```