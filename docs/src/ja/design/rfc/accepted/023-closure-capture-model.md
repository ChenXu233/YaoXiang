---
title: RFC-023：クロージャ捕獲モデル
---

# RFC-023: クロージャ捕獲モデル

> **状態**: 承認済み
> **著者**: 晨煦
> **作成日**: 2026-05-29
> **最終更新**: 2026-05-29

> **参照**:
> - [RFC-007: 関数構文の統一](./accepted/007-function-syntax-unification.md)
> - [RFC-009: 所有権モデル v9](./accepted/009-ownership-model.md)
> - [RFC-011: ジェネリクス型システム設計](./accepted/011-generic-type-system.md) — 第 2.4 節：Dup/Clone 組み込みマーカートレイト

## 要約

本 RFC は YaoXiang 言語の**クロージャ捕獲モデル**を定義する。コンパイラはクロージャ本体が参照する外部変数を自動分析し、変数の型（Dup/非Dup）およびクロージャがエスケープするかどうかに応じて、自動的に捕獲方式を選択する——Dup 型は直接コピー、非 Dup で非エスケープなら借用、非 Dup でエスケープなら Move。ユーザーの注釈は一切不要で、関数呼び出しの自動借用選択と同一のルールを共有する。

## 動機

### なぜ必要か？

現在のクロージャ捕獲は**空実装**——`MakeClosure` 命令の `env` フィールドは常に空であり、lambda は外部変数を参照できない。所有權令牌システムではクロージャが `&T` 令牌（ゼロコストコピー）を捕獲できることが要求されており、これはコアな使用シナリオである。

### 現在の問題

```yaoxiang
# このようなコードは現在コンパイル不可——lambda は threshold を参照できない
filter_by: (items: List(Point), threshold: &Float) -> List(Point) = {
    items.filter(|p| p.x > threshold)  # ❌ threshold が捕獲できない
}
```

## 提案

### コア設計

クロージャ捕獲はコンパイラが全自动で判断する。ルールは関数呼び出しの自動借用選択と**完全に同一**である：

```
変数の型    クロージャがエスケープするか    捕獲方式
─────────────────────────────────────────────────────
Dup         任意                            コピー（ビットコピーまたはゼロコスト）
非 Dup      非エスケープ                    自動借用（&T または &mut T）
非 Dup      エスケープ                      Move（所有権移転）
```

**エスケープ判定**：

```
spawn { || ... }           → エスケープ
return || ...              → エスケープ
let x = || ... ;  x を格納 → エスケープ
items.filter(|p| ...)      → 非エスケープ（sync 高階関数呼び出し）
||.method()                → 非エスケープ（即時呼び出し）
```

保守原則：判断できない場合はエスケープとして扱う。

### 示例

```yaoxiang
# 1. Dup 令牌——直接コピー（ゼロコスト）
filter_by: (items: List(Point), threshold: &Float) -> List(Point) = {
    # threshold: &Float → Dup → コンパイラが令牌をクロージャにコピー
    # ゼロサイズ令牌、ゼロ実行時オーバーヘッド
    items.filter(|p| p.x > threshold)
}

# 2. 非 Dup + 非エスケープ——自動借用
process: (buf: Buffer) -> Void = {
    # buf は Dup でない、filter は非エスケープ → 自動的に &Buffer 令牌を生成
    transform(|b| b.read())
    # クロージャ終了後に令牌が解放され、buf は再び使用可能
}

# 3. クロージャがエスケープ——Move
spawn_worker: (data: Data) -> Void = {
    # data は Dup でない、spawn → エスケープ → Move
    spawn { use(data) }
}

# 4. 混合捕獲
complex: (items: List(Point), config: &Config, buf: Buffer) -> List(Point) = {
    # config: &Config → Dup → 令牌をコピー
    # buf: Buffer → 非 Dup、非エスケープ → &mut Buffer 借用
    items.filter(|p| {
        let threshold = config.get_threshold()
        buf.update(p)
        p.x > threshold
    })
}

# 5. 借用衝突検出
bad: (buf: Buffer) -> Void = {
    closure = |b| b.write()
    buf.read()  # ❌ buf はすでにクロージャに借用されている、此处冲突
}
```

### 構文変化

**ゼロ構文変化**。捕獲方式はコンパイラが自動決定し、ユーザーが注釈を付ける必要はない。

## 詳細設計

### 型システムへの影響

Lambda の型シグネチャは変わらない：` (params) -> Return`。捕獲された変数は型シグネチャに反映されず、コンパイラの IR 生成段階で処理される。

### コンパイラ改动

| コンポーネント | 改动 | 説明 |
|------|------|------|
| `capture.rs`（新規作成） | 捕獲分析 + エスケープ分析 + パターン選択 | ~150 行 |
| `expressions.rs` | lambda 型推論で捕獲分析を呼び出す | ~10 行 |
| `ir_gen.rs` | MakeClosure env 填充；ZST スキップ | ~80 行 |
| `ir.rs` | MakeClosure env 型調整の可能性 | ~5 行 |

**捕獲分析フロー**：

