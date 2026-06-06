```markdown
---
title: "RFC-019: 型レベル同像性 (Typed Homoiconicity) - 構文即型"
status: "草案"
author: "晨煦"
created: "2026-02-20"
updated: "YYYY-MM-DD"
---

# RFC-019: 型レベル同像性 (Typed Homoiconicity) - 構文即型

>

>

>
> **⚠️ 恒久的実験的宣言**: これは**探索的実験**であり、「構文即型」という言語設計理念の可行性を検証するためのものである。**本 RFC は決してマージされない**。結果にかかわらず、dev/main ブランチには反映されない。実験ブランチは完了後に破棄またはアーカイブされる。
>
> - **実験目標**：型レベル同像性の実装難易度と潜在価値を検証する
> - **損切りライン**：6ヶ月進展なしの場合は諦める
> - **成功基準**：少なくとも1つのユーザー定義キーワードが（完全解析→コンパイル→実行の）完走できること
>
> **main ブランチへのマージは保証されない**。様々な理由により拒否または放棄される可能性がある。本機能を本番環境で使用しないこと。
>
> **⚠️ 位置づけの説明**: 本 RFC は言語設計の思考実験でありエンジニアリングソリューションを提供しない。実用的な拡張可能パーサーが必要な場合は、Rust の `syn::Parse` や Haskell の `parsec` を参照すること。

---

## 抄録

本 RFC は革新的な言語設計実験を提案する：**言語の構文構造そのものを型システムの一部とする**。

中心的なアイデアは Lisp の「コード即データ」（同像性）に由来するが、**静的型システム**によって実装される：
- 構文木（AST）は型である
- キーワードは型の事前定義インスタンスである
- ユーザーは型を定義することで言語構文を拡張できる

つまり：言語本身就是组成可能で拡張可能な「ビルディングブロック」になる。

---

## 動機

### なぜこの実験を行うのか？

1. **統一性の追求**：「キーワード」という特殊な構文要素を排除し、すべてを型と関数にする
2. **言語の拡張性**：ユーザーが関数を定義するのと同様に新しい構文構造を定義できる
3. **型安全なマクロ**：従来のマクロ（テキスト置換）は危険だが、型レベル同像性はコンパイル時チェックを提供できる
4. **学習目的**：言語設計の本質を深く理解する

### Lisp との関係

Lisp はすでに「コード即データ」を実現している：
```lisp
; Lisp コードそのものが S-expression
(if (> x 0) "positive" "negative")
```

本実験の差異は：**この理念を静的型システムで強化する**点にある。

---

## 提案

### 中心概念

#### 1. 構文木即型（AST as Type）

```yaoxiang
// 構文木のノードはすべて型である
If: Type = { condition: Expr, then: Block, else: Block }
While: Type = { condition: Expr, body: Block }
Return: Type = { value: Expr }
Block: Type = { statements: Array[Expr] }
Let: Type = { name: String, value: Expr, body: Expr }
Function: Type = { params: Array[Param], body: Expr }
Call: Type = { func: Expr, args: Array[Expr] }

// 基本型
Literal: Type = { value: Int }
StringLiteral: Type = { value: String }
Variable: Type = { name: String }
```

#### 2. キーワード = 型を処理する関数

```yaoxiang
// 評価器はこれらの型を処理する関数である
eval_if: (node: If, env: Env) -> Value = ...
eval_while: (node: While, env: Env) -> Value = ...
eval_return: (node: Return, env: Env) -> Value = ...
eval_block: (node: Block, env: Env) -> Value = ...

// コンパイラも関数にできる
compile_if: (node: If, ctx: CompileContext) -> IR = ...
compile_while: (node: While, ctx: CompileContext) -> IR = ...
```

#### 3. 型が解析規則を携带（中心的イノベーション）

これが本実験の鍵である：**型はデータを記述するだけでなく、コードをどのように解析するかの規則も携带する**。

```yaoxiang
// 構文規則型
SyntaxRule: Type = {
    // この型のコードをどのように解析するか
    parse: (token_stream: TokenStream) -> (Self, remaining_tokens)

    // 型インスタンスをどのようにコンパイル/評価するか
    compile: (node: Self, ctx: CompileContext) -> IR
    eval: (node: Self, env: Env) -> Value
}

