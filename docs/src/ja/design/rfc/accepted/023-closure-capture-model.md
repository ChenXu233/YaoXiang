---
title:  "RFC-023：クロージャ捕获モデル"
---

# RFC-023: クロージャ捕獲モデル

> **ステータス**: 承認済み
> **著者**: 晨煦
> **作成日**: 2026-05-29
> **最終更新**: 2026-05-29

> **参照**:
> - [RFC-007: 関数構文の統一](./accepted/007-function-syntax-unification.md)
> - [RFC-009: 所有権モデル v9](./accepted/009-ownership-model.md)
> - [RFC-011: ジェネリック型システム設計](./accepted/011-generic-type-system.md) — 第 2.4 節：Dup/Clone 組み込み marker trait

## 摘要

本 RFC は YaoXiang 言語の**クロージャ捕獲モデル**を定義する。コンパイラはクロージャ体が参照する外部変数を自動分析し、変数の型（Dup/非Dup）およびクロージャがエスケープするかどうかに応じた捕獲方式を自動選択する——Dup 型は直接コピー、非 Dup で非エスケープの場合は借用、非 Dup でエスケープの場合は Move。用户はゼロ标注で、関数呼び出しの自動借用選択と同一のルールセットを共有する。

## 動機

### なぜ必要か？

現在のクロージャ捕獲は**空実装**——`MakeClosure` 命令の `env` フィールドは常に空であり、lambda は外部変数を参照できない。借用トークンシステムはクロージャが `&T` トークン（ゼロコストコピー）を捕獲できることを要求するが、これはコアな使用シナリオである。

### 現在の問題

```yaoxiang
# このようなコードは現在コンパイル不可——lambda は threshold を参照できない
filter_by: (items: List(Point), threshold: &Float) -> List(Point) = {
    items.filter(|p| p.x > threshold)  # ❌ threshold が捕獲できない
}
```

## 提案

### コア設計

クロージャ捕獲はコンパイラが全自动的に判断する。ルールは関数呼び出しの自動借用選択と**完全に同一**である：

```
変数の型    クロージャがエスケープ    捕獲方式
─────────────────────────────────────────
Dup         任意                    コピー（ビットコピーまたはゼロコスト）
非 Dup      非エスケープ            自動借用（&T または &mut T）
非 Dup      エスケープ              Move（所有権移転）
```

**エスケープ判定**：

```
spawn { || ... }           → エスケープ
return || ...              → エスケープ
let x = || ... ;  x をフィールドに保存 → エスケープ
items.filter(|p| ...)      → 非エスケープ（sync 高階関数呼び出し）
||.method()                → 非エスケープ（即時呼び出し）
```

保守原則：判断できない場合はエスケープとして処理する。

### 示例

```yaoxiang
# 1. Dup トークン——直接コピー（ゼロコスト）
filter_by: (items: List(Point), threshold: &Float) -> List(Point) = {
    # threshold: &Float → Dup → コンパイラがトークンをクロージャにコピー
    # ゼロサイズトークン、ゼロ実行時オーバーヘッド
    items.filter(|p| p.x > threshold)
}

# 2. 非 Dup + 非エスケープ——自動借用
process: (buf: Buffer) -> Void = {
    # buf は Dup でない、filter はエスケープしない → 自動的に &Buffer トークンを作成
    transform(|b| b.read())
    # クロージャ返却後トークン解放、buf は使用可能に戻る
}

# 3. クロージャがエスケープ——Move
spawn_worker: (data: Data) -> Void = {
    # data は Dup でない、spawn → エスケープ → Move
    spawn { use(data) }
}

# 4. 混合捕獲
complex: (items: List(Point), config: &Config, buf: Buffer) -> List(Point) = {
    # config: &Config → Dup → トークンをコピー
    # buf: Buffer → Dup でない、非エスケープ → &mut Buffer 借用
    items.filter(|p| {
        let threshold = config.get_threshold()
        buf.update(p)
        p.x > threshold
    })
}

# 5. 借用衝突検出
bad: (buf: Buffer) -> Void = {
    closure = |b| b.write()
    buf.read()  # ❌ buf はすでにクロージャに借用されており、ここでは衝突
}
```

### 構文変化

**ゼロ構文変化**。捕獲方式はコンパイラが自動的に決定し、ユーザーは标注を書く必要はない。

## 詳細設計

### 型システムへの影響

Lambda の型シグネチャは変更なし：`（params) -> Return`。捕獲された変数は型シグネチャに反映されず、コンパイラが IR 生成段階で処理する。

### コンパイラの改动

| コンポーネント | 改动 | 説明 |
|------|------|------|
| `capture.rs`（新規作成） | 捕獲分析 + エスケープ分析 + パターン選択 | ~150 行 |
| `expressions.rs` | lambda 型推論で捕獲分析を呼び出す | ~10 行 |
| `ir_gen.rs` | MakeClosure env 填充；ZST スキップ | ~80 行 |
| `ir.rs` | MakeClosure env 型調整の可能性 | ~5 行 |

**捕獲分析フロー**：

