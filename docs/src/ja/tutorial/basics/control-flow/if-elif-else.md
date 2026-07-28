---
title: if-else-if-else
---

# if-else-if-else

`if-else-if-else`
はプログラミングにおいて最も基本的な意思決定ツールです。そのロジックは非常に直感的です——**条件が成立すれば、あるコードを実行し、そうでなければ次の条件をチェックし、どれも成立しなければデフォルトの経路に進む**。

## 基本構文

構文仕様において `if` 式と `if` 文の定義は完全に一致しています：

```
if Expr Block ('else' 'if' Expr Block)* ('else' Block)?
```

日常言語で解釈すると：`if` で始まり、その後に条件式とコードブロックを書き、その後ゼロ個以上の
`else if 条件 コードブロック`を続き、最後にオプションで
`else コードブロック`を一つ置くことができます。

最もシンプルな形式——`if` のみ：

```yaoxiang
if temperature > 30 {
    print("天気が暑い、エアコンをつけよう")
}
```

`else` を加える：

```yaoxiang
if is_raining {
    print("傘を持っていく")
} else {
    print("傘はいらない")
}
```

複数の条件には `else if` を使用：

```yaoxiang
score = 85

if score >= 90 {
    print("優秀")
} else if score >= 80 {
    print("良好")
} else if score >= 60 {
    print("合格")
} else {
    print("もっと的努力が必要")
}
```

## if は式である

これは YaoXiang の制御フローにおける最も重要な特性の一つです：**`if`
は式として使用でき、値を計算できます**。

```yaoxiang
// if 式：各分支の値が result に代入される
result = if x > 0 {
    "正の数"
} else if x < 0 {
    "負の数"
} else {
    "零"
}
// result は今 "正の数"、"負の数"、または "零" のいずれか
```

`if` を式として使用する場合、すべての分支の戻り値の型が一致している必要があります：

```yaoxiang
score = 88

// すべての分支が String を返し、型が一致しているので問題なし
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

各分支のコードブロックにおいて、**最後の式の値がその分支の戻り値になります**。`return`
で明示的に返すこともできますが、branch 内では通常、式を直接書けば十分です。

```yaoxiang
// 式を直接書く——推奨
category = if age < 18 { "未成年" } else { "成人" }

// 明示的に return を使う也可以——効果は同じ
category = if age < 18 {
    return "未成年"
} else {
    return "成人"
}
```

もし条件判断だけで値が必要ない場合は、単なる文として使用すれば問題ありません——式形式と完全に互換性があります。

## ネストされた if

`if` の中にさらに `if` を書くことができ、複数のレベルの条件判断を処理できます：

```yaoxiang
age = 25
has_ticket = true

if age >= 18 {
    if has_ticket {
        print("入場歓迎")
    } else {
        print("先にチケットを買ってください")
    }
} else {
    print("未成年は保護者同伴が必要です")
}
```

式がネストされている場合、YaoXiang には C 言語のような「dangling
else（宙吊り else）」の曖昧さはなく、すべての `else` は常に最も近くてまだペアになっていない `if`
に属します。

## ブール演算子で条件を組合せる

条件の中で `and`、`or`、`not` を使用して複数の判断を組合せることもできます：

```yaoxiang
username = "admin"
password = "123456"

// and：両方の条件が成立
if username == "admin" and password == "123456" {
    print("ログイン成功")
}

// or：いずれかの条件が成立
if role == "admin" or role == "moderator" {
    print("管理権限あり")
}

// not：否定
if not is_banned {
    print("発言許可")
}

// 組合せて使用
if (age >= 18 and age <= 60) or is_vip {
    print("イベント参加可能")
}
```

演算子の優先順位は、`not` が `and` より高く、`and` が `or`
より高いです。心配な場合は括弧を追加して、意図を明確にしましょう。

## まとめ

| 要点           | 説明                                                    |
| -------------- | ------------------------------------------------------- |
| 基本構造       | `if 条件 { ... } else if 条件 { ... } else { ... }`     |
| else if        | YaoXiang は `else if` で多方向分岐を実装する            |
| 式             | `if` は値を返すことができ、すべてのbranchの型が一致必須 |
| branchの戻り値 | branchブロック内の最後の式の値が戻り値になる            |
| ネスト         | `if` 内にさらに `if` を書ける、宙吊りelseの曖昧さはない |
| ブール演算     | `and`、`or`、`not` で条件を組合せる                     |

次の章では `for` ループ——コレクションと範囲を反復する標準的な方法を学びます。
