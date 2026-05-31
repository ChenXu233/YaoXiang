---
title: "RFC-019：类型级同像性 (Typed Homoiconicity)"
---

# RFC-019: 类型级同像性 (Typed Homoiconicity) - 文法即型

> **状態**: 草案
>
> **著者**: 晨煦
>
> **作成日**: 2026-02-20
>
> **⚠️ 永続的な実験的宣言**: これは「文法即型」という言語設計理念の実行可能性を検証するための**探索的実験**です。**本 RFC は永不合并**であり、結果にかかわらず dev/main ブランチにはマージされません。実験ブランチは完了後に破棄またはアーカイブされます。
>
> - **実験目的**: 型レベル同像性の実装難易度と潜在価値を検証する
> - **損切りライン**: 6ヶ月間進捗がない場合は放棄
> - **成功基準**: 少なくとも1つのユーザー定義キーワードを動かせること（完全解析→コンパイル→実行）
>
> **main ブランチへのマージは保証されません**。将来、さまざまな理由により拒否または放棄される可能性があります。商用環境でこの機能を使用しないでください。

---

## 摘要

本 RFC はorra激进的な言語設計実験を提案します：**言語の文法構造そのものを型システムの一部にする**というものです。

コアとなるアイデアは Lisp の「コード即データ」（同像性）に由来しますが、**静的型システム**によって実装されます：

- 構文木（AST）は型である
- キーワードは型の事前定義インスタンスである
- ユーザーは型を定義することで言語文法を拡張できる

つまり、言語そのものが組み合わせ可能で拡張可能な「ビルディングブロック」になります。

---

## 動機

### なぜこの実験を行うのか？

1. **統一性の追求**: 「キーワード」という特殊な文法要素を排除し、すべてを型と関数にする
2. **言語の拡張性**: ユーザーが関数を定義するのと同様に新しい文法構造を定義できる
3. **型安全なマクロ**: 従来のマクロ（テキスト置換）は危険だが、型レベル同像性はコンパイル時チェックを提供できる
4. **学習目的**: 言語設計の本質を深く理解する

### Lisp との関係

Lisp はすでに「コード即データ」を実装しています：

```lisp
; Lisp のコード自体が S-expression
(if (> x 0) "positive" "negative")
```

本実験の違いは：**この理念を静的型システムで強化する**ことです。

---

## 提案

### コアコンセプト

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

// コンパイラも関数でありうる
compile_if: (node: If, ctx: CompileContext) -> IR = ...
compile_while: (node: While, ctx: CompileContext) -> IR = ...
```

#### 3. 型が解析規則を携带する（コアイノベーション）

これが本実験の鍵です：**型はデータを記述するだけでなく、コードをどのように解析するかの規則も携带します**。

```yaoxiang
// 文法規則の型
SyntaxRule: Type = {
    // この型のコードをどのように解析するか
    parse: (token_stream: TokenStream) -> (Self, remaining_tokens)

    // この型のインスタンスをコンパイル/評価する方法
    compile: (node: Self, ctx: CompileContext) -> IR
    eval: (node: Self, env: Env) -> Value
}

// IF 型の文法規則
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

#### 4. ユーザー定義の文法拡張

ユーザーは独自の「キーワード」を定義できます：

```yaoxiang
// ユーザーが新しい文法構造を定義する: unless
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

### 例

#### 完整示例：自定义循环语法

```yaoxiang
// "times" ループを定義する: n.times { ... } を n 回実行する
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
    print("Hello!")
}

// 展開結果
i = 0
while i < 5 {
    print("Hello!")
    i = i + 1
}
```

#### 示例：模式匹配语法

```yaoxiang
// ユーザーがパターン照合を定義する
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
│                    ソースコード                      │
└─────────────────┬───────────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────────┐
│              文法解析器 (Parser)                      │
│  - キーワードの認識                                  │
│  - 対応する SyntaxRule 型を見つける                  │
│  - 型の parse メソッドを呼び出す                     │
└─────────────────┬───────────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────────┐
│              AST (型のインスタンス)                  │
│  If, While, Match, TimesLoop...                    │
└─────────────────┬───────────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────────┐
│              コンパイラ/インタプリタ                 │
│  - 型の compile/eval メソッドを呼び出す             │
│  - ターゲットコードを生成または実行                  │
└─────────────────────────────────────────────────────┘
```

### 主要技術的課題

#### 1. 制御フローの関数化

課題：`if` は片方の分支のみを評価する必要があり、通常の関数呼び出しでは不可。

解決策：thunk（遅延評価）を渡す

```yaoxiang
// コンパイル後の内部表現
If: Type = {
    condition: Expr,
    then: () -> Value,  // thunk、遅延評価
    else: () -> Value
}
```

#### 2. return の非局所リターン

課題：`return` は多层の関数を脱出する必要がある。

解決策：

- 案 A: コンパイル時の CPS 変換
- 案 B: Result/Either モナドを使用
- 案 C: return のスコープを制限

#### 3. 文法の曖昧性

課題：`if(x > 0) { 1 }` を関数呼び出しかキーワードかをどのように区別するか？

解決策：

- キーワードは特殊文法を使用（例如 `if ... { } else { }`）
- または型システムで制約

#### 4. 無限再帰

課題：ユーザーが自己参照する文法規則を定義する可能性がある。

解決策：コンパイル時に循環依存を検出

---

## 既存システムとの関係

### RFC-010（统一类型语法）との関係

RFC-010 は `name: type = value` の統一文法を実装しています。本 RFC はその延長線上にあります：

| RFC-010 | 本 RFC |
|----------|--------|
| 変数、関数、型はすべて `name: type = value` | キーワードも `name: type = value` |
| 型は値 | 文法規則も値 |
| `Type` はメタ型 | `SyntaxRule` は文法のメタ型 |

### Lisp/マクロとの比較

| 機能 | Lisp マクロ | 本実験 |
|------|---------|--------|
| コードの表現 | S-expression (リスト) | 型のインスタンス |
| 拡張方法 | defmacro | SyntaxRule 型を定義 |
| 型安全性 | 弱い（テキスト置換） | 強い（型チェック） |
| 解析タイミング | ランタイム/コンパイル時 | コンパイル時 |
| IDE サポート | 弱い | 強い（型情報） |

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
- 実験に失敗した場合、ブランチは破棄される

### 開発フェーズ

> **⚠️ 実験時間上限: 6ヶ月**

| フェーズ | 目標 | 予想法時間 | 備考 |
|------|------|----------|------|
| Phase 1 | 概念実証: 既存文法での AST 型の実装 | 2週間 | |
| Phase 2 | 基本的な評価器の実装 | 2週間 | 重要な課題: if/return 制御フロー |
| Phase 3 | SyntaxRule 型の解析規則の実装 | 3週間 | |
| Phase 4 | ユーザー定義の文法拡張 | 3週間 | コア目標: 少なくとも1つのカスタムキーワードを動作させる |
| Phase 5 | 最適化とドキュメント | 2週間 | 実験終了 |

**タイムアウト処理**: Phase 2（制御フロー実装）が4週間進捗がない場合は、放棄を検討すべきです。

---

## 权衡

### 長所

- **究極の統一性**: キーワードと通常コードの境界を排除
- **言語の拡張性**: ユーザーが独自の文法を定義可能
- **型安全性**: 従来のマクロより安全
- **学習価値**: 言語の本質を深く理解

### 短所

- **実装が複雑**: コンパイラの大規模な修正が必要
- **パフォーマンスの懸念**: ランタイム解釈が遅い可能性
- **学習曲線**: 概念的抽象化が必要、型システムの理解が必要
- **実用的疑問**: 過剰エンジニアリングの可能性

### リスク

- 実験が失敗し、実用的なシナリオが見つからない
- 実装難しさが予想を超える
- 既存機能との衝突

---

## 開放問題

- [ ] 構文衝突（ユーザー定義規則と組み込みの衝突）をどのように処理するか？
- [ ] パフォーマンス最適化の方法は？
- [ ] 構文のインポート/エクスポート機構が必要か？
- [ ] 既存のモジュールシステムとどのように統合するか？

---

## 付録

### 用語集

| 用語 | 定義 |
|------|------|
| 同像性 (Homoiconicity) | コードとデータが同一の表現を使用すること |
| 構文木 (AST) | プログラムの抽象構文木表現 |
| SyntaxRule | 構文解析規則を携带する型 |
| Thunk | 遅延評価の関数ラッパー |
| CPS | Continuation Passing Style、Continuation Passing Style |

### 参考文献

- [Lisp Wiki: Homoiconicity](https://en.wikipedia.org/wiki/Homoiconicity)
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
       ⚠️ 永続的な実験的ブランチ (exp/typed-homoiconicity)

       可能性のある結果：
       ├─► 成功裏の検証 → アーカイブ、永不合并
       ├─► 失敗 → ブランチを破棄
       └─► タイムアウト → 放棄して破棄

       ⚠️ どのような結果であれ、本 RFC は永不合并
```

> **⚠️ 重要な注意**: これは探索的実験であり、**永不合并**です。商用コードでこの機能に依存しないでください。