```
1. lambda body AST を走査
2. すべての Expr::Var(name) 参照を収集
3. フィルタリング：クロージャ外部スコープの変数のみを保持
4. 分類：Read（読み取り専用）/ Write（読み書き）/ Move（移転済み）
5. 型属性を確認： Dup かどうか
6. エスケープ判定：クロージャの使用方式
7. 捕獲モードを選択：
   Dup → Copy
   非Dup + 非エスケープ + Read → Borrow（&T）
   非Dup + 非エスケープ + Write → BorrowMut（&mut T）
   非Dup + エスケープ → Move
```

**IR 生成**：

```rust
// 現在（空）
Instruction::MakeClosure { dst, func, env: Vec::new() }

// 改める
Instruction::MakeClosure { dst, func, env: captured_env }

// captured_env の生成ロジック：
for captured in captures {
    match captured.mode {
        Copy if is_zst(captured.ty) => {
            // ゼロサイズ型——命令を生成しない
            // クロージャ体が直接外層変数を参照（コンパイル時消除）
        }
        Copy => {
            // Move dst, src を生成（Dup 型の浅コピー）
        }
        Borrow => {
            // Borrow dst, src を生成（ReadToken を作成）
        }
        BorrowMut => {
            // Borrow dst, src を生成（WriteToken を作成）
        }
        Move => {
            // Move dst, src を生成（所有権移転）
        }
    }
}
```

### 実行時動作

捕獲方式は実行時パフォーマンスに影響しない：

- **Dup + ZST**（例：`&T` トークン）→ ゼロ命令、クロージャ体が直接外層変数を参照
- **Dup + 非 ZST**（例：Int）→ 1 回レジスタコピー
- **Borrow/BorrowMut**→ トークン作成（コンパイル時コンセプト、ゼロオーバーヘッド）
- **Move** → 通常の Move と同じコスト

### 後方互換性

完全互換。現在すべての lambda は外部変数を捕獲できず、本 RFC は表現力を追加するだけで、既存のコードを破壊しない。

## トレードオフ

### メリット

1. **ゼロ标注**：ユーザーは捕獲标注を書く必要がない
2. **関数呼び出しとの統一**：捕獲ルール = 関数呼び出しの自動借用ルール
3. **ゼロコスト**：Dup トークンの捕獲は完全にコンパイル時に消除
4. **安全**：エスケープ分析により use-after-free を防止

### デメリット

1. **エスケープ分析が保守的**：判断できない場合はエスケープとして処理し、不必要な Move を起こす可能性がある
2. **暗黙的**：捕獲方式是コードに反映されず、デバッグ時はコンパイル出力を確認する必要がある

## 代替案

| 方案 | 選択しない理由 |
|------|--------------|
| Rust 式の明示的 `move` キーワード | 新構文の導入によりユーザーの認知的負担が増加 |
| すべて Move | ゼロコストトークン借用を表現できない |
| すべて借用 | クロージャがエスケープするとダングリング参照が発生 |
| ユーザーが手動で捕獲方式を标注 | 「コンパイラ全自动」の設計哲学に反する |

## 実装戦略

### フェーズ分け

1. **Phase 1**：捕獲分析（外部変数参照の識別のみ、捕獲方式の区別なし）
2. **Phase 2**：エスケープ分析 + パターン選択
3. **Phase 3**：IR 生成 + ZST 最適化
4. **Phase 4**：借用衝突検出の統合

### 依存関係

- RFC-011（ジェネリックシステム、第 2.4 節 Dup/Clone trait）に依存——変数が複製可能か判断するために Dup trait が必要
- RFC-009 v9（借用トークン）に依存——Borrow/BorrowMut 捕獲モードにはトークン型が必要
- RFC-023 および本 RFC の実装後、借用トークンシステム（RFC-009 v9 の実装）に 착手可能

### リスク

- エスケープ分析が過度に保守的であり、不必要な Move を起こす可能性がある；后续で最適化可能
- ジェネリッククロージャの捕獲分析には追加処理が必要な場合がある

## 設計決定記録

| 決定 | 決定内容 | 理由 | 日付 |
|------|------|------|------|
| 捕獲方式選択 | 全自动 | 関数呼び出しルールとの統一 | 2026-05-29 |
| エスケープ分析 | 保守原則 | 判断できない場合はエスケープ、安全優先 | 2026-05-29 |
| ZST 最適化 | IR 生成時にスキップ | 後続最適化パスよりシンプル | 2026-05-29 |
| 捕獲を型シグネチャに反映しない | コンパイラの内部処理 | lambda 型をシンプルに保つ | 2026-05-29 |

## 参考文献

### YaoXiang 公式ドキュメント

- [RFC-007: 関数構文の統一](./accepted/007-function-syntax-unification.md)
- [RFC-009: 所有権モデル v9](./accepted/009-ownership-model.md)
- [RFC-011: ジェネリック型システム設計](./accepted/011-generic-type-system.md) — 第 2.4 節：Dup/Clone 組み込み marker trait

### 外部参照

- [Rust クロージャ捕獲ルール](https://doc.rust-lang.org/reference/types/closure.html#capture-modes)
- [Swift クロージャ捕獲セマンティクス](https://docs.swift.org/swift-book/documentation/the-swift-programming-language/closures/#Capturing-Values)