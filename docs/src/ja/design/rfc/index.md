---
title: "RFC インデックス"
---

# YaoXiang RFC（リクエスト・フォア・コメント）インデックス

> RFC（Request for Comments）は、YaoXiang言語の特性設計提案の正式な提出フォーマットです。

## 目次

- [テンプレート](#テンプレート)
- [草案RFC](#草案rfc)
- [審査中RFC](#審査中rfc)
- [採用済みRFC](#採用済みrfc)
- [廃止済みRFC](#廃止済みrfc)
- [拒否済みRFC](#拒否済みrfc)

---

## テンプレート

| ファイル | 説明 |
|------|------|
| [RFC_TEMPLATE.md](RFC_TEMPLATE.md) | RFC標準テンプレート |
| [EXAMPLE_full_feature_proposal.md](EXAMPLE_full_feature_proposal.md) | 完全な例（パターン照合強化） |

---

## 草案RFC

| 番号 | タイトル | 著者 | 作成日 | 状態 |
|------|------|------|----------|------|
| RFC-019 | [RFC-019: 型レベル同像性 (Typed Homoiconicity) - 構文即ち型](./draft/019-typed-homoiconicity.md) | 晨煦 | 2026-02-20 | 草案 |
| RFC-025 | [RFC-025: 拡張可能な原語型メカニズム](./draft/025-primitive-extension.md) | 晨煦 | 2026-06-05 | 草案 |

---

## 審査中RFC

| 番号 | タイトル | 著者 | 作成日 | 状態 |
|------|------|------|----------|------|
| RFC-018 | [RFC-018：LLVM AOT コンパイラ設計](./review/018-llvm-aot-compiler.md) | 晨煦 | 2026-02-15 | 審査中 |
| RFC-022 | [RFC 022: ホア論理静的検証サポート（仕様アノテーションと仕様型）](./review/022-hoare-logic-static-verification.md) | 晨煦 | 2026-03-16 | 審査中 |
| RFC-026 | [RFC-026：FFI コアメカニズム](./review/026-ffi-core-mechanism.md) | 晨煦 | 2026-06-05 | 審査中 |

---

## 採用済みRFC

| 番号 | タイトル | 著者 | 作成日 | 状態 |
|------|------|------|----------|------|
| RFC-004 | [RFC-004: 関数適用法の複数位置ユニオン束縛設計](./accepted/004-curry-multi-position-binding.md) | 晨煦 | 2025-01-05 | 採用済み |
| RFC-006 | [RFC-006: ドキュメントサイト構築](./accepted/006-documentation-site-optimization.md) | 晨煦 | 2025-01-05 | 採用済み |
| RFC-007 | [RFC-007: 関数定義構文統一方案](./accepted/007-function-syntax-unification.md) | 沫郁酱 | 2025-01-05 | 採用済み |
| RFC-008 | [RFC-008：Runtime 並行モデルとスケジューラ分離設計](./accepted/008-runtime-concurrency-model.md) | 晨煦 | 2025-01-05 | 採用済み |
| RFC-009 | [RFC-009: 所有権モデル設計](./accepted/009-ownership-model.md) | 晨煦 | 2025-01-08 | 採用済み |
| RFC-010 | [RFC-010: 統一型構文 - name: type = value モデル](./accepted/010-unified-type-syntax.md) | 晨煦 | 2025-01-20 | 採用済み |
| RFC-011 | [RFC-011: ジェネリクスシステム設計 - ゼロコスト抽象化とマクロ代替](./accepted/011-generic-type-system.md) | 晨煦 | 2025-01-25 | 採用済み |
| RFC-012 | [RFC 012: F-String テンプレート文字列](./accepted/012-f-string-template-strings.md) | Chen Xu | 2025-01-27 | 採用済み |
| RFC-013 | [RFC 013: エラーコード仕様](./accepted/013-error-code-specification.md) | 晨煦 | 2026-02-02 | 採用済み |
| RFC-014 | [RFC-014: パッケージ管理システム設計](./accepted/014-package-manager.md) | 晨煦 | 2026-02-12 | 採用済み |
| RFC-015 | [RFC-015: YaoXiang 設定システム設計](./accepted/015-configuration-system.md) | 晨煦 | 2026-02-12 | 採用済み |
| RFC-017 | [RFC-017: 言語サーバープロトコル（LSP）サポート設計](./accepted/017-lsp-support.md) | 晨煦 | 2026-02-15 | 審査中 |
| RFC-023 | [RFC-023: クロージャ捕獲モデル](./accepted/023-closure-capture-model.md) | 晨煦 | 2026-05-29 | 採用済み |
| RFC-024 | [RFC-024：spawn ブロックに基づく並行モデル](./accepted/024-concurrency-model.md) | 晨煦 | 2026-06-05 | 採用済み |

---

## 廃止済みRFC

| 番号 | タイトル | 著者 | 作成日 | 状態 |
|------|------|------|----------|------|
| RFC-001 | [RFC-001：spawnモデルとエラー処理システム](./deprecated/001-concurrent-model-error-handling.md) | 晨煦 | 2025-01-05 | 廃止済み（RFC-024に取代） |
| RFC-020 | [RFC-020：動的モジュールとFFI統合](./deprecated/020-dynamic-modules-ffi.md) | 晨煦（コミュニティ議論の整理に基づく） | 2026-03-14 | 廃止済み |
| RFC-021 | [RFC-021: ライブラリ駆動FFI拡張と異言語呼び出しサポート](./deprecated/021-library-driven-ffi-extension.md) | 晨煦 | 2026-03-14 | 廃止済み |

---

## 拒否済みRFC

| 番号 | タイトル | 著者 | 作成日 | 状態 |
|------|------|------|----------|------|
| RFC-002 | [RFC-002：クロスプラットフォームI/Oとlibuv統合](./rejected/002-cross-platform-io-libuv.md) | 晨煦 | 2025-01-05 | 拒否済み |
| RFC-003 | [RFC-003：バージョニング計画](./rejected/003-version-planning.md) | 晨煦 | 2025-01-05 | 拒否済み |
| RFC-005 | [RFC-005: 自動化CVEセキュリティ検査システム](./rejected/005-automated-cve-scanning.md) | 晨煦 | 2025-01-05 | 拒否済み |
| RFC-016 | [RFC 016: 量子ネイティブサポートとマルチバックエンド統合](./rejected/016-quantum-native-support.md) | 晨煦 | 2026-02-13 | 拒否済み |

---

## RFCライフサイクル

```
草案 → 審査中 → 採用済み → 廃止済み（取代）
                  ↓
               拒否済み（不採用）
```

### 状態の説明

| 状態 | 位置 | 説明 |
|------|------|------|
| **草案** | `rfc/draft/` | 作成者草案、審査提出待ち |
| **審査中** | `rfc/review/` | コミュニティ議論とフィードバック公開中 |
| **採用済み** | `rfc/accepted/` | 正式設計ドキュメントとなり、実装段階へ |
| **廃止済み** | `rfc/deprecated/` | かつて採用済み、新設計に取代 |
| **拒否済み** | `rfc/rejected/` | 拒否されたRFCドキュメント |

---

## RFCの提交

1. [RFC_TEMPLATE.md](RFC_TEMPLATE.md) を読んでフォーマット要件を理解する
2. [EXAMPLE_full_feature_proposal.md](EXAMPLE_full_feature_proposal.md) を參考に書き方を學ぶ
3. 新規ファイルを作成し、`番号-記述的タイトル.md` と命名する
4. ファイルを `docs/reference/rfc/draft/` ディレクトリに入れる
5. このインデックスファイルを更新し、新しいRFCエントリを追加する
6. 審査プロセスにPRを提交する

---

## 寄稿ガイドライン

寄稿ガイドラインについては、[CONTRIBUTING.md](../../../../CONTRIBUTING.md) を參照してください。