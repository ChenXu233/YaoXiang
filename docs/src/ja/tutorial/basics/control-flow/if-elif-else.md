---
title: if-elif-else
---

# if-elif-else

`if-elif-else` はプログラミングにおいて最も基本的な意思決定ツールです。そのロジックは非常に直感的で——**条件が成立すれば、あるコードを実行する。さもなければ、次の条件をチェックする。すべて不成立なら、デフォルトの経路へ進む**。

## 基本構文

構文仕様では `if` 式と `if` 文の定義はまったく同じです：

```
if Expr Block ('elif' Expr Block)* ('else' Block)?
```

日常語で翻訳すると：`if` で始まり、続いて条件式とコードブロック、その後にはゼロ個以上の `elif 条件 コードブロック` を続け、最後にオプションで `else コードブロック` を置けます。

最もシンプルな形式——`if` だけ：

```yaoxiang
if temperature > 30 {
    print("天热了，开空调吧")
}
```

`else` を加えると：

```yaoxiang
if is_raining {
    print("带伞")
} else {
    print("不用带伞")
}
```

複数の条件は `elif` で：

```yaoxiang
score = 85

if score >= 90 {
    print("优秀")
} elif score >= 80 {
    print("良好")
} elif score >= 60 {
    print("及格")
} else {
    print("需要努力")
}
```

YaoXiang のキーワードは `elif` であり、`else if` ではない点に注意してください。これは言語がキーワードを意図的に簡潔に保つという設計思想の現れです。

## if は式である

これは YaoXiang の制御フローにおける最も重要な特徴の一つです：**`if` は式として使え、値を計算できる**。

```yaoxiang
// if 式：各分岐の値が result に代入される
result = if x > 0 {
    "正数"
} elif x < 0 {
    "负数"
} else {
    "零"
}
// result は現在 "正数"、"负数" または "零" のいずれか
```

`if` を式として使う場合、すべての分岐の戻り値の型は一致している必要があります：

```yaoxiang
score = 88

// すべての分岐が String を返すため、型が一貫しており、問題なし
grade = if score >= 90 {
    "A"
} elif score >= 80 {
    "B"
} elif score >= 60 {
    "C"
} else {
    "D"
}
print(grade)  // "B"
```

各分岐のコードブロックにおいて、**最後の式の値がその分岐の戻り値となります**。`return` を使って明示的に返すこともできますが、分岐内では通常は式を直接書くだけで十分です。

```yaoxiang
// 式を直接書く——推奨
category = if age < 18 { "未成年" } else { "成年" }

// 明示的に return することもできる——効果は同じ
category = if age < 18 {
    return "未成年"
} else {
    return "成年"
}
```

値の取得が目当てではなく、条件判定に `if` を使うだけなら、それは通常の文となり、式の形式と完全に互換性があります。

## if のネスト

`if` の中にさらに `if` を書いて、多層の条件判定を処理できます：

```yaoxiang
age = 25
has_ticket = true

if age >= 18 {
    if has_ticket {
        print("欢迎入场")
    } else {
        print("请先购票")
    }
} else {
    print("未成年人需家长陪同")
}
```

式がネストされる場合、YaoXiang には C 言語のような「ぶら下がり `else`」の曖昧さが存在しません——すべての `else` は常に対応する相手がいない最も近い `if` に紐づきます。

## ブール演算子による条件の組み合わせ

条件の中では `and`、`or`、`not` を使って複数の判定を組み合わせられます：

```yaoxiang
username = "admin"
password = "123456"

// and：両方の条件が成立
if username == "admin" and password == "123456" {
    print("登录成功")
}

// or：いずれかの条件が成立
if role == "admin" or role == "moderator" {
    print("有管理权限")
}

// not：否定
if not is_banned {
    print("允许发言")
}

// 組み合わせて使用
if (age >= 18 and age <= 60) or is_vip {
    print("可以参加活动")
}
```

演算子の優先順位は `not` が `and` より高く、`and` が `or` より高いです。不安な場合は括弧を付けて、意図をより明確にしましょう。

## 小結

| 要点 | 説明 |
|------|------|
| 基本構造 | `if 条件 { ... } elif 条件 { ... } else { ... }` |
| elif | YaoXiang は `elif` を使う。`else if` ではない |
| 式 | `if` は値を返せる。すべての分岐の型は一致している必要がある |
| 分岐の戻り値 | 分岐ブロック内の最後の式の値が戻り値となる |
| ネスト | `if` の中にさらに `if` を書ける。ぶら下がり `else` の曖昧さはない |
| ブール演算 | `and`、`or`、`not` で条件を組み合わせる |

次の章では `for` ループ——集合や範囲を反復処理する標準的な方法について学びます。