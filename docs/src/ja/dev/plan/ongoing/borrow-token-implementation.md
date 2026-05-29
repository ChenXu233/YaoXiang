---
title: 借用トークンシステム実装ロードマップ
status: ongoing
created: 2026-05-29
---

# 借用トークンシステム実装ロードマップ

## 目標

RFC-009 v9 の借用トークンシステムを完全に実装し、古い乞丐版借用を置き換える。

## 前提依存チェーン

```
RFC-009 v9 (借用トークン設計) ← 完了
    │
    ├── 1. 型プロパティシステム [設計完了 → type-property-system-dup.md]
    │      ├── Dup trait の定義と実装
    │      ├── trait solver の再帰 struct 検査
    │      └── auto-derive の再帰フィールド検査
    │
    ├── 2. クロージャキャプチャモデル [設計完了 → closure-capture-model.md]
    │      ├── 変数キャプチャ解析
    │      ├── エスケープ解析
    │      └── MakeClosure env 投入
    │
    └── 3. 借用トークン実装 [フェーズ 1、2 完了待ち]
           ├── MonoType::Ref { mutable, inner }
           ├── borrow checker pass (middle/passes/lifetime/)
           ├── トークン競合検出 (フロー敏感生存分析)
           └── ZST 最適化 (トークンコンパイル後消滅)
```

## フェーズ

### フェーズ 1：型プロパティシステム

**状態**：設計完了 → [type-property-system-dup.md](type-property-system-dup.md)

**範囲**：
- Dup trait を組み込み marker trait として登録
- 原語型を自動的に Dup としてマーク
- struct/列挙型/タプルの自動導出：全フィールド Dup → 型 Dup
- trait solver が再帰 struct/列挙型/タプルの検査をサポート
- auto-derive がジェネリックコンテナフィールド (`List(Int)` 等) をサポート
- Send/Sync の削除（ユーザーには非公開、编译器が全自动処理）

**関連ファイル**：
- `src/frontend/core/types/base/mono.rs`
- `src/frontend/core/typecheck/traits/std_traits.rs`
- `src/frontend/core/typecheck/traits/auto_derive.rs`
- `src/frontend/core/typecheck/traits/solver.rs`

### フェーズ 2：クロージャキャプチャモデル

**状態**：設計待ち

**範囲**：
- 型検査時に lambda が参照する外部変数を解析
- 各変数のキャプチャ方式を決定（借用トークン vs Move）
- IR 生成時に MakeClosure env を投入
- 借用トークンのクロージャ内での伝播をサポート

### フェーズ 3：借用トークン実装

**状態**：フェーズ 1、2 完了待ち

**範囲**：
- AST: `Type::Ref`、`Expr::Borrow`
- 詞法: `&` と `&mut` トークン
- MonoType: `Ref { mutable, inner }`
- IR: 借用命令（必要に応じて）
- Passes: `BorrowChecker` (フロー敏感生存分析)
- ZST 最適化: トークンコンパイル後消除

## 参考

- [RFC-009 所有権モデル v9](../../design/rfc/accepted/009-ownership-model.md)
- [RFC-010 統一型構文](../../design/rfc/accepted/010-unified-type-syntax.md)
- [RFC-011 ジェネリック型システム設計](../../design/rfc/accepted/011-generic-type-system.md)