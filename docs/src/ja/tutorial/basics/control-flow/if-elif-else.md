---
title: if-elif-else
---

# if-elif-else

`if-elif-else` はプログラミングにおいて最も基本的な意思決定ツールです。そのロジックは非常に直感的で、**条件が成立すればあるコードを実行し、そうでなければ次の条件を確認し、どれも成立しなければデフォルトのパスに進む**というものです。

## 基本構文

言語仕様において `if` 式と `if` 文の定義はまったく同じです。

```
if Expr Block ('elif' Expr Block)* ('else' Block)?
```

日常語に訳すと：`if` で始まり、その後ろに条件式とコードブロックが続き、続いて 0 個以上の `elif 条件 コードブロック` を、最後にもしあれば `else コードブロック` 1 つを続けます。

最もシンプルな形式—`if` だけ:

```yaoxiang
if temperature > 30 {
    println("天热了，开空调吧")
}
```

`else` を加えると:

```yaoxiang
if is_raining {
    println("带伞")
} else {
    println("不用带伞")
}
```

複数の条件には `elif` を使います:

```yaoxiang
score = 85

if score >= 90 {
    println("优秀")
} elif score >= 80 {
    println("良好")
} elif score >= 60 {
    println("及格")
} else {
    println("需要努力")
}
```

YaoXiang のキーワードは `elif` であって、`else if` ではない点に注意してください。これは言語が意図的にキーワードを簡潔に保つという設計思想の表れです。

## if は式である

これは YaoXiang の制御フローにおける最も重要な特徴のひとつです。**`if` は式として使うことができ、値を算出できます。**

```yaoxiang
# if 式: 各分岐の値が result に代入される
result = if x > 0 {
    "正数"
} elif x < 0 {
    "负数"
} else {
    "零"
}
# result は現在 "正数"、"负数"、"零" のいずれか
```

`if` を式として使う場合、すべての分岐の戻り値の型は一致していなければなりません。

```yaoxiang
score = 88

# すべての分岐が String を返すので、型は一致し、問題ない
grade = if score >= 90 {
    "A"
} elif score >= 80 {
    "B"
} elif score >= 60 {
    "C"
} else {
    "D"
}
println(grade)  # "B"
```

各分岐のコードブロックにおいて、**最後の式の値がその分岐の戻り値となります**。`return` を使って明示的に返すこともできますが、分岐内では通常、式を直接書くだけで十分です。

```yaoxiang
# 式を直接書く — 推奨
category = if age < 18 { "未成年" } else { "成年" }

# 明示的に return することもできる — 効果は同じ
category = if age < 18 {
    return "未成年"
} else {
    return "成年"
}
```

単に `if` を条件判断のために使うだけで値が不要なら、それは通常の文であり、式の形式と完全に互換性があります。

## ネストした if

`if` の内側にもうひとつ `if` を書くことで、多層の条件判断を処理できます。

```yaoxiang
age = 25
has_ticket = true

if age >= 18 {
    if has_ticket {
        println("欢迎入场")
    } else {
        println("请先购票")
    }
} else {
    println("未成年人需家长陪同")
}
```

式がネストしている場合でも、YaoXiang には C 言語のような「ぶら下がり else」の曖昧さはありません。すべての `else` は常に対応する相手がいない最も近い `if` に紐づきます。

## ブール演算子を使った条件の組み合わせ

条件の中では `and`、`or`、`not` を使って複数の判断を組み合わせられます。

```yaoxiang
username = "admin"
password = "123456"

# and: 両方の条件が成立
if username == "admin" and password == "123456" {
    println("登录成功")
}

# or: いずれかの条件が成立
if role == "admin" or role == "moderator" {
    println("有管理权限")
}

# not: 否定
if not is_banned {
    println("允许发言")
}

# 組み合わせて使う
if (age >= 18 and age <= 60) or is_vip {
    println("可以参加活动")
}
```

演算子の優先順位は、`not` が `and` より高く、`and` が `or` より高いです。心配な場合は括弧を付けて、意図をより明確にしましょう。

## まとめ

| ポイント | 説明 |
|------|------|
| 基本構造 | `if 条件 { ... } elif 条件 { ... } else { ... }` |
| elif | YaoXiang では `elif` を使う(`else if` ではない) |
| 式 | `if` は値を返せる。すべての分岐の型は一致しなければならない |
| 分岐の戻り値 | 分岐ブロックの最後の式の値が戻り値となる |
| ネスト | `if` の中にさらに `if` を書ける。ぶら下がり else の曖昧さはない |
| ブール演算 | `and`、`or`、`not` で条件を組み合わせる |

次の章では `for` ループ — コレクションや範囲を反復処理する標準的な方法 — について学びます。