```
1. lambda body AST を巡回
2. すべての Expr::Var(name) 参照を収集
3. フィルタリング：クロージャの外部スコープの変数のみを保持
4. 分類：Read（読み取り専用）/ Write（読み書き）/ Move（移転済み）
5. 型属性を調査：Dup かどうか
6. エスケープを判定：クロージャの使用方式
7. 捕獲パターンを選択：
   Dup → Copy
   非Dup + 非エスケープ + Read → Borrow（&T）
   非Dup + 非エスケープ + Write → BorrowMut（&mut T）
   非Dup + エスケープ → Move
```

**IR 生成**：

```rust
// 現在（空）
Instruction::MakeClosure { dst, func, env: Vec::new() }

// 改为
Instruction::MakeClosure { dst, func, env: captured_env }

// captured_env の生成ロジック：
for captured in captures {
    match captured.mode {
        Copy if is_zst(captured.ty) => {
            // ゼロサイズ型——命令を生成しない
            // クロージャ本体は直接外層変数を参照（コンパイル時解決）
        }
        Copy => {
            // Move dst, src を生成（Dup 型のシャローコピー）
        }
        Borrow => {
            // Borrow dst, src を生成（ReadToken を生成）
        }
        BorrowMut => {
            // Borrow dst, src を生成（WriteToken を生成）
        }
        Move => {
            // Move dst, src を生成（所有権移転）
        }
    }
}
```

### 実行時動作

捕獲方式は実行時性能に影響しない：

- **Dup + ZST**（例：`&T` 令牌）→ ゼロ命令、クロージャ本体は直接外層変数を参照
- **Dup + 非 ZST**（例：Int）→ 1 回レジスタコピー
- **Borrow/BorrowMut**→ 令牌生成（コンパイル時の概念、オーバーヘッドゼロ）
- **Move** → 通常の Move と同じコスト

### 後方互換性

完全に互換性あり。現在のすべての lambda は外部変数を捕獲できないため、本 RFC は表現力を追加するだけで、既存のコードを破壊しない。

## トレードオフ

### 优点

1. **ゼロ注釈**：ユーザーは捕獲注釈を記述する必要がない
2. **関数呼び出しとの統一**：捕獲ルール = 関数呼び出しの自動借用ルール
3. **ゼロコスト**：Dup 令牌の捕獲は完全にコンパイル時に解決
4. **安全**：エスケープ分析により use-after-free を防止

### 缺点

1. **エスケープ分析が保守的**：判断できない場合にエスケープとして扱い、不必要な Move が発生する可能性がある
2. **暗黙的**：捕獲方式はコードに反映されない、デバッグ時にコンパイル出力を確認する必要がある

## 代替方案

| 方案 | 選擇しない理由 |
|------|--------------|
| Rust 形式の明示的 `move` キーワード | 新しい構文の導入、ユーザーの認知的負荷が増加 |
| すべて Move | ゼロコスト令牌借用を表現できない |
| すべて借用 | クロージャがエスケープするとダングリング参照が発生 |
| ユーザーが手動で捕獲方式を注釈 | 「コンパイラ全自动」という設計思想に反する |

## 実装戦略

### フェーズ分け

1. **Phase 1**：捕獲分析（外部変数の参照を識別のみ、捕獲方式の区別なし）
2. **Phase 2**：エスケープ分析 + パターン選択
3. **Phase 3**：IR 生成 + ZST 最適化
4. **Phase 4**：借用衝突検出統合

### 依存関係

- RFC-011（ジェネリクス型システム、第 2.4 節 Dup/Clone trait）に依存——変数がコピー可能かどうかの判断に必要
- RFC-009 v9（借用令牌）に依存——Borrow/BorrowMut 捕獲モードには令牌型が必要
- RFC-023 と本 RFC の実装後、借用令牌システム（RFC-009 v9 実装）に着手可能

### リスク

- エスケープ分析が過度に保守的になり、不必要な Move が発生する可能性がある；後続で最適化可能
- ジェネリクスクロージャの捕獲分析には追加の処理が必要な可能性がある

## 設計決定記録

| 決定 | 決定内容 | 理由 | 日付 |
|------|------|------|------|
| 捕獲方式選択 | 全自动 | 関数呼び出しルールと統一 | 2026-05-29 |
| エスケープ分析 | 保守原則 | 判断できない場合はエスケープ、安全優先 | 2026-05-29 |
| ZST 最適化 | IR 生成時にスキップ | 後続の最適化パスよりシンプル | 2026-05-29 |
| 捕獲は型シグネチャに反映しない | コンパイラ内部処理 | lambda 型をシンプルに維持 | 2026-05-29 |

## 参考文献

### YaoXiang 公式ドキュメント

- [RFC-007: 関数構文の統一](./accepted/007-function-syntax-unification.md)
- [RFC-009: 所有権モデル v9](./accepted/009-ownership-model.md)
- [RFC-011: ジェネリクス型システム設計](./accepted/011-generic-type-system.md) — 第 2.4 節：Dup/Clone 組み込みマーカートレイト

### 外部参照

- [Rust クロージャ捕獲ルール](https://doc.rust-lang.org/reference/types/closure.html#capture-modes)
- [Swift クロージャ捕獲セマンティクス](https://docs.swift.org/swift-book/documentation/the-swift-programming-language/closures/#Capturing-Values)