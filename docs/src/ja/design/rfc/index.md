```markdown
---
title: "RFC インデックス"
---

# YaoXiang RFC（Request for Comments）インデックス

> RFC（Request for Comments）はYaoXiang言語の特性設計提案を正式に提出するフォーマットです。

## 目次

- [テンプレート](#テンプレート)
- [ドラフトRFC](#ドラフトrfc)
- [レビュー中RFC](#レビュー中rfc)
- [承認済みRFC](#承認済みrfc)
- [廃止RFC](#廃止rfc)
- [拒否RFC](#拒否rfc)

---

## テンプレート

| ファイル | 説明 |
|------|------|
| [RFC_TEMPLATE.md](RFC_TEMPLATE.md) | RFC標準テンプレート |
| [EXAMPLE_full_feature_proposal.md](EXAMPLE_full_feature_proposal.md) | 完全なサンプル（パターンマッチングの強化） |

---

## ドラフトRFC

| 番号 | タイトル | 著者 | 作成日 | ステータス |
|------|------|------|----------|------|
| RFC-019 | [RFC-019: 型レベル同像性 (Typed Homoiconicity) - 構文即型](./draft/019-typed-homoiconicity.md) | 晨煦 | 2026-02-20 | ドラフト |
| RFC-028 | [RFC-028：JIT コンパイラ — VM内多段実行エンジン](./draft/028-jit-compiler.md) | 晨煦 | 2026-06-11 | ドラフト |
| RFC-029 | [RFC-029：モジュール意味論システム](./draft/029-module-semantics.md) | 晨煦 | 2026-06-13 | ドラフト |

---

## レビュー中RFC

| 番号 | タイトル | 著者 | 作成日 | ステータス |
|------|------|------|----------|------|
| RFC-025 | [RFC-025: 拡張可能プリミティブ型機構](./review/025-primitive-extension.md) | 晨煦 | 2026-06-05 | レビュー中 |
| RFC-026 | [RFC-026：FFI コア機構](./review/026-ffi-core-mechanism.md) | 晨煦 | 2026-06-05 | レビュー中 |

---

## 承認済みRFC

| 番号 | タイトル | 著者 | 作成日 | ステータス |
|------|------|------|----------|------|
| RFC-004 | [RFC-004: カリー化メソッドの複数位置ユニオン束縛設計](./accepted/004-curry-multi-position-binding.md) | 晨煦 | 2025-01-05 | 承認済み |
| RFC-006 | [RFC-006: ドキュメントサイト構築](./accepted/006-documentation-site-optimization.md) | 晨煦 | 2025-01-05 | 承認済み |
| RFC-007 | [RFC-007: 関数定義構文の統一方案](./accepted/007-function-syntax-unification.md) | 沫郁酱 | 2025-01-05 | 承認済み |
| RFC-008 | [RFC-008：Runtime 並行モデルとスケジューラの疎結合設計](./accepted/008-runtime-concurrency-model.md) | 晨煦 | 2025-01-05 | 承認済み |
| RFC-009 | [RFC-009: 所有権モデル設計](./accepted/009-ownership-model.md) | 晨煦 | 2025-01-08 | 承認済み |
| ↳ RFC-009a | [RFC-009a: トークンライフタイム解析——ホーア証明パイプラインに基づく](./accepted/009a-borrow-proof-pipeline.md) | 晨煦 | 2026-06-13 | 承認済み |
| RFC-010 | [RFC-010: 型構文の統一 - name: type = value モデル](./accepted/010-unified-type-syntax.md) | 晨煦 | 2025-01-20 | 承認済み |
| RFC-011 | [RFC-011: ジェネリクスシステム設計 - ゼロコスト抽象とマクロ代替](./accepted/011-generic-type-system.md) | 晨煦 | 2025-01-25 | 承認済み |
| ↳ RFC-011a | [RFC-011a: インターフェース実装と動的ディスパッチ](./draft/011a-interface-implementation.md) | 晨煦 | 2026-06-14 | ドラフトRFC |
| RFC-012 | [RFC 012: F-String テンプレート文字列](./accepted/012-f-string-template-strings.md) | Chen Xu | 2025-01-27 | 承認済み |
| RFC-013 | [RFC 013: エラーコード規範](./accepted/013-error-code-specification.md) | 晨煦 | 2026-02-02 | 承認済み |
| RFC-014 | [RFC-014: パッケージ管理システム設計](./accepted/014-package-manager.md) | 晨煦 | 2026-02-12 | 承認済み |
| ↳ RFC-014a | [RFC-014a: Registry プロトコル規範](./draft/014a-registry-protocol.md) | 晨煦 | 2026-06-11 | ドラフトRFC |
| ↳ RFC-014b | [RFC-014b: ビルドシステムとバイナリ配布](./draft/014b-build-system.md) | 晨煦 | 2026-06-11 | ドラフトRFC |
| ↳ RFC-014c | [RFC-014c: ワークスペースサポート](./draft/014c-workspace.md) | 晨煦 | 2026-06-11 | ドラフトRFC |
| RFC-015 | [RFC-015: YaoXiang 設定システム設計](./accepted/015-configuration-system.md) | 晨煦 | 2026-02-12 | 承認済み |
| RFC-017 | [RFC-017: 言語サーバープロトコル（LSP）サポート設計](./accepted/017-lsp-support.md) | 晨煦 | 2026-02-15 | レビュー中 |
| RFC-018 | [RFC-018：LLVM AOT コンパイラ設計](./accepted/018-llvm-aot-compiler.md) | 晨煦 | 2026-02-15 | 承認済み |
| RFC-023 | [RFC-023: クロージャ捕捉モデル](./accepted/023-closure-capture-model.md) | 晨煦 | 2026-05-29 | 承認済み |
| RFC-024 | [RFC-024：spawn ブロックに基づく並行モデル](./accepted/024-concurrency-model.md) | 晨煦 | 2026-06-05 | 承認済み |
| RFC-027 | [RFC-027：コンパイル時述語と統一静的検証](./accepted/027-compile-time-evaluation-types.md) | 晨煦 | 2026-06-07 | 承認済み |

---

## 廃止RFC

| 番号 | タイトル | 著者 | 作成日 | ステータス |
|------|------|------|----------|------|
| RFC-001 | [RFC-001：並作モデルとエラーハンドリングシステム](./deprecated/001-concurrent-model-error-handling.md) | 晨煦 | 2025-01-05 | 廃止（RFC-024 に置き換え） |
| RFC-020 | [RFC-020：動的モジュールと FFI 統合](./deprecated/020-dynamic-modules-ffi.md) | 晨煦（コミュニティでの議論に基づく整理） | 2026-03-14 | 廃止 |
| RFC-021 | [RFC-021: ライブラリ駆動 FFI 拡張と言語間呼び出しサポート](./deprecated/021-library-driven-ffi-extension.md) | 晨煦 | 2026-03-14 | 廃止 |
| RFC-022 | [RFC 022: ホーア論理静的検証サポート（仕様コメントと仕様型）](./deprecated/022-hoare-logic-static-verification.md) | 晨煦 | 2026-03-16 | 廃止（RFC-027 に置き換え） |

---

## 拒否RFC

| 番号 | タイトル | 著者 | 作成日 | ステータス |
|------|------|------|----------|------|
| RFC-002 | [RFC-002：クロスプラットフォームI/Oとlibuv統合](./rejected/002-cross-platform-io-libuv.md) | 晨煦 | 2025-01-05 | 拒否 |
| RFC-003 | [RFC-003：バージョン計画](./rejected/003-version-planning.md) | 晨煦 | 2025-01-05 | 拒否 |
| RFC-005 | [RFC-005: 自動CVEセキュリティ検査システム](./rejected/005-automated-cve-scanning.md) | 晨煦 | 2025-01-05 | 拒否 |
| RFC-016 | [RFC 016: 量子ネイティブサポートとマルチバックエンド統合](./rejected/016-quantum-native-support.md) | 晨煦 | 2026-02-13 | 拒否 |

---

## RFCライフサイクル

```
ドラフト → レビュー中 → 承認済み → 廃止（置き換え）
                       ↓
                    拒否（不承認）
