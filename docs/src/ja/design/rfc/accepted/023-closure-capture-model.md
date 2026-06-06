```markdown
---
title: "RFC-023: クロージャキャプチャモデル"
status: "承認済み"
author: "晨煦"
created: "2026-05-29"
updated: "2026-05-29"
---

# RFC-023: クロージャキャプチャモデル

> **参考**:
> - [RFC-007: 関数構文の統一](./accepted/007-function-syntax-unification.md)
> - [RFC-009: 所有権モデル v9](./accepted/009-ownership-model.md)
> - [RFC-011: ジェネリック型システム設計](./accepted/011-generic-type-system.md) — 第 2.4 節： Dup/Clone 組み込みマーカtrait

## 要約

本 RFC は YaoXiang 言語の**クロージャキャプチャモデル**を定義する。コンパイラはクロージャボディが参照する外部変数を自動分析し、変数の型（Dup/非Dup）およびクロージャがエスケープするかどうかに応じて、キャプチャ方式を自動選択する——Dup 型は直接コピー、非 Dup で非エスケープの場合は借用、非 Dup でエスケープの場合は Move となる。ユーザーはゼロ\Annotation} 하며、関数呼び出しの自動借用選択と同一のルールを共有する。

## 動機

### なぜ必要か？

現在、クロージャキャプチャは**空実装**——`MakeClosure` 命令の `env` フィールドは常に空であり、lambda は外部変数を参照できない。所有権トークンシステムは `&T` トークン（ゼロコストコピー）をキャプチャできるクロージャを必要としており、これはコアな使用シナリオである。

### 現在の問題

```yaoxiang
# このようなコードは現在コンパイル不可——lambda は threshold を参照できない
filter_by: (items: List(Point), threshold: &Float) -> List(Point) = {
    items.filter(|p| p.x > threshold)  # ❌ threshold がキャプチャできない
}
```

## 提案

### コアデザイン

クロージャキャプチャはコンパイラが全自动的に判断する。ルールは関数呼び出しの自動借用選択と**完全に同一**である：

```
変数の型    クロージャがエスケープ    キャプチャ方式
──────────────────────────────────────────────────────────
Dup         任意                    コピー（ビットコピーまたはゼロコスト）
非 Dup      エスケープしない        自動借用（&T または &mut T）
非 Dup      エスケープする          Move（所有権移転）
```

**エスケープ判定**：

```
spawn { || ... }           → エスケープ
return || ...              → エスケープ
let x = || ... ;  x をフィールドに格納 → エスケープ
items.filter(|p| ...)      → エスケープしない（sync 高階関数呼び出し）
||.method()                → エスケープしない（即時呼び出し）
```

保守原則：判断できない場合はエスケープとして扱う。

### 例

```yaoxiang
# 1. Dup トークン——直接コピー（ゼロコスト）
filter_by: (items: List(Point), threshold: &Float) -> List(Point) = {
    # threshold: &Float → Dup → コンパイラがトークンをクロージャにコピー
    # ゼロサイズトークン、ゼロ実行時オーバーヘッド
    items.filter(|p| p.x > threshold)
}

# 2. 非 Dup + エスケープなし——自動借用
process: (buf: Buffer) -> Void = {
    # buf は Dup でない、filter はエスケープしない → 自動的に &Buffer トークンを作成
    transform(|b| b.read())
    # クロージャ終了後にトークン解放、buf が再び利用可能に
}

# 3. クロージャがエスケープ——Move
spawn_worker: (data: Data) -> Void = {
    # data は Dup でない、spawn → エスケープ → Move
    spawn { use(data) }
}

# 4. 混合キャプチャ
complex: (items: List(Point), config: &Config, buf: Buffer) -> List(Point) = {
    # config: &Config → Dup → トークンをコピー
    # buf: Buffer → Dup でない、エスケープしない → &mut Buffer 借用
    items.filter(|p| {
        let threshold = config.get_threshold()
        buf.update(p)
        p.x > threshold
    })
}

# 5. 借用競合検出
bad: (buf: Buffer) -> Void = {
    closure = |b| b.write()
    buf.read()  # ❌ buf はすでにクロージャに借用されている、ここでは競合発生
}
```

### 構文の変更

**ゼロ構文変更**。キャプチャ方式はコンパイラが自動的に決定し、ユーザーが\Annotation}する必要はない。

## 詳細設計

### 型システムへの影響

Lambda の型署名は変わらない：`{params) -> Return`。キャプチャされた変数は型署名に反映されず、コンパイラの IR 生成段階で処理される。

### コンパイラの更改

| コンポーネント | 更改 | 説明 |
|------|------|------|
| `capture.rs`（新規作成） | キャプチャ分析 + エスケープ分析 + パターン選択 | 約 150 行 |
| `expressions.rs` | lambda 型推論でキャプチャ分析を呼び出す | 約 10 行 |
| `ir_gen.rs` | MakeClosure env への詰める；ZST スキップ | 約 80 行 |
| `ir.rs` | MakeClosure env 型を調整する必要がある可能性 | 約 5 行 |

**キャプチャ分析フロー**：

```
1. lambda body AST を走査
2. すべての Expr::Var(name) 参照を収集
3. フィルタリング：クロージャの外部スコープの変数のみ保持
4. 分類：Read（読み取り専用）/ Write（読み書き）/ Move（移転済み）
5. 型属性を照会： Dup かどうか
6. エスケープ判定：クロージャの使用方式
7. キャプチャモードを選択：
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
            // ゼロサイズ型——命令を生成しない
            // クロージャボディは外層変数を直接参照（コンパイル時に除去）
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