// IF 型の構文規則
IF: SyntaxRule = {
    // "if (cond) { then } else { else }" を解析
    parse: (tokens: TokenStream) -> (If, remaining) = {
        consume("if")
        cond = parse_expression(tokens)
        consume("{")
        then_block = parse_block(tokens)
        consume("}")
        consume("else")
        consume("{")
        else_block = parse_block(tokens)
        consume("}")
        return If(cond, then_block, else_block), tokens
    }

    eval: (node: If, env: Env) -> Value = {
        if eval(node.condition, env) != 0 {
            return eval(node.then, env)
        } else {
            return eval(node.else, env)
        }
    }
}
```

#### 4. ユーザー定義構文拡張

ユーザーは独自の「キーワード」を定義できる：

```yaoxiang
// ユーザーが新しい構文構造を定義：unless
Unless: SyntaxRule = {
    parse: (tokens: TokenStream) -> (If, remaining) = {
        consume("unless")
        cond = parse_expression(tokens)
        consume("{")
        body = parse_block(tokens)
        consume("}")
        // unless は if (!cond) { body } と同等
        return If(Not(cond), body, Block([])), tokens
    }
}

// 使用例
unless x > 0 {
    print("x is not positive")
}

// 展開後
if !(x > 0) {
    print("x is not positive")
}
```

### 例

#### 完全例：カスタムループ構文

```yaoxiang
// "times" ループを定義：n.times { ... } を n 回実行
TimesLoop: SyntaxRule = {
    parse: (tokens: TokenStream) -> (While, remaining) = {
        receiver = parse_expression(tokens)  // 数値を取得
        consume(".times")
        consume("{")
        body = parse_block(tokens)
        consume("}")
        // while ループに変換
        counter_var = gensym("i")
        return While(
            Less(Variable(counter_var), receiver),
            Block([
                body,
                Assign(counter_var, Add(Variable(counter_var), Literal(1)))
            ])
        ), tokens
    }
}

// 使用例
5.times {
    print("Hello!")
}

// 展開後
i = 0
while i < 5 {
    print("Hello!")
    i = i + 1
}
```

#### 例：パターン照合構文

```yaoxiang
// ユーザーがパターン照合を定義
Match: SyntaxRule = {
    parse: (tokens: TokenStream) -> (MatchNode, remaining) = {
        subject = parse_expression(tokens)
        consume("{")
        cases = []
        while !check("}") {
            pattern = parse_pattern(tokens)
            consume("=>")
            body = parse_expression(tokens)
            cases.push((pattern, body))
        }
        consume("}")
        return MatchNode(subject, cases), tokens
    }
}

// 使用例
match x {
    0 => "zero",
    1 => "one",
    n if n > 10 => "big",
    _ => "other"
}
```

---

## 詳細設計

### システムアーキテクチャ

```
┌─────────────────────────────────────────────────────┐
│                    ソースコード                       │
└─────────────────┬───────────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────────┐
│              構文解析器 (Parser)                      │
│  - キーワードの識別                                   │
│  - 対応する SyntaxRule 型の検索                       │
│  - 型の parse メソッド呼び出し                        │
└─────────────────┬───────────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────────┐
│              AST (型インスタンス)                     │
│  If, While, Match, TimesLoop...                     │
└─────────────────┬───────────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────────┐
│              コンパイラ/インタープリタ                 │
│  - 型の compile/eval メソッド呼び出し                 │
│  - ターゲットコード生成または実行                     │
└─────────────────────────────────────────────────────┘
```

### 主要技術的課題

#### 1. 制御流の関数化

問題：`if` は片方のブランチのみを評価する必要があるため、普通の関数呼び出しは使えない。

解決策：thunk（遅延評価）を渡す

```yaoxiang
// コンパイル後の内部表現
If: Type = {
    condition: Expr,
    then: () -> Value,  // thunk、遅延評価
    else: () -> Value
}
```

#### 2. return の非局所的リターン

問題：`return` は多层の関数を脱出する必要がある。

解決策：
- 案 A：コンパイル時 CPS 変換
- 案 B：Result/Either モナドを使用
- 案 C：return のスコープを制限

#### 3. 構文曖昧性

問題：`if(x > 0) { 1 }` を関数呼び出しとキーワードのどちらとして扱うか？

解決策：
- キーワードには特殊構文を使用（例：`if ... { } else { }`）
- または型システムで制約

#### 4. 無限再帰

問題：ユーザーが自己参照する構文規則を定義する可能性がある。

解決策：コンパイル時に循環依存を検出

---

## 既存システムとの関係

### RFC-010（統一型構文）との関係

RFC-010 は `name: type = value` の統一構文を実装しており、本 RFC はその延長である：

| RFC-010 | 本 RFC |
|----------|--------|
| 変数、関数、型はすべて `name: type = value` | キーワードも `name: type = value` |
| 型は値 | 構文規則も値 |
| `Type` はメタ型 | `SyntaxRule` は構文のメタ型 |

### Lisp/マクロとの比較

| 機能 | Lisp マクロ | 本実験 |
|------|-------------|--------|
| コード表現 | S-expression (リスト) | 型インスタンス |
| 拡張方法 | defmacro | SyntaxRule 型を定義 |
| 型安全性 | 弱（テキスト置換） | 強（型チェック） |
| 解析タイミング | 実行時/コンパイル時 | コンパイル時 |
| IDE サポート | 弱 | 強（型情報） |

---

## ブランチ計画

### 実験ブランチ

```
ブランチ名: exp/typed-homoiconicity
dev ブランチから作成
```

**重要**：
- これは**実験的ブランチ**であり、dev との頻繁なマージはない
- 長期的に独立開発になる可能性がある
- **main へのマージは保証されない**
- 実験が失敗した場合、ブランチは破棄される

### 開発フェーズ

> **⚠️ 実験時間上限：6ヶ月**

| フェーズ | 目標 | 予定時間 | 備考 |
|------|------|----------|------|
| Phase 1 | 概念検証：既存構文で AST 型を実装 | 2週間 | |
| Phase 2 | 基本的な評価器を実装 | 2週間 | 鍵となる課題：if/return 制御流 |
| Phase 3 | SyntaxRule 型の解析規則を実装 | 3週間 | |
| Phase 4 | ユーザー定義構文拡張 | 3週間 | 中心的目標：少なくとも1つのカスタムキーワードを完走 |
| Phase 5 | 最適化とドキュメント | 2週間 | 実験終了 |

**タイムアウト処理**：Phase 2（制御流実装）が4週間進捗しない場合は、諦めることを検討すべき。

---

## トレードオフ

### メリット

- **究極の統一性**：キーワードと通常のコードの境界を消除
- **言語の拡張性**：ユーザーが独自の構文を定義可能
- **型安全性**：従来のマクロより安全
- **学習価値**：言語の本質を深く理解できる

### デメリット

- **実装が複雑**：コンパイラの大規模な改修が必要
- **パフォーマンスの懸念**：実行時解釈が遅い可能性
- **学習曲線**：概念が抽象的で、型システムを深く理解する必要がある
- **実用性の疑問**：過剰エンジニアリングの可能性

### リスク

- 実験が失敗し、実用的なユースケースが見つからない
- 実装难易度が予想を超える
- 既存機能との衝突

---

## 未解決問題

- [ ] 構文衝突の処理方法（ユーザー定義規則と組み込み規則の衝突）は？
- [ ] パフォーマンス最適化方案は？
- [ ] 構文インポート/エクスポート机制が必要か？
- [ ] 既存のモジュールシステムとの統合方法は？

---

## 付録

### 用語集

| 用語 | 定義 |
|------|------|
| 同像性 (Homoiconicity) | コードとデータが同一の表現を使用すること |
| 構文木 (AST) | プログラムの抽象構文木表現 |
| SyntaxRule | 構文解析規則を携带する型 |
| Thunk | 遅延評価の関数ラッパー |
| CPS | Continuation Passing Style、続渡し形式 |

### 参考文献

- [Lisp Wiki: Homoiconicity](https://en.wikipedia.org/wiki/Homoiconicity)
- [Julia メタプログラミング](https://docs.julialang.org/en/v1/manual/metaprogramming/)
- [Rust 手続きマクロ](https://doc.rust-lang.org/book/ch19-06-macros.html)

---

## ライフサイクルと归宿

```
┌─────────────┐
│   草案       │  ← 現在のステータス
└──────┬──────┘
       │
       ▼
       ⚠️ 恒久的実験的ブランチ (exp/typed-homoiconicity)

       予想される結果：
       ├─► 成功検証 → アーカイブ、永遠にマージなし
       ├─► 失敗 → ブランチを破棄
       └─► タイムアウト → 放棄して破棄

       ⚠️ どのような結果になっても、本 RFC は決してマージされない
```

> **⚠️ 重要な注意**: これは探索的実験であり、**決してマージされない**。本機能に依存した本番コードを作成しないこと。
```