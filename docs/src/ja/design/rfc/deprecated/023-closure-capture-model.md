---
title: 'RFC-023: クロージャキャプチャモデル'
status: '廃棄'
author: '晨煦'
created: '2026-05-29'
updated: '2026-06-16'
---

> **廃棄理由**：2026-06-16 言語設計意思決定——Lambda/関数値は外層変数を暗黙的にキャプチャせず、明示的なパラメータ渡しまたは関数適用（apply）を使用。`spawn { }` は同フレームで実行され、クロージャキャプチャを伴わない。本 RFC のキャプチャ分析システムは完全に削除された（~850 行のコード）。
> 文脈依存の正解 = クロージャはパラメータのみを受け取り + 柯里化は作成時点で固化（SPEC §12.3 / RFC-009 §2.3）。
> 詳細は [RFC-009 設計意思決定記録](../accepted/009-ownership-model.md#設計意思決定記録) を参照。

# RFC-023: クロージャキャプチャモデル

> **参考**:
>
> - [RFC-007: 関数構文統一](../accepted/007-function-syntax-unification.md)
> - [RFC-009: 所有権モデル v9](../accepted/009-ownership-model.md)
> - [RFC-011: ジェネリクスシステム設計](../accepted/011-generic-type-system.md)
>   — 第 2.4 節：Dup/Clone 組み込みマーカートレイト

## 摘要

本 RFC は YaoXiang 言語の**クロージャキャプチャモデル**を定義する。コンパイラはクロージャ体が参照する外部変数を自動分析し、変数の型（Dup/非Dup）およびクロージャがエスケープするかどうかにより、キャプチャ方式を自動選択する——Dup 型は直接コピー、非 Dup で非エスケープなら借用、非 Dup でエスケープなら Move。ユーザーは注釈なしで済み、関数呼び出しの自動借用選択と同一のルールを共有する。

## 動機

### なぜ必要か？

現在のクロージャキャプチャは**未実装**——`MakeClosure` 命令の `env`
フィールドは常に空であり、lambda は外部変数を参照できない。借用トークンシステムは閉包による `&T`
トークン（ゼロコストコピー）のキャプチャを要求しており、これはコアな使用シナリオである。

### 現在の問題

```yaoxiang
# このようなコードは現在コンパイル不可——lambda は threshold を参照できない
filter_by: (items: List(Point), threshold: &Float) -> List(Point) = {
    items.filter(|p| p.x > threshold)  # ❌ threshold をキャプチャできない
}
```

## 提案

### 中核設計

クロージャキャプチャはコンパイラが全自动で判断する。ルールは関数呼び出しの自動借用選択と**完全に同一**である：

```
変数の型    クロージャがエスケープ    キャプチャ方式
─────────────────────────────────────────
Dup         任意                      コピー（ビットコピーまたはゼロコスト）
非 Dup      非エスケープ              自動借用（&T または &mut T）
非 Dup      エスケープ                Move（所有権移転）
```

**エスケープ判定**：

```
spawn { || ... }           → エスケープ
return || ...              → エスケープ
let x = || ... ;  x 存字段 → エスケープ
items.filter(|p| ...)      → 非エスケープ（sync 高階関数呼び出し）
||.method()                → 非エスケープ（当场呼び出し）
```

保守原則：判断できない場合はエスケープとして扱う。

### 例

```yaoxiang
# 1. Dup トークン——直接コピー（ゼロコスト）
filter_by: (items: List(Point), threshold: &Float) -> List(Point) = {
    # threshold: &Float → Dup → コンパイラがトークンをクロージャにコピー
    # ゼロサイズトークン、ゼロランタイムオーバーヘッド
    items.filter(|p| p.x > threshold)
}

# 2. 非 Dup + 非エスケープ——自動借用
process: (buf: Buffer) -> Void = {
    # buf は Dup でない、filter はエスケープしない → 自動作成 &Buffer トークン
    transform(|b| b.read())
    # クロージャ返回後トークン開放、buf は再度使用可能
}

# 3. クロージャがエスケープ——Move
spawn_worker: (data: Data) -> Void = {
    # data は Dup でない、spawn → エスケープ → Move
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

# 5. 借用競合検出
bad: (buf: Buffer) -> Void = {
    closure = |b| b.write()
    buf.read()  # ❌ buf は既にクロージャに借用されており、ここで競合発生
}
```

### 構文変更

**ゼロ構文変更**。キャプチャ方式是コンパイラが自動决定し、ユーザーは注釈を必要としない。

## 詳細設計

### 型システムへの影響

Lambda の型署名は変更なし：`(params) -> Return`。キャプチャされた変数は型署名に反映されず、コンパイラが IR 生成段階で処理する。

### コンパイラ変更

| コンポーネント           | 変更内容                                   | 規模     |
| ------------------------ | ------------------------------------------ | -------- |
| `capture.rs`（新規作成） | キャプチャ分析 + エスケープ分析 + モード選択 | ~150 行  |
| `expressions.rs`         | lambda 型推論でキャプチャ分析を呼び出す     | ~10 行   |
| `ir_gen.rs`              | MakeClosure env 填充；ZST スキップ          | ~80 行   |
| `ir.rs`                  | MakeClosure env 型調整が必要な可能性あり   | ~5 行    |

**キャプチャ分析フロー**：

```
1. lambda body AST を巡回
2. すべての Expr::Var(name) 参照を収集
3. フィルター：クロージャの外部スコープの変数のみを保持
4. 分類：Read（読み取り専用）/ Write（読み書き）/ Move（移転済み）
5. 型属性を查询：Dup かどうか
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
            // クロージャ体は直接外層変数を参照（コンパイル時に排除）
        }
        Copy => {
            // Move dst, src を生成（Dup 型のシャローコピー）
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

### ランタイム動作

キャプチャ方式はランタイム性能に影響しない：

- **Dup + ZST**（例：`&T` トークン）→ ゼロ命令、クロージャ体は外層変数を直接参照
- **Dup + 非 ZST**（例：Int）→ 1 回のレジスタコピー
- **Borrow/BorrowMut**→ トークンを作成（コンパイル時概念、ゼロオーバーヘッド）
- **Move** → 通常の Move と同じコスト

### 後方互換性

完全互換。現在すべての lambda は外部変数をキャプチャできず、本 RFC は表現力を追加するだけで、既存のコードを一切壊さない。

## 权衡

### 利点

1. **ゼロ注釈**：ユーザーはキャプチャ注釈を何も書く必要がない
2. **関数呼び出しと統一**：キャプチャ規則 = 関数呼び出しの自動借用規則
3. **ゼロコスト**：Dup トークンのキャプチャは完全にコンパイル時に排除される
4. **安全**：エスケープ分析が use-after-free を防ぐ

### 欠点

1. **エスケープ分析が保守的**：判断できない場合はエスケープ扱いになり、不要な Move が発生する可能性がある
2. **暗黙的**：キャプチャ方式是コードに反映されず、デバッグ時にコンパイル出力を確認する必要がある

## 代替案

| 案                             | 選択しない理由                             |
| ------------------------------ | ------------------------------------------ |
| Rust 形式の明示的 `move` キーワード | 新構文の導入、ユーザーの認知負荷増加       |
| すべて Move                    | ゼロコストトークン借用を表現できない       |
| すべて借用                     | クロージャのエスケープでdangling参照を生成 |
| ユーザーが手動でキャプチャ方式を注釈 | 「コンパイラ全自动」の設計思想に反する     |

## 実装戦略

### フェーズ分け

1. **Phase 1**：キャプチャ分析（外部変数参照の識別のみ、キャプチャ方式の区別なし）
2. **Phase 2**：エスケープ分析 + モード選択
3. **Phase 3**：IR 生成 + ZST 最適化
4. **Phase 4**：借用競合検出の統合

### 依存関係

- RFC-011（ジェネリクスシステム、第 2.4 節 Dup/Clone トレイト）に依存——変数の複製可能性判断に Dup トレイトが必要
- RFC-009 v9（借用トークン）に依存——Borrow/BorrowMut キャプチャモードはトークン型を必要とする
- RFC-023 と本 RFC 実装後、借用トークンシステム（RFC-009 v9 実装）に着手可能

### リスク

- エスケープ分析が過度に保守的になり、不要な Move が発生する可能性がある——後続で最適化可能
- ジェネリクスクロージャのキャプチャ分析は追加処理が必要な可能性あり

## 設計意思決定記録

| 意思決定                   | 決定           | 理由                               | 日付       |
| -------------------------- | -------------- | ---------------------------------- | ---------- |
| キャプチャ方式選択         | 全自动         | 関数呼び出し規則と統一             | 2026-05-29 |
| エスケープ分析             | 保守原則       | 判断できない場合はエスケープ、安全優先 | 2026-05-29 |
| ZST 最適化                 | IR 生成時にスキップ | 後続の最適化パスより簡单           | 2026-05-29 |
| キャプチャを型署名に反映しない | コンパイラ内部処理 | lambda 型を簡洁に維持             | 2026-05-29 |

## 参考文献

### YaoXiang 公式ドキュメント

- [RFC-007: 関数構文統一](../accepted/007-function-syntax-unification.md)
- [RFC-009: 所有権モデル v9](../accepted/009-ownership-model.md)
- [RFC-011: ジェネリクスシステム設計](../accepted/011-generic-type-system.md) — 第 2.4 節： Dup/Clone 組み込みマーカートレイト

### 外部参照

- [Rust クロージャキャプチャ規則](https://doc.rust-lang.org/reference/types/closure.html#capture-modes)
- [Swift クロージャキャプチャセマンティクス](https://docs.swift.org/swift-book/documentation/the-swift-programming-language/closures/#Capturing-Values)