```

### ステータス説明

| ステータス | 場所 | 説明 |
|------|------|------|
| **ドラフト** | `rfc/draft/` | 著者の草稿、レビュー提出待ち |
| **レビュー中** | `rfc/review/` | コミュニティの議論とフィードバックを募集 |
| **承認済み** | `rfc/accepted/` | 正式な設計ドキュメントとなり、実装段階へ |
| **廃止** | `rfc/deprecated/` | 一度は承認されたが、新しい設計に置き換えられた |
| **拒否** | `rfc/rejected/` | 不承認となったRFCドキュメント |

---

## RFCの提出

1. [RFC_TEMPLATE.md](RFC_TEMPLATE.md) を読んでフォーマット要件を確認
2. [EXAMPLE_full_feature_proposal.md](EXAMPLE_full_feature_proposal.md) を参考に書き方を学ぶ
3. 新規ファイルを作成し、`番号-説明的なタイトル.md` と命名
4. ファイルを `docs/reference/rfc/draft/` ディレクトリに配置
5. 本インデックスファイルを更新し、新規RFCエントリを追加
6. PRを提出してレビューフローに進む

---

## コントリビューションガイド

コントリビューションガイドについては [CONTRIBUTING.md](../../../../CONTRIBUTING.md) を参照してください。
```