### 実行時動作

キャプチャ方式は実行時パフォーマンスに影響しない：

- **Dup + ZST**（例：`&T` トークン）→ ゼロ命令、クロージャボディは外層変数を直接参照
- **Dup + 非 ZST**（例：Int）→ 1 回のレジスタコピー
- **Borrow/BorrowMut**→ トークン作成（コンパイル時コンセプト、ゼロオーバーヘッド）
- **Move** → 通常の Move と同様のコスト

### 後方互換性

完全に互換性あり。現在のすべての lambda は外部変数をキャプチャできず、本 RFC は表現力を追加するだけで、既存のコードを破壊しない。

## トレードオフ

### メリット

1. **ゼロ\Annotation}**：ユーザーはキャプチャ\Annotation}を書く必要がない
2. **関数呼び出しとの統一**：キャプチャルール = 関数呼び出しの自動借用ルール
3. **ゼロコスト**：Dup トークンのキャプチャは完全にコンパイル時に除去
4. **安全**：エスケープ分析により use-after-free を防止

### デメリット

1. **エスケープ分析が保守的**：判断できない場合はエスケープとして扱い、不要な Move が発生する可能性がある
2. **暗黙的**：キャプチャ方式はコードに反映されず、デバッグ時はコンパイル出力を確認する必要がある

## 代替案

| 案 | なぜ選択しないか |
|------|------------------|
| Rust 形式の明示的 `move` キーワード | 新しい構文を導入し、ユーザーの認知的負荷を増加させる |
| すべて Move | ゼロコストトークン借用を表現できない |
| すべて借用 | クロージャがエスケープするとdangling 参照が発生する |
| ユーザーが手動でキャプチャ方式を\Annotation} | 「コンパイラが全自动」を التصميم}哲学に反する |

## 実装戦略

### 段階的划分

1. **Phase 1**：キャプチャ分析（外部変数参照の識別のみ、キャプチャ方式の区別なし）
2. **Phase 2**：エスケープ分析 + モード選択
3. **Phase 3**：IR 生成 + ZST 最適化
4. **Phase 4**：借用競合検出の統合

### 依存関係

- RFC-011（ジェネリックシステム、第 2.4 節 Dup/Clone trait）に依存——変数がコピー可能かどうかの判断に Dup trait が必要
- RFC-009 v9（借用トークン）に依存——Borrow/BorrowMut キャプチャモードにはトークン型が必要
- RFC-023 および本 RFC 実装後、借用トークンシステム（RFC-009 v9 実装）に着手可能

### リスク

- エスケープ分析が過度に保守的で、不要な Move が発生する可能性がある；これは後続の最適化で改善可能
- ジェネリッククロージャのキャプチャ分析には追加の処理が必要な可能性がある

## 設計決定記録

| 決定 | 決定内容 | 理由 | 日付 |
|------|------|------|------|
| キャプチャ方式選択 | 全自動 | 関数呼び出しルールとの統一 | 2026-05-29 |
| エスケープ分析 | 保守原則 | 判断できない場合はエスケープ、安全優先 | 2026-05-29 |
| ZST 最適化 | IR 生成時にスキップ | 後続の最適化パスよりシンプル | 2026-05-29 |
| キャプチャを型署名に反映しない | コンパイラの内部処理 | lambda 型の簡潔さを維持 | 2026-05-29 |

## 参考文献

### YaoXiang 公式ドキュメント

- [RFC-007: 関数構文の統一](./accepted/007-function-syntax-unification.md)
- [RFC-009: 所有権モデル v9](./accepted/009-ownership-model.md)
- [RFC-011: ジェネリック型システム設計](./accepted/011-generic-type-system.md) — 第 2.4 節： Dup/Clone 組み込みマーカtrait

### 外部参照

- [Rust クロージャキャプチャルール](https://doc.rust-lang.org/reference/types/closure.html#capture-modes)
- [Swift クロージャキャプチャセマンティクス](https://docs.swift.org/swift-book/documentation/the-swift-programming-language/closures/#Capturing-Values)
```