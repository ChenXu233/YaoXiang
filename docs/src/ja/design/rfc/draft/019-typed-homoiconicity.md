```markdown
---
title: RFC-019：類型レベル同像性（Typed Homoiconicity）
---

# RFC-019: 類型レベル同像性 (Typed Homoiconicity) - 構文即ち型

> **ステータス**: 草案
>
> **著者**: 晨煦
>
> **作成日**: 2026-02-20
>
> **⚠️ 恒久実験的宣言**: これは「構文即ち型」という言語設計理念の実行可能性を検証するための**探索的実験**です。**本 RFC は決してマージされず**、結果にかかわらず dev/main ブランチには入りません。実験ブランチは完了後に廃棄またはアーカイブされます。
>
> - **実験目標**: 類型レベル同像性の実装難易度と潜在価値を検証する
> - **止损線**: 6ヶ月進捗なしの場合は諦める
> - **成功基準**: 少なくとも1つのユーザー定義キーワードが通る（完全な解析→コンパイル→実行）
>
> **main ブランチへのマージは保証されません**。将来、様々な理由により拒否または放棄される可能性があります。本機能を本番環境で使用しないでください。

---

## 摘要

本 RFC は、言語の構文構造そのものを型システムの一部にするという、攻撃的な言語設計実験を提案します。

核心理想は Lisp の「コード即ちデータ」（同像性）に由来しますが、**静的型システム**を通じて実装されます：

- 抽象構文木（AST）は型である
- キーワードは型の事前定義インスタンスである
- ユーザーは型を定義することで言語構文を拡張できる

つまり、言語本身が合成可能で拡張可能な「ビルディングブロック」になります。

---

## 動機

### なぜこの実験を行うのか？

1. **統一性への追求**：「キーワード」という特殊な構文要素を排除し、すべてを型と関数にする
2. **言語の拡張性**: ユーザーが関数を定義するのと同じように新しい構文構造を定義できる
3. **型安全なマクロ**: 伝統のマクロ（テキスト置換）は危険であり、類型レベル同像性はコンパイル時チェックを提供できる
4. **学習目的**: 言語設計の本質を深く理解する

### Lisp との関係

Lisp はすでに「コード即ちデータ」を実装しています：

```lisp
; Lisp コードは本身就是 S式
(if (> x 0) "positive" "negative")
```

本実験の差異は：**この理念を静的型システムで強化する**ことです。

---

## 提案

### コアコンセプト

#### 1. 抽象構文木即ち型（AST as Type）

```yaoxiang
// 抽象構文木のノードはすべて型である
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

