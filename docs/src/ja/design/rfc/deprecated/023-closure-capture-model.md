---
title: "RFC-023: クロージャキャプチャモデル"
status: "廃止"
author: "晨煦"
created: "2026-05-29"
updated: "2026-06-16"
---

> **廃止理由**：2026-06-16 言語設計判断——Lambda/関数値は外側の変数を暗黙的にキャプチャせず、明示的な引数渡しを使用します。`spawn { }` は同フレーム実行であり、クロージャキャプチャには関わりません。本 RFC のキャプチャ解析システムは完全に削除されました（約 850 行のコード）。詳細は [RFC-009 設計判断](../accepted/009-ownership-model.md#設計判断記録) を参照してください。

# RFC-023: クロージャキャプチャモデル

> **参考**:
> - [RFC-007: 関数構文統一](./accepted/007-function-syntax-unification.md)
> - [RFC-009: 所有権モデル v9](./accepted/009-ownership-model.md)
> - [RFC-011: ジェネリクスシステム設計](./accepted/011-generic-type-system.md) — 第 2.4 節：Dup/Clone 組み込み marker trait

## 概要

本 RFC は YaoXiang 言語の**クロージャキャプチャモデル**を定義します。コンパイラはクロージャ本体が参照する外部変数を自動解析し、変数の型（Dup/非 Dup）およびクロージャがエスケープするかどうかに応じて、自動的にキャプチャ方式を選択します。Dup 型は直接コピー、非 Dup で非エスケープは借用、非 Dup でエスケープは Move。ユーザーは注釈なしで、関数呼び出しの自動借用選択と同じルールを共有します。

## 動機

### なぜ必要か？

現在のクロージャキャプチャは**空実装**です。`MakeClosure` 命令の `env` フィールドは常に空であり、lambda はいかなる外部変数も参照できません。借用トークンシステムはクロージャが `&T` トークン（ゼロコストコピー）をキャプチャできることを要求しており、これは中核的なユースケースです。

### 現状の問題

```yaoxiang
# このコードは現状コンパイルできない——lambda は threshold を参照できない
filter_by: (items: List(Point), threshold: &Float) -> List(Point) = {
    items.filter(|p| p.x > threshold)  # ❌ threshold はキャプチャできない
}
```

## 提案

### 中核設計

クロージャキャプチャはコンパイラによって完全に自動判断されます。ルールは関数呼び出しの自動借用選択と**完全に同じ**です：

```
変数の型    クロージャのエスケープ    キャプチャ方式
─────────────────────────────────────────
Dup         任意                      コピー（ビットコピーまたはゼロコスト）
非 Dup      非エスケープ              自動借用（&T または &mut T）
非 Dup      エスケープ                Move（所有権移転）
```

**エスケープ判定**：

```
spawn { || ... }           → エスケープ
return || ...              → エスケープ
let x = || ... ;  x がフィールドに保存される → エスケープ
items.filter(|p| ...)      → 非エスケープ（sync 高階関数呼び出し）
||.method()                → 非エスケープ（即座に呼び出し）
```

保守的原則：判定不能な場合はエスケープとして扱います。

### 例

```yaoxiang
# 1. Dup トークン——直接コピー（ゼロコスト）
filter_by: (items: List(Point), threshold: &Float) -> List(Point) = {
    # threshold: &Float → Dup → コンパイラがトークンをクロージャにコピー
    # ゼロサイズトークン、ランタイムオーバーヘッドなし
    items.filter(|p| p.x > threshold)
}

# 2. 非 Dup + 非エスケープ——自動借用
process: (buf: Buffer) -> Void = {
    # buf は非 Dup、filter は非エスケープ → 自動的に &Buffer トークンを作成
    transform(|b| b.read())
    # クロージャ終了後にトークン解放、buf は再び使用可能
}

# 3. クロージャのエスケープ——Move
spawn_worker: (data: Data) -> Void = {
    # data は非 Dup、spawn → エスケープ → Move
    spawn { use(data) }
}

# 4. 混合キャプチャ
complex: (items: List(Point), config: &Config, buf: Buffer) -> List(Point) = {
    # config: &Config → Dup → トークンをコピー
    # buf: Buffer → 非 Dup、非エスケープ → &mut Buffer 借用
    items.filter(|p| {
        let threshold = config.get_threshold()
        buf.update(p)
        p.x > threshold
    })
}

# 5. 借用競合の検出
bad: (buf: Buffer) -> Void = {
    closure = |b| b.write()
    buf.read()  # ❌ buf は既にクロージャに借用されており、ここで競合
}
```

### 構文変更

**構文変更はゼロ**。キャプチャ方式はコンパイラによって自動決定され、ユーザーは注釈を書く必要はありません。

## 詳細設計

### 型システムへの影響

Lambda の型シグネチャは変更されません：`(params) -> Return`。キャプチャされる変数は型シグネチャには現れず、コンパイラが IR 生成段階で処理します。

### コンパイラの変更

| コンポーネント | 変更内容 | 説明 |
|------|------|------|
| `capture.rs`（新規） | キャプチャ解析 + エスケープ解析 + モード選択 | 約 150 行 |
| `expressions.rs` | lambda 型推論がキャプチャ解析を呼び出し | 約 10 行 |
| `ir_gen.rs` | MakeClosure env への充填；ZST のスキップ | 約 80 行 |
| `ir.rs` | MakeClosure env 型の調整が必要な可能性 | 約 5 行 |

**キャプチャ解析フロー**：

```
1. lambda 本体 AST を走査
2. すべての Expr::Var(name) 参照を収集
3. フィルタリング：クロージャ外部スコープの変数のみ保持
4. 分類：Read（読み取り専用）/ Write（読み書き）/ Move（転送される）
5. 型属性を照会：Dup かどうか
6. エスケープ判定：クロージャの使用方式
7. キャプチャモードの選択：
   Dup → Copy
   非Dup + 非エスケープ + Read → Borrow（&T）
   非Dup + 非エスケープ + Write → BorrowMut（&mut T）
   非Dup + エスケープ → Move
```

**IR 生成**：

```rust
// 現在（空）
Instruction::MakeClosure { dst, func, env: Vec::new() }

// 変更後
Instruction::MakeClosure { dst, func, env: captured_env }

// captured_env の生成ロジック：
for captured in captures {
    match captured.mode {
        Copy if is_zst(captured.ty) => {
            // ゼロサイズ型——いかなる命令も生成しない
            // クロージャ本体は外側を直接参照（コンパイル時に除去）
        }
        Copy => {
            // Move dst, src を生成（Dup 型のシャローコピー）
        }
        Borrow => {
            // Borrow dst, src を生成（ReadToken の作成）
        }
        BorrowMut => {
            // Borrow dst, src を生成（WriteToken の作成）
        }
        Move => {
            // Move dst, src を生成（所有権移転）
        }
    }
}
```

### ランタイム動作

キャプチャ方式はランタイム性能に影響しません：

- **Dup + ZST**（例：`&T` トークン）→ ゼロ命令、クロージャ本体は外側変数を直接参照
- **Dup + 非 ZST**（例：Int）→ 1 回のレジスタコピー
- **Borrow/BorrowMut**→ トークン作成（コンパイル時の概念、ゼロオーバーヘッド）
- **Move** → 通常の Move と同じコスト

### 後方互換性

完全互換です。現在の lambda はすべて外部変数をキャプチャできないため、本 RFC は表現力を追加するだけで、既存コードを破壊しません。

## トレードオフ

### 利点

1. **注釈ゼロ**：ユーザーはキャプチャ注釈を書く必要がない
2. **関数呼び出しとの統一**：キャプチャルール = 関数呼び出しの自動借用ルール
3. **ゼロコスト**：Dup トークンのキャプチャは完全にコンパイル時に除去される
4. **安全**：エスケープ解析が use-after-free を防止

### 欠点

1. **エスケープ解析が保守的**：判定不能な場合はエスケープとして処理されるため、不要な Move が発生する可能性
2. **暗黙的**：キャプチャ方式がコードに現れないため、デバッグ時にコンパイル出力を確認する必要がある

## 代替案

| 代替案 | 採用しない理由 |
|------|--------------|
| Rust 式の明示的な `move` キーワード | 新しい構文を導入し、ユーザーの認知負荷が増大 |
| 全て Move | ゼロコストのトークン借用を表現できない |
| 全て借用 | クロージャのエスケープによりダングリング参照が発生 |
| ユーザーが手動でキャプチャ方式を注釈 | 「コンパイラ全自動」の設計哲学に反する |

## 実装戦略

### 段階分け

1. **Phase 1**：キャプチャ解析（外部変数参照の識別の、キャプチャ方式の区別なし）
2. **Phase 2**：エスケープ解析 + モード選択
3. **Phase 3**：IR 生成 + ZST 最適化
4. **Phase 4**：借用競合検出の統合

### 依存関係

- RFC-011（ジェネリクスシステム、第 2.4 節 Dup/Clone trait）に依存——変数がコピー可能か判断するために Dup trait が必要
- RFC-009 v9（借用トークン）に依存——Borrow/BorrowMut キャプチャモードにはトークン型が必要
- RFC-023 と本 RFC の実装後、借用トークンシステム（RFC-009 v9 実装）に着手可能

### リスク

- エスケープ解析が過度に保守的になり、不要な Move が発生する可能性；後日最適化可能
- ジェネリッククロージャのキャプチャ解析には追加処理が必要な可能性

## 設計判断記録

| 判断 | 決定 | 理由 | 日付 |
|------|------|------|------|
| キャプチャ方式の選択 | 全自動 | 関数呼び出しルールとの統一 | 2026-05-29 |
| エスケープ解析 | 保守的原則 | 判定不能時はエスケープとして処理、安全優先 | 2026-05-29 |
| ZST 最適化 | IR 生成時にスキップ | 後続の最適化パスよりも単純 | 2026-05-29 |
| キャプチャを型シグネチャに含めない | コンパイラ内部で処理 | lambda 型の簡潔性を維持 | 2026-05-29 |

## 参考文献

### YaoXiang 公式ドキュメント

- [RFC-007: 関数構文統一](./accepted/007-function-syntax-unification.md)
- [RFC-009: 所有権モデル v9](./accepted/009-ownership-model.md)
- [RFC-011: ジェネリクスシステム設計](./accepted/011-generic-type-system.md) — 第 2.4 節：Dup/Clone 組み込み marker trait

### 外部参考

- [Rust クロージャキャプチャルール](https://doc.rust-lang.org/reference/types/closure.html#capture-modes)
- [Swift クロージャキャプチャセマンティクス](https://docs.swift.org/swift-book/documentation/the-swift-programming-language/closures/#Capturing-Values)
```