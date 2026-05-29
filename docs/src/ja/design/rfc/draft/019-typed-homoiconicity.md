```markdown
---
title: RFC-019：型レベル同像性 (Typed Homoiconicity)
---

# RFC-019: 型レベル同像性 (Typed Homoiconicity) - 構文即ち型

> **状態**: 草案
>
> **著者**: 晨煦
>
> **作成日**: 2026-02-20
>
> **⚠️ 永久実験的宣言**: これは「構文即ち型」という言語設計理念の実行可能性を探る**探索的実験**です。**本 RFC は決してマージされず**、結果にかかわらず dev/main ブランチには入りません。実験ブランチは完了後に破棄またはアーカイブされます。
>
> - **実験目的**: 型レベル同像性の実装難易度と潜在価値を検証する
> - **損切りライン**: 6ヶ月進展がなければ断念
> - **成功基準**: 少なくとも1つのユーザー定義キーワードを完走できる（完全解析→コンパイル→実行）
>
> **main ブランチへのマージは保証されません**。様々な理由から拒否または放棄される可能性があります。本機能を本番環境で使用しないでください。

---

## 概要

本 RFC はorra激进的な言語設計実験を提案します：**言語の構文構造そのものを型システムの一部にする**。

コアとなるアイデアは Lisp の「コード即ちデータ」（同像性）に由来しますが、**静的型システム**によって実装されます：

- 構文木（AST）は型である
- キーワードは型の事前定義インスタンスである
- ユーザーは型を定義することで言語の構文を拡張できる

つまり、言語そのものが合成可能で拡張可能な「ビルディングブロック」になります。

---

## 動機

### なぜこの実験を行うのか？

1. **統一性への追求**：「キーワード」という特殊な構文要素を排除し、すべてを型と関数にする
2. **言語の拡張性**：関数を定義するのと同じように新しい構文構造をユーザーが定義できる
3. **型安全なマクロ**：従来のマクロ（テキスト置換）は危険であり、型レベル同像性はコンパイル時のチェックを提供できる
4. **学習目的**：言語設計の本質を深く理解する

### Lisp との関係

Lisp はすでに「コード即ちデータ」を実装しています：

```lisp
; Lisp のコード自体が S-expression
(if (> x 0) "positive" "negative")
```

本実験の違いは：**この理念を静的型システムで強化する**ことです。

---

## 提案

### コアコンセプト

#### 1. 構文木即ち型（AST as Type）

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

#### 3. 型が解析ルールを携带（コアイノベーション）

これが本実験の鍵です：**型はデータを記述するだけでなく、コードの解析方法」も携带します**。

```yaoxiang
// 構文ルール型
SyntaxRule: Type = {
    // この型のコードをどのように解析するか
    parse: (token_stream: TokenStream) -> (Self, remaining_tokens)

    // 型インスタンスをどのようにコンパイル/評価するか
    compile: (node: Self, ctx: CompileContext) -> IR
    eval: (node: Self, env: Env) -> Value
}

// IF 型の構文ルール
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

#### 4. ユーザー定義の構文拡張

ユーザーは自分の「キーワード」を定義できます：

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

// 展開結果
if !(x > 0) {
    print("x is not positive")
}
```

### 示例

#### 完全示例：カスタムループ構文

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

// 展開結果
i = 0
while i < 5 {
    print("Hello!")
    i = i + 1
}
```

#### 示例：パターン照合構文

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
│  - キーワードを認識                                   │
│  - 対応する SyntaxRule 型を見つける                   │
│  - 型の parse メソッドを呼び出す                       │
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
│              コンパイラ/インタプリタ                   │
│  - 型の compile/eval メソッドを呼び出す                │
│  - ターゲットコードを生成または実行                     │
└─────────────────────────────────────────────────────┘
```

### 关键技术問題

#### 1. 制御フロー関數化

問題：`if` は片方の分支のみを評価する必要があり、通常の関数呼び出しでは做不到。

解決策：thunk（遅延評価）を渡す

```yaoxiang
// コンパイル後の内部表現
If: Type = {
    condition: Expr,
    then: () -> Value,  // thunk、遅延評価
    else: () -> Value
}
```

#### 2. return の非局所返戻

問題：`return` は多层の関数から抜ける必要がある。

解決策：

- 案 A：コンパイル時の CPS 変換
- 案 B：Result/Either モナドを使用
- 案 C：return のスコープを制限

#### 3. 構文歧義

問題：`if(x > 0) { 1 }` を関数呼び出しかキーワードかき、見分けられない。

解決策：

- キーワードは特殊構文を使用（例：`if ... { } else { }`）
- または型システムで制約

#### 4. 無限再帰

問題：ユーザーが自己参照の構文ルールを定義する可能性がある。

解決策：コンパイル時に循環依存を検出

---

## 既存システムとの関係

### RFC-010（統一型構文）との関係

RFC-010 は `name: type = value` の統一構文を実装しており、本 RFC はその延長です：

| RFC-010 | 本 RFC |
|----------|--------|
| 変数、関数、型はすべて `name: type = value` | キーワードも `name: type = value` |
| 型は値 | 構文ルールも値 |
| `Type` はメタ型 | `SyntaxRule` は構文のメタ型 |

### Lisp/マクロとの比較

| 機能 | Lisp マクロ | 本実験 |
|------|---------|--------|
| コード表現 | S-expression（リスト） | 型インスタンス |
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
- 長期的に独立開発となる可能性がある
- **main へのマージは保証されない**
- 実験が失敗した場合、ブランチは破棄される

### 開発フェーズ

> **⚠️ 実験時間上限：6ヶ月**

| フェーズ | 目標 | 予定期間 | 備考 |
|------|------|----------|------|
| Phase 1 | 概念検証：既存構文で AST 型を実装 | 2週間 | |
| Phase 2 | 基本的な評価器を実装 | 2週間 | 关键課題：if/return 制御フロー |
| Phase 3 | SyntaxRule 型の解析ルールを実装 | 3週間 | |
| Phase 4 | ユーザー定義構文拡張 | 3週間 | コア目標：少なくとも1つのカスタムキーワードを完走 |
| Phase 5 | 最適化とドキュメンテーション | 2週間 | 実験終了 |

**タイムアウト処理**：Phase 2（制御フロー実装）が4週間進捗がなければ、断念を検討すべき。

---

## トレードオフ

### 优点

- **究極の統一性**：キーワードと通常コードの境界を消除
- **言語の拡張性**：ユーザーが独自の構文を定義できる
- **型安全性**：従来のマクロより安全
- **学習価値**：言語の本質を深く理解できる

### 缺点

- **実装が複雑**：コンパイラの大規模な変更が必要
- **パフォーマンス懸念**：実行時解釈が遅い可能性
- **学習曲線**：概念が抽象的で、型システムの理解が必要
- **実用性が疑わしい**：過剰エンジニアリングの可能性

### リスク

- 実験が失敗し、実用的なシナリオが見つからない
- 実装難易度が予想を超える
- 既存機能との衝突

---

## 開放問題

- [ ] 構文衝突の処理方法（ユーザー定義ルールと組み込みの衝突）は？
- [ ] パフォーマンス最適化方案は？
- [ ] 構文のインポート/エクスポート機構が必要か？
- [ ] 既存のモジュールシステムとの統合方法は？

---

## 付録

### 用語集

| 用語 | 定義 |
|------|------|
| 同像性 (Homoiconicity) | コードとデータが同一の表現を使用する |
| 構文木 (AST) | プログラムの抽象構文木表現 |
| SyntaxRule | 構文解析ルールを携带する型 |
| Thunk | 遅延評価の関数ラッパー |
| CPS | Continuation Passing Style、連続渡しスタイル |

### 参考文献

- [Lisp Wiki：Homoiconicity](https://en.wikipedia.org/wiki/Homoiconicity)
- [Julia メタプログラミング](https://docs.julialang.org/en/v1/manual/metaprogramming/)
- [Rust 手続きマクロ](https://doc.rust-lang.org/book/ch19-06-macros.html)

---

## ライフサイクルと归宿

```
┌─────────────┐
│   草案      │  ← 現在の状態
└──────┬──────┘
       │
       ▼
       ⚠️ 永久実験的ブランチ (exp/typed-homoiconicity)

       可能性のある結果：
       ├─► 成功検証 → アーカイブ、決してマージしない
       ├─► 失敗 → ブランチを破棄
       └─► タイムアウト → 放棄して破棄

       ⚠️ どのような結果でも、本 RFC は決してマージしない
```

> **⚠️ 重要なお知らせ**: これは探索的実験であり、**決してマージされません**。本機能を本番コードに依存しないでください。
```