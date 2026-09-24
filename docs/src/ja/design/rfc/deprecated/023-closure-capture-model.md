---
title: 'RFC-023: クロージャキャプチャモデル'
status: '廃止'
author: '晨煦'
created: '2026-05-29'
updated: '2026-06-16'
---

> **廃止理由**：2026-06-16 言語設計判断——Lambda/関数値は外側の変数を暗黙的にキャプチャせず、明示的なパラメータ渡しに変更。`spawn { }`
> は同一フレームで実行され、クロージャキャプチャは関与しない。本 RFC のキャプチャ解析システムは完全に削除された（約 850 行のコード）。コンテキスト依存の正解 = クロージャはパラメータのみ受け取る + カリー化は生成時点で固定（SPEC
> §12.3 / RFC-009 §2.3）。詳細は [RFC-009 設計判断](../accepted/009-ownership-model.md#設計判断記録)
> を参照。

# RFC-023: クロージャキャプチャモデル

> **参考**:
>
> - [RFC-007: 関数構文統一](../accepted/007-function-syntax-unification.md)
> - [RFC-009: 所有権モデル v9](../accepted/009-ownership-model.md)
> - [RFC-011: ジェネリックシステム設計](../accepted/011-generic-type-system.md)
>   — 第 2.4 節：Dup/Clone 組み込み marker trait

## 要約

本 RFC は YaoXiang 言語の**クロージャキャプチャモデル**を定義する。コンパイラはクロージャ本体が参照する外部変数を自動解析し、変数の型（Dup/非 Dup）とクロージャがエスケープするかに基づいてキャプチャ方式を自動選択する——Dup 型は直接コピー、非 Dup で非エスケープは借用、非 Dup でエスケープは Move。ユーザーは注釈不要で、関数呼び出しの自動借用選択と同じルールを共有する。

## 動機

### なぜ必要か？

現在のクロージャキャプチャは**空実装**——`MakeClosure` 命令の `env`
フィールドは常に空で、lambda は外部変数を一切参照できない。借用トークンシステムはクロージャが `&T`
トークン（ゼロコストコピー）をキャプチャできることを要求しており、これは中核的なユースケースである。

### 現在の問題

```yaoxiang
# このコードは現在コンパイルできない——lambda は threshold を参照できない
filter_by: (items: List(Point), threshold: &Float) -> List(Point) = {
    items.filter(|p| p.x > threshold)  # ❌ threshold をキャプチャできない
}
```

## 提案

### 中核設計

クロージャキャプチャはコンパイラが完全に自動判定する。ルールは関数呼び出しの自動借用選択と**完全に同一**：

```
変数型      クロージャのエスケープ  キャプチャ方式
─────────────────────────────────────────
Dup         任意                  コピー（ビットコピーまたはゼロコスト）
非 Dup      非エスケープ          自動借用（&T または &mut T）
非 Dup      エスケープ            Move（所有権移転）
```

**エスケープ判定**：

```
spawn { || ... }           → エスケープ
return || ...              → エスケープ
let x = || ... ;  x がフィールドに保存 → エスケープ
items.filter(|p| ...)      → 非エスケープ（同期高階関数呼び出し）
||.method()                → 非エスケープ（即座に呼び出し）
```

保守的原則：判定不能な場合はエスケープとして処理。

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
    # buf は非 Dup、filter は非エスケープ → &Buffer トークンを自動生成
    transform(|b| b.read())
    # クロージャ終了後にトークン解放、buf は再び使用可能
}

# 3. クロージャエスケープ——Move
spawn_worker: (data: Data) -> Void = {
    # data は非 Dup、spawn → エスケープ → Move
    spawn { use(data) }
}

# 4. 混合キャプチャ
complex: (items: List(Point), config: &Config, buf: Buffer) -> List(Point) = {
    # config: &Config → Dup → トークンをコピー
    # buf: Buffer → 非 Dup、非エスケープ → &mut Buffer を借用
    items.filter(|p| {
        let threshold = config.get_threshold()
        buf.update(p)
        p.x > threshold
    })
}

# 5. 借用競合検出
bad: (buf: Buffer) -> Void = {
    closure = |b| b.write()
    buf.read()  # ❌ buf は既にクロージャに借用されており、ここで競合
}
```

### 構文変更

**構文変更ゼロ**。キャプチャ方式はコンパイラが自動決定し、ユーザーは注釈不要。

## 詳細設計

### 型システムへの影響

Lambda の型署名は不変：`(params) -> Return`。キャプチャされる変数は型署名に反映されず、コンパイラが IR 生成段階で処理する。

### コンパイラ変更

| コンポーネント       | 変更                                         | 説明      |
| -------------------- | -------------------------------------------- | --------- |
| `capture.rs`（新規） | キャプチャ解析 + エスケープ解析 + モード選択 | 約 150 行 |
| `expressions.rs`     | lambda 型推論でキャプチャ解析を呼び出し      | 約 10 行  |
| `ir_gen.rs`          | MakeClosure env 充填；ZST スキップ           | 約 80 行  |
| `ir.rs`              | MakeClosure env 型の調整が必要な可能性       | 約 5 行   |

**キャプチャ解析フロー**：

```
1. lambda body AST を走査
2. すべての Expr::Var(name) 参照を収集
3. フィルタ：クロージャ外スコープの変数のみ保持
4. 分類：Read（読み取り専用）/ Write（読み書き）/ Move（移転）
5. 型属性を照会：Dup か否か
6. エスケープ判定：クロージャの使用方式
7. キャプチャモード選択：
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
            // クロージャ本体は外側を直接参照（コンパイル時消除）
        }
        Copy => {
            // Move dst, src を生成（Dup 型のシャローコピー）
        }
        Borrow => {
            // Borrow dst, src を生成（ReadToken 作成）
        }
        BorrowMut => {
            // Borrow dst, src を生成（WriteToken 作成）
        }
        Move => {
            // Move dst, src を生成（所有権移転）
        }
    }
}
```

### ランタイム動作

キャプチャ方式はランタイム性能に影響しない：

- **Dup + ZST**（例：`&T` トークン）→ 命令ゼロ、クロージャ本体は外側変数を直接参照
- **Dup + 非 ZST**（例：Int）→ レジスタコピー 1 回
- **Borrow/BorrowMut**→ トークン作成（コンパイル時概念、ゼロオーバーヘッド）
- **Move** → 通常の Move と同コスト

### 後方互換性

完全互換。現在すべての lambda は外部変数をキャプチャできないため、本 RFC は表現力を追加するだけで、既存コードを破壊しない。

## トレードオフ

### 利点

1. **注釈ゼロ**：ユーザーはキャプチャ注釈を書く必要がない
2. **関数呼び出しと統一**：キャプチャルール = 関数呼び出し自動借用ルール
3. **ゼロコスト**：Dup トークンのキャプチャはコンパイル時に完全に消除
4. **安全**：エスケープ解析により use-after-free を防止

### 欠点

1. **エスケープ解析が保守的**：判定不能な場合はエスケープとして処理するため、不必要な Move が発生する可能性
2. **暗黙的**：キャプチャ方式はコードに反映されないため、デバッグ時にコンパイル出力を確認する必要がある

## 代替案

| 案                                   | 選択しない理由                         |
| ------------------------------------ | -------------------------------------- |
| Rust 式明示的 `move` キーワード      | 新構文導入、ユーザーの認知負荷増加     |
| すべて Move                          | ゼロコストトークン借用を表現できない   |
| すべて借用                           | クロージャエスケープでダングリング参照 |
| ユーザーが手動でキャプチャ方式を注釈 | 「コンパイラ全自動」の設計哲学に反する |

## 実装戦略

### 段階区分

1. **Phase 1**：キャプチャ解析（外部変数参照の識別のみ、キャプチャ方式の区別なし）
2. **Phase 2**：エスケープ解析 + モード選択
3. **Phase 3**：IR 生成 + ZST 最適化
4. **Phase 4**：借用競合検出の統合

### 依存関係

- RFC-011（ジェネリックシステム、第 2.4 節 Dup/Clone trait）に依存——変数コピー可否の判定に Dup
  trait が必要
- RFC-009 v9（借用トークン）に依存——Borrow/BorrowMut キャプチャモードにトークン型が必要
- RFC-023 と本 RFC 実装後、借用トークンシステム（RFC-009 v9 実装）に着手可能

### リスク

- エスケープ解析が過度に保守的になり、不必要な Move が発生する可能性；後続の最適化で対応可能
- ジェネリッククロージャのキャプチャ解析に追加処理が必要な可能性

## 設計判断記録

| 判断                           | 決定                | 理由                             | 日付       |
| ------------------------------ | ------------------- | -------------------------------- | ---------- |
| キャプチャ方式選択             | 全自動              | 関数呼び出しルールと統一         | 2026-05-29 |
| エスケープ解析                 | 保守的原則          | 判定不能時はエスケープ、安全優先 | 2026-05-29 |
| ZST 最適化                     | IR 生成時にスキップ | 後続最適化パスより簡潔           | 2026-05-29 |
| キャプチャを型署名に反映しない | コンパイラ内部処理  | lambda 型の簡潔性維持            | 2026-05-29 |

## 参考文献

### YaoXiang 公式ドキュメント

- [RFC-007: 関数構文統一](../accepted/007-function-syntax-unification.md)
- [RFC-009: 所有権モデル v9](../accepted/009-ownership-model.md)
- [RFC-011: ジェネリックシステム設計](../accepted/011-generic-type-system.md)
  — 第 2.4 節：Dup/Clone 組み込み marker trait

### 外部参考

- [Rust クロージャキャプチャルール](https://doc.rust-lang.org/reference/types/closure.html#capture-modes)
- [Swift クロージャキャプチャセマンティクス](https://docs.swift.org/swift-book/documentation/the-swift-programming-language/closures/#Capturing-Values)
