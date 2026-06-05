---
title: "RFC-019：型レベル同像性 (Typed Homoiconicity)"
---

# RFC-019：型レベル同像性 (Typed Homoiconicity) - 構文即ち型

> **状態**: 草案
>
> **作者**: 晨煦
>
> **作成日**: 2026-02-20
>
> **⚠️ 恒久的実験的宣言**: これは**探索的実験**であり、「構文即ち型」という言語設計理念の実現可能性を検証するためのものである。**本 RFC は決してマージされない**。結果がどうあれ、dev/main ブランチには取り込まれない。実験ブランチは完了後に廃棄またはアーカイブされる。
>
> - **実験目的**: 型レベル同像性の実装難易度と潜在的価値を検証する
> - **損切りライン**: 6ヶ月間進展がなければ断念
> - **成功基準**: 少なくとも1つのユーザー定義キーワードを完成させることができる（解析→コンパイル→実行の完全実装）
>
> **main ブランチへのマージは保証されない**。様々な理由により将来拒絶または放棄される可能性がある。本機能を本番環境で使用しないこと。
>
> **⚠️ 位置づけの説明**: 本 RFC は言語設計の思考実験であり、工程方案を提供するものではない。実用的な拡張可能なパーサー・パターンが必要であれば、Rust の `syn::Parse` や Haskell の `parsec` を参照されたい。

---

## 概要

本 RFC は、言語の構文構造自体を型システムの構成要素にするという、攻撃的な言語設計実験を提案する。

核心的な思想は Lisp の「コード即ちデータ」（同像性）に由来するが、**静的型システム**によって実現される：

- 構文木（AST）は型である
- キーワードは型の事前定義インスタンスである
- ユーザーは型を定義することで言語構文を拡張できる

つまり：言語本身が合成可能で拡張可能な「ビルディングブロック」になる。

---

## 動機

### なぜこの実験を行うのか？

1. **統一性への追求**：「キーワード」という特別な構文要素を排除し、全てを型と関数にする
2. **言語の拡張性**：関数を定義するのと同様に、新しい構文構造をユーザーが定義できる
3. **型安全なマクロ**：従来のマクロ（テキスト置換）は危険であり、型レベル同像性はコンパイル時チェックを提供できる
4. **学習目的**：言語設計の本質を深く理解する

### Lisp との関係

Lisp は既に「コード即ちデータ」を実現している：

```lisp
; Lisp のコード自体は S-expression である
(if (> x 0) "positive" "negative")
```

本実験の違いは：**静的型システムでこの理念を強化する**点にある。

---

## 提案

### 核心概念

#### 1. 構文木即ち型（AST as Type）

```yaoxiang
// 構文木のノードは全て型である
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

// コンパイラも関数でありうる
compile_if: (node: If, ctx: CompileContext) -> IR = ...
compile_while: (node: While, ctx: CompileContext) -> IR = ...
```

#### 3. 型は解析規則を携带する（核心的イノベーション）

これが本実験の鍵である：**型はデータを記述するだけでなく、コードをどのように解析するかの規則も携带する**。

```yaoxiang
// 構文規則の型
SyntaxRule: Type = {
    // この型のコードをどのように解析するか
    parse: (token_stream: TokenStream) -> (Self, remaining_tokens)

    // 型インスタンスをどのようにコンパイル/評価するか
    compile: (node: Self, ctx: CompileContext) -> IR
    eval: (node: Self, env: Env) -> Value
}

// IF 型の構文規則
IF: SyntaxRule = {
    // "if (cond) { then } else { else }" を解析する
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

ユーザーは独自の「キーワード」を定義できる：

```yaoxiang
// ユーザーが新しい構文構造を定義する：unless
Unless: SyntaxRule = {
    parse: (tokens: TokenStream) -> (If, remaining) = {
        consume("unless")
        cond = parse_expression(tokens)
        consume("{")
        body = parse_block(tokens)
        consume("}")
        // unless は if (!cond) { body } と同等である
        return If(Not(cond), body, Block([])), tokens
    }
}

// 使用例
unless x > 0 {
    print("x は正ではありません")
}

// 展開結果
if !(x > 0) {
    print("x は正ではありません")
}
```

### 例

#### 完全な例：カスタムループ構文

```yaoxiang
// "times" ループを定義する：n.times { ... } を n 回実行する
TimesLoop: SyntaxRule = {
    parse: (tokens: TokenStream) -> (While, remaining) = {
        receiver = parse_expression(tokens)  // 数値を取得
        consume(".times")
        consume("{")
        body = parse_block(tokens)
        consume("}")
        // while ループに変換する
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
    print("こんにちは！")
}

// 展開結果
i = 0
while i < 5 {
    print("こんにちは！")
    i = i + 1
}
```

#### 例：パターンマッチング構文

```yaoxiang
// ユーザーがパターンマッチングを定義する
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
    0 => "ゼロ",
    1 => "いち",
    n if n > 10 => "大きい",
    _ => "その他"
}
```

---

## 詳細設計

### システムアーキテクチャ

```
┌─────────────────────────────────────────────────────┐
│                    ソースコード                      │
└─────────────────┬───────────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────────┐
│              構文解析器 (Parser)                     │
│  - キーワードを認識する                               │
│  - 対応する SyntaxRule 型を見つける                   │
│  - 型の parse メソッドを呼び出す                      │
└─────────────────┬───────────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────────┐
│              AST (型のインスタンス)                   │
│  If, While, Match, TimesLoop...                      │
└─────────────────┬───────────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────────┐
│              コンパイラ/インタープリタ                │
│  - 型の compile/eval メソッドを呼び出す              │
│  - ターゲットコードを生成または実行する              │
└─────────────────────────────────────────────────────┘
```

### 重要な技術的課題

#### 1. 制御フローの関数化

問題：`if` は1つのブランチのみを評価する必要があるため、普通の関数呼び出しは使えない。

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

問題：`return` は多层の関数から脱出す必要がある。

解決策：
- 案 A：コンパイル時の CPS 変換
- 案 B：Result/Either モナドを使用
- 案 C：return のスコープを制限する

#### 3. 構文曖昧性

問題：`if(x > 0) { 1 }` を関数呼び出しかキーワードかどう见分けるか？

解決策：
- キーワードは特殊構文を使用（例：`if ... { } else { }`）
- または型システムの制約で区別

#### 4. 無限再帰

問題：ユーザーが自己参照する構文規則を定義する可能性がある。

解決策：コンパイル時に循環依存を検出する

---

## 既存システムとの関係

### RFC-010（統一型構文）との関係

RFC-010 は `name: type = value` の統一構文を実装しており、本 RFC はその延長である：

| RFC-010 | 本 RFC |
|----------|--------|
| 変数、関数、型は全て `name: type = value` | キーワードも `name: type = value` |
| 型は値である | 構文規則も値である |
| `Type` はメタ型である | `SyntaxRule` は構文のメタ型である |

### Lisp/マクロとの比較

| 機能 | Lisp マクロ | 本実験 |
|------|-------------|--------|
| コード表現 | S-expression（リスト） | 型インスタンス |
| 拡張方式 | defmacro | SyntaxRule 型を定義 |
| 型安全性 | 弱い（テキスト置換） | 強い（型チェック） |
| 解析タイミング | 実行時/コンパイル時 | コンパイル時 |
| IDE サポート | 弱い | 強い（型情報） |

---

## ブランチ計画

### 実験ブランチ

```
ブランチ名: exp/typed-homoiconicity
dev ブランチから作成
```

**重要**：

- これは**実験的ブランチ**であり、dev との頻繁なマージは行わない
- 長期的に独立開発となる可能性がある
- **main へのマージは保証されない**
- 実験が失敗した場合、ブランチは廃棄される

### 開発フェーズ

> **⚠️ 実験時間上限：6ヶ月**

| フェーズ | 目標 | 予想時間 | 備考 |
|----------|------|----------|------|
| Phase 1 | 概念検証：既存構文で AST 型を実装 | 2週間 | |
| Phase 2 | 基本的な評価器を実装 | 2週間 | 重要な課題：if/return 制御フロー |
| Phase 3 | SyntaxRule 型の解析規則を実装 | 3週間 | |
| Phase 4 | ユーザー定義構文拡張 | 3週間 | 核心目標：少なくとも1つのカスタムキーワードを完成させる |
| Phase 5 | 最適化とドキュメント | 2週間 | 実験終了 |

**タイムアウト処理**：Phase 2（制御フロー実装）が4週間進展 없을場合、放棄を検討すべきである。

---

## トレードオフ

### 利点

- **究極の統一性**：キーワードと обычных コードの境界を消除
- **言語の拡張性**：ユーザーは独自の構文を定義できる
- **型安全性**：従来のマクロより安全
- **学習価値**：言語の本質を深く理解できる

### 欠点

- **実装が複雑**：コンパイラ大幅改造が必要
- **パフォーマンス懸念**：実行時解釈は遅い可能性がある
- **学習曲線**：概念が抽象的で、型システムを深く理解する必要がある
- **実用性に疑問**：オーバエンジニアリングかもしれない

### リスク

- 実験が失敗し、実用的なシナリオが見つからない
- 実装難度が予想を超える
- 既存機能との衝突

---

## 開放問題

- [ ] 構文衝突の処理方法（ユーザー定義規則と組込み規則の衝突）は？
- [ ] パフォーマンス最適化方案は？
- [ ] 構文インポート/エクスポート機構が必要か？
- [ ] 既存モジュールシステムとの統合方法は？

---

## 付録

### 用語集

| 用語 | 定義 |
|------|------|
| 同像性 (Homoiconicity) | コードとデータが同一の表現を使用する |
| 構文木 (AST) | プログラムの抽象構文木表現 |
| SyntaxRule | 構文解析規則を携带する型 |
| Thunk | 遅延評価の関数ラッパー |
| CPS | Continuation Passing Style、続行渡しスタイル |

### 参考文献

- [Lisp Wiki: Homoiconicity](https://en.wikipedia.org/wiki/Homoiconicity)
- [Julia メタプログラミング](https://docs.julialang.org/en/v1/manual/metaprogramming/)
- [Rust 手続きマクロ](https://doc.rust-lang.org/book/ch19-06-macros.html)

---

## ライフサイクルと行く末

```
┌─────────────┐
│   草案      │  ← 現在の状態
└──────┬──────┘
       │
       ▼
       ⚠️ 恒久的実験的ブランチ (exp/typed-homoiconicity)

       予想される結果：
       ├─► 成功検証 → アーカイブ、マージ永不
       ├─► 失敗 → ブランチ廃棄
       └─► タイムアウト → 放棄して廃棄

       ⚠️ いかなる結果でも、本 RFC は決してマージされない
```

> **⚠️ 重要なお知らせ**: これは探索的実験であり、**決してマージされない**。本機能に依存する本番コードを作成しないこと。