// コンパイラも関数でありえる
compile_if: (node: If, ctx: CompileContext) -> IR = ...
compile_while: (node: While, ctx: CompileContext) -> IR = ...
```

#### 3. 型が解析規則を伴う（コアイノベーション）

これが本実験の鍵です：**型はデータを記述するだけでなく、コードをどのように解析するかの規則성도携带します**。

```yaoxiang
// 構文規則型
SyntaxRule: Type = {
    // この型のコードをどのように解析するか
    parse: (token_stream: TokenStream) -> (Self, remaining_tokens)

    // この型のインスタンスをどのようにコンパイル/評価するか
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

#### 4. ユーザー定義構文拡張

ユーザーは独自の「キーワード」を定義できます：

```yaoxiang
// ユーザーが新しい構文構造を定義する：unless
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

// 使用
unless x > 0 {
    print("x is not positive")
}

// 展開結果
if !(x > 0) {
    print("x is not positive")
}
```

### 例

#### 完全な例：カスタムループ構文

```yaoxiang
// "times" ループを定義する：n.times { ... } を n 回実行
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

// 使用
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

// 使用
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
│                    ソースコード                        │
└─────────────────┬───────────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────────┐
│              構文解析器 (Parser)                       │
│  - キーワードを識別                                   │
│  - 対応する SyntaxRule 型を見つける                   │
│  - 型の parse メソッドを呼び出す                       │
└─────────────────┬───────────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────────┐
│              AST (型インスタンス)                      │
│  If, While, Match, TimesLoop...                      │
└─────────────────┬───────────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────────┐
│              コンパイラ/インタプリタ                     │
│  - 型の compile/eval メソッドを呼び出す                │
│  - ターゲットコードを生成または実行                     │
└─────────────────────────────────────────────────────┘
```

### 关键技术問題

#### 1. 制御流の関数化

問題：`if` は片方の分支のみを評価する必要があり、普通の関数呼び出しでは不行。

解決策：thunk（遅延評価）を渡す

```yaoxiang
// コンパイル後の内部表現
If: Type = {
    condition: Expr,
    then: () -> Value,  // thunk、遅延評価
    else: () -> Value
}
```

#### 2. return の非局所的戻り

問題：`return` は多层の関数から脱出す必要がある。

解決策：

- 案 A: コンパイル時の CPS 変換
- 案 B: Result/Either モナドを使用
- 案 C: return のスコープを制限

#### 3. 構文岐義

問題：`if(x > 0) { 1 }` を関数呼び出しかキーワードかき、どのように区別するか？

解決策：

- キーワードは特殊構文を使用（例：`if ... { } else { }`）
- または型システムで制約

#### 4. 無限再帰

問題：ユーザーは自己参照する構文規則を定義かもしれない。

解決策：コンパイル時に循環依存を検出

---

## 既存システムとの関係

### RFC-010（統一型構文）との関係

RFC-010 は `name: type = value` の統一構文を実装しており、本 RFC はその延長です：

| RFC-010 | 本 RFC |
|---------|--------|
| 変数、関数、型はすべて `name: type = value` | キーワードも `name: type = value` |
| 型は値である | 構文規則も値である |
| `Type` はメタ型である | `SyntaxRule` は構文のメタ型である |

### Lisp/マクロとの比較

| 機能 | Lisp マクロ | 本実験 |
|------|------------|--------|
| コード表現 | S式（リスト） | 型インスタンス |
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

- これは**実験ブランチ**であり、dev との頻繁なマージはない
- 長期的に独立開發の可能性あり
- **main へのマージは保証されない**
- 実験に失敗した場合、ブランチは廃棄される

### 開発フェーズ

> **⚠️ 実験時間上限: 6ヶ月**

| フェーズ | 目標 | 予想法時間 | 備考 |
|---------|------|-----------|------|
| Phase 1 | 概念検証：既存構文で AST 型を実装 | 2週間 | |
| Phase 2 | 基本的な評価器を実装 | 2週間 | 重要な課題：if/return 制御流 |
| Phase 3 | SyntaxRule 型の解析規則を実装 | 3週間 | |
| Phase 4 | ユーザー定義構文拡張 | 3週間 | コア目標：少なくとも1つのカスタムキーワードを通す |
| Phase 5 | 最適化とドキュメンテーション | 2週間 | 実験終了 |

**タイムアウト処理**: Phase 2（制御流実装）が4週間進捗なしの場合は、諦めるを検討すべき。

---

## トレードオフ

### メリット

- **究極の統一性**: キーワードと обычныхコードの境界を消除
- **言語の拡張性**: ユーザーが独自の構文を定義できる
- **型安全性**: 伝統のマクロより安全
- **学習価値**: 言語の本質を深く理解できる

### デメリット

- **実装の複雑さ**: コンパイラ大幅改造が必要
- **パフォーマンス懸念**: 実行時解釈が遅い可能性
- **学習曲線**: 概念が抽象的で、型システムを深く理解する必要がある
- **実用性の疑問**: 過度エンジニアリングかもしれない

### リスク

- 実験が失敗し、実用シナリオが見つからない
- 実装難しさが予想を超える
- 既存機能との衝突

---

## オープン問題

- [ ] 構文衝突の処理方法（ユーザー定義規則と組み込みの衝突）は？
- [ ] パフォーマンス最適化方案は？
- [ ] 構文インポート/エクスポート機構が必要か？
- [ ] 既存モジュールシステムとの統合方法は？

---

## 付録

### 用語集

| 用語 | 定義 |
|------|------|
| 同像性 (Homoiconicity) | コードとデータが同じ表現を使用すること |
| 抽象構文木 (AST) | プログラムの抽象構文木表現 |
| SyntaxRule | 構文解析規則を携带する型 |
| Thunk | 遅延評価の関数包装 |
| CPS | Continuation Passing Style、継続渡スタイル |

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
       ⚠️ 恒久実験ブランチ (exp/typed-homoiconicity)

       あり得る結果：
       ├─► 成功検証 → アーカイブ、永遠にマージなし
       ├─► 失敗 → ブランチ廃棄
       └─► タイムアウト → 諦めて廃棄

       ⚠️ どのような結果でも、本 RFC は決してマージされない
```

> **⚠️ 重要な注意**: これは探索的実験であり、**決してマージされません**。本番コードで本機能に依存しないでください。
```