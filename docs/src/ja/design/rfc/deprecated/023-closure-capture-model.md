---
title: 'RFC-023: クロージャ捕獲モデル'
status: '廃止'
author: '晨煦'
created: '2026-05-29'
updated: '2026-06-16'
---

> **廃止理由**：2026-06-16 言語設計の意思決定——Lambda/関数値は外層変数を暗黙的に捕獲せず、明示的なパラメータ引き渡しを使用する。`spawn { }`
> は同フレームで実行され、クロージャ捕獲は関係ない。本 RFC の捕獲分析システムは完全に削除された（~850 行のコード）。詳細は
> [RFC-009 設計意思決定](../accepted/009-ownership-model.md#設計意思決定記録) を参照。

# RFC-023: クロージャ捕獲モデル

> **参考**:
>
> - [RFC-007: 関数構文統一](./accepted/007-function-syntax-unification.md)
> - [RFC-009: 所有権モデル v9](./accepted/009-ownership-model.md)
> - [RFC-011: ジェネリクスシステム設計](./accepted/011-generic-type-system.md)
>   — 第 2.4 節：Dup/Clone 組み込み marker trait

## 摘要

本 RFC は YaoXiang 言語の**クロージャ捕獲モデル**を定義する。コンパイラはクロージャ体が参照する外部変数を自動分析し、変数の型（Dup/非Dup）およびクロージャがエスケープするかどうかにより、捕獲方式を自動選択する——Dup 型は直接コピー、非 Dup で非エスケープの場合は借用、非 Dup でエスケープの場合は Move。ユーザーはアノテーション不要で、関数呼び出しの自動借用選択と同一のルールを共有する。

## 動機

### なぜ必要か？

現在のクロージャ捕獲は**空実装**——`MakeClosure` 命令の `env`
フィールドは常に空であり、lambda は外部変数を参照できない。所有権トークンシステムはクロージャが `&T`
トークン（ゼロコストコピー）を捕獲できる必要があるため、これはコアな使用シナリオである。

### 現在の問題

```yaoxiang
# このコードは現在コンパイル不可——lambda は threshold を参照できない
filter_by: (items: List(Point), threshold: &Float) -> List(Point) = {
    items.filter(|p| p.x > threshold)  # ❌ threshold を捕獲できない
}
```

## 提案

### コア設計

クロージャ捕獲はコンパイラが全自动的に判断する。ルールは関数呼び出しの自動借用選択と**完全に同一**である：

```
変数型    クロージャがエスケープ    捕獲方式
─────────────────────────────────────────
Dup       任意                    コピー（ビットコピーまたはゼロコスト）
非 Dup    エスケープしない        自動借用（&T または &mut T）
非 Dup    エスケープ              Move（所有権移転）
```

**エスケープ判定**：

```
spawn { || ... }           → エスケープ
return || ...              → エスケープ
let x = || ... ;  x をフィールドに格納 → エスケープ
items.filter(|p| ...)      → エスケープしない（sync 高階関数呼び出し）
||.method()                → エスケープしない（当场呼び出し）
```

保守的原則：判断できない場合はエスケープとして扱う。

### 示例

```yaoxiang
# 1. Dup トークン——直接コピー（ゼロコスト）
filter_by: (items: List(Point), threshold: &Float) -> List(Point) = {
    # threshold: &Float → Dup → コンパイラがトークンをクロージャにコピー
    # ゼロサイズトークン、ゼロランタイムオーバーヘッド
    items.filter(|p| p.x > threshold)
}

# 2. 非 Dup + 非エスケープ——自動借用
process: (buf: Buffer) -> Void = {
    # buf は Dup でない、filter はエスケープしない → 自動生成 &Buffer トークン
    transform(|b| b.read())
    # クロージャ返回後トークン解放、buf は再利用可
}

# 3. クロージャがエスケープ——Move
spawn_worker: (data: Data) -> Void = {
    # data は Dup でない、spawn → エスケープ → Move
    spawn { use(data) }
}

# 4. 混合捕獲
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
    buf.read()  # ❌ buf は既にクロージャに借用されている、ここでは競合発生
}
```

### 構文変化

**構文変化ゼロ**。捕獲方式はコンパイラが自動決定し、ユーザーがアノテーションを書く必要はない。

## 詳細設計

### 型システムへの影響

Lambda の型署名は変わらない：(params) ->
Return。捕獲された変数は型署名に反映されず、コンパイラが IR 生成段階で処理する。

### コンパイラ変更

| コンポーネント       | 変更内容                               | 説明    |
| -------------------- | -------------------------------------- | ------- |
| `capture.rs`（新規） | 捕獲分析 + エスケープ分析 + モード選択 | ~150 行 |
| `expressions.rs`     | lambda 型推論で捕獲分析を呼叫          | ~10 行  |
| `ir_gen.rs`          | MakeClosure env 填充；ZST スキップ     | ~80 行  |
| `ir.rs`              | MakeClosure env 型調整が必要かも       | ~5 行   |

**捕獲分析フロー**：

```
1. lambda body AST を巡回
2. すべての Expr::Var(name) 参照を収集
3. フィルタ：クロージャの外部スコープの変数のみを保持
4. 分類：Read（読み取り専用）/ Write（読み書き）/ Move（移転済み）
5. 型属性を查询：Dup かどうか
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

// 変更後
Instruction::MakeClosure { dst, func, env: captured_env }

// captured_env 生成ロジック：
for captured in captures {
    match captured.mode {
        Copy if is_zst(captured.ty) => {
            // ゼロサイズ型——命令を生成しない
            // クロージャ体は直接外層を参照（コンパイル時に消去）
        }
        Copy => {
            // Move dst, src を生成（Dup 型の浅コピー）
        }
        Borrow => {
            // Borrow dst, src を生成（ReadToken 生成）
        }
        BorrowMut => {
            // Borrow dst, src を生成（WriteToken 生成）
        }
        Move => {
            // Move dst, src を生成（所有権移転）
        }
    }
}
```

### ランタイム動作

捕獲方式はランタイムパフォーマンスに影響しない：

- **Dup + ZST**（如 `&T` トークン）→ ゼロ命令、クロージャ体は直接外層変数を参照
- **Dup + 非 ZST**（如 Int）→ 1 回レジスタコピー
- **Borrow/BorrowMut**→ トークン生成（コンパイル時コンセプト、ゼロオーバーヘッド）
- **Move** → 通常 Move と同じコスト

### 後方互換性

完全互換。現在すべての lambda は外部変数を捕獲できないため、本 RFC は表現力を追加するだけで、既存のコードを破壊しない。

## 权衡

### 优点

1. **ゼロアノテーション**：ユーザーは捕獲アノテーションを書く必要がない
2. **関数呼び出しとの統一**：捕獲ルール = 関数呼び出しの自動借用ルール
3. **ゼロコスト**：Dup トークンの捕獲は完全にコンパイル時に消去
4. **安全**：エスケープ分析により use-after-free を防止

### 欠点

1. **エスケープ分析が保守的**：判断できない場合はエスケープとして扱い、不必要な Move を行う可能性がある
2. **暗黙的**：捕獲方式是はコードに反映されず、デバッグ時はコンパイル出力を確認する必要がある

## 代替方案

| 方案                                     | 为什么不選択                           |
| ---------------------------------------- | -------------------------------------- |
| Rust 式の明示的 `move` キーワード        | 新構文導入、ユーザーの認知的負荷増加   |
| すべて Move                              | ゼロコストトークン借用を表現できない   |
| すべて借用                               | クロージャのエスケープ导致垂れ参照     |
| ユーザーが手動で捕獲方式をアノテーション | 「コンパイラ全自动」の設計哲学に反する |

## 実装策略

### 段階划分

1. **Phase 1**：捕獲分析（外部変数参照のみ識別、捕獲方式是は区別しない）
2. **Phase 2**：エスケープ分析 + モード選択
3. **Phase 3**：IR 生成 + ZST 最適化
4. **Phase 4**：借用競合検出統合

### 依存関係

- RFC-011（ジェネリクスシステム、第 2.4 節 Dup/Clone
  trait）に依存——変数がコピー可能か判断するために Dup trait が必要
- RFC-009 v9（借用トークン）に依存——Borrow/BorrowMut 捕獲モードにはトークン型が必要
- RFC-023 と本 RFC 実装後、借用トークンシステム（RFC-009 v9 実装）に 착수 可能

### リスク

- エスケープ分析が過度に保守的になり、不要な Move が発生する可能性がある；后续 оптимизация 可能
- ジェネリクスクロージャの捕獲分析には追加処理が必要かもしれない

## 設計决策記録

| 决策                       | 決定                | 原因                                   | 日付       |
| -------------------------- | ------------------- | -------------------------------------- | ---------- |
| 捕獲方式是選択             | 全自动              | 関数呼び出しルールと統一               | 2026-05-29 |
| エスケープ分析             | 保守的原則          | 判断できない場合はエスケープ、安全優先 | 2026-05-29 |
| ZST 最適化                 | IR 生成時にスキップ | 後続最適化 pass よりシンプル           | 2026-05-29 |
| 捕獲は型署名に反映されない | コンパイラ内部処理  | lambda 型をシンプルに保持              | 2026-05-29 |

## 参考文献

### YaoXiang 公式ドキュメント

- [RFC-007: 関数構文統一](./accepted/007-function-syntax-unification.md)
- [RFC-009: 所有権モデル v9](./accepted/009-ownership-model.md)
- [RFC-011: ジェネリクスシステム設計](./accepted/011-generic-type-system.md)
  — 第 2.4 節：Dup/Clone 組み込み marker trait

### 外部参考文献

- [Rust クロージャ捕獲ルール](https://doc.rust-lang.org/reference/types/closure.html#capture-modes)
- [Swift クロージャ捕獲意味論](https://docs.swift.org/swift-book/documentation/the-swift-programming-language/closures/#Capturing-Values)
