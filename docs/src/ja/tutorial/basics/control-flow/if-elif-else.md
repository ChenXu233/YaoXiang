---
title: 'if-else-if-else'
---

# if-else-if-else

`if-else-if-else`
はプログラミングにおける最も基本的な意思決定ツールです。そのロジックは非常に直感的です——**条件が成立すれば、あるコードを実行する。そうでなければ、次の条件を確認する。どちらも成立しなければ、デフォルトのパスへ進む**。

## 基本構文

文法仕様における `if` 式と `if` 文の定義はまったく同じです：

```
if Expr Block ('else' 'if' Expr Block)* ('else' Block)?
```

日常の言葉で言い換えると：`if`
で始まり、その後ろに条件式とコードブロックが続きます。続いて、ゼロ個以上の
`else if 条件 コードブロック` をつなぎ、最後にオプションの `else コードブロック`
をひとつつけることができます。

最もシンプルな形式——`if` のみ：

```yaoxiang
if temperature > 30 {
    print("天热了，开空调吧")
}
```

`else` を加える：

```yaoxiang
if is_raining {
    print("带伞")
} else {
    print("不用带伞")
}
```

複数の条件は `else if` でつなぐ：

```yaoxiang
score = 85

if score >= 90 {
    print("优秀")
} else if score >= 80 {
    print("良好")
} else if score >= 60 {
    print("及格")
} else {
    print("需要努力")
}
```

## if は式

これは YaoXiang の制御フローにおいて最も重要な特徴のひとつです：**`if`
は式として使われ、値を算出することができる**。

```yaoxiang
// if 式：各分岐の値が result に代入される
result = if x > 0 {
    "正数"
} else if x < 0 {
    "负数"
} else {
    "零"
}
// result は現在 "正数"、"负数"、"零" のいずれか
```

`if` を式として使う場合、すべての分岐の戻り値の型は一致している必要があります：

```yaoxiang
score = 88

// すべての分岐が String を返し、型は一致しているため問題ない
grade = if score >= 90 {
    "A"
} else if score >= 80 {
    "B"
} else if score >= 60 {
    "C"
} else {
    "D"
}
print(grade)  // "B"
```

各分岐のコードブロックでは、**最後の式の値がその分岐の戻り値となります**。`return`
で明示的に返すこともできますが、分岐内ではふつう式を直接書くだけで十分です。

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

`if`
を条件判定のためだけに使って値が不要なら、それは通常の文となり、式の形式と完全に互換性があります。

## ネストした if

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

式がネストするとき、YaoXiang には C 言語のような「ぶら下がり else」の曖昧さがありません——各 `else`
は常に、まだペアになっていない最も近い `if` に対応します。

## ブール演算子で条件を組み合わせる

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

// 組み合わせて使う
if (age >= 18 and age <= 60) or is_vip {
    print("可以参加活动")
}
```

演算子の優先順位は、`not` が `and` より高く、`and` が `or`
より高いです。心配なときは括弧を付けて、意図をより明確にしましょう。

## まとめ

| ポイント     | 説明                                                            |
| ------------ | --------------------------------------------------------------- |
| 基本構造     | `if 条件 { ... } else if 条件 { ... } else { ... }`             |
| else if      | YaoXiang では `else if` で多分岐を実現する                      |
| 式           | `if` は値を返せる。すべての分岐の型は一致している必要がある     |
| 分岐の戻り値 | 分岐ブロック内の最後の式の値が戻り値になる                      |
| ネスト       | `if` の中にさらに `if` を書ける。ぶら下がり else の曖昧さはない |
| ブール演算   | `and`、`or`、`not` で条件を組み合わせる                         |

次の章では、`for` ループ——コレクションや範囲を反復処理する標準的な方法を学びます。
