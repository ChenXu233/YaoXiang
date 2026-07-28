---
title: '関数関連フォーマット規則'
description: '関数定義、関数呼び出し、Lambda式のフォーマット規則'
---

# 関数関連フォーマット規則

---

## §4 関数定義

**§4.1 関数シグネチャ。** 関数名とパラメータリストの間にはスペースを入れない。

```
// ✅ 正しい
foo: (a: Int, b: Int) -> Int = a + b

// ❌ 誤り
foo : (a: Int, b: Int) -> Int = a + b
```

**§4.2 パラメータリストの改行。** パラメータリストが行幅を超える場合は、各パラメータを1行に記述し、末尾にカンマを付ける。

```
// 行幅を超える場合
very_long_function_name: (first_param: Int, second_param: Int, third_param: Int) -> Int = first_param + second_param + third_param

// フォーマット後
very_long_function_name:
    first_param: Int,
    second_param: Int,
    third_param: Int,
) -> Int = first_param + second_param + third_param
```

**§4.3 戻り値の型。** 戻り値の型とパラメータリストの間は `->` で接続し、`->` の前後にはそれぞれスペースを1つ入れる。

```
// ✅ 正しい
foo: () -> Int = 1

// ❌ 誤り
foo: () ->Int = 1
foo: ()-> Int = 1
foo:()-> Int = 1
```

**§4.4 関数本体。** 関数本体と戻り値の型の間はスペース1つで区切る。

```
// ✅ 正しい
foo: () -> Int = 1

// ❌ 誤り（スペース2つ）
foo: () -> Int  = 1
```

---

## §7 関数呼び出し

**§7.1 パラメータリスト。** パラメータ間はカンマで区切り、カンマの後にはスペースを1つ入れる。

```
// ✅ 正しい
foo(1, 2, 3)

// ❌ 誤り
foo(1,2,3)
foo(1 , 2 , 3)
```

**§7.2 名前付き引数。** 名前付き引数は `name = value` 形式を使用する。

```
// ✅ 正しい
foo(x = 1, y = 2)

// ❌ 誤り
foo(x=1, y=2)
```

**§7.3 引数の改行。** 引数リストが行幅を超える場合は、各引数を1行に記述し、末尾にカンマを付ける。

```
// 行幅を超える場合
very_long_function_name(first_argument, second_argument, third_argument)

// フォーマット後
very_long_function_name(
    first_argument,
    second_argument,
    third_argument,
)
```

---

## §12 Lambda 式

**§12.1 Lambda 形式。** Lambda は `(params) => body` 形式を使用する。

```
// ✅ 正しい
f = (x) => x + 1

// 単一式ボディ
f = (x) => x * 2

// 複数文ボディ
f = (x) => {
    y = x + 1
    y * 2
}
```
