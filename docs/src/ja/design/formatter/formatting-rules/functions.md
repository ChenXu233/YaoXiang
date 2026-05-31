---
title: "関数関連のフォーマット規則"
description: 関数定義、関数呼び出し、Lambda式のフォーマット規則
---

# 関数関連のフォーマット規則

---

## §4 関数定義

**§4.1 関数シグネチャ。** 関数名とパラメータリストの間にスペースを入れない。

```
// ✅ 正しい
fn foo(a: Int, b: Int) -> Int { ... }

// ❌ 間違い
fn foo (a: Int, b: Int) -> Int { ... }
```

**§4.2 パラメータリストの改行。** パラメータリストが行幅を超える場合は、各パラメータを1行に置き、末尾カンマを付ける。

```
// 行幅を超える場合
fn very_long_function_name(first_param: Int, second_param: Int, third_param: Int) -> Int { ... }

// フォーマット後
fn very_long_function_name(
    first_param: Int,
    second_param: Int,
    third_param: Int,
) -> Int { ... }
```

**§4.3 戻り値の型。** 戻り値の型とパラメータリストの間は ` -> ` で接続し、`->` の前後にはスペースを1つ入れる。

```
// ✅ 正しい
fn foo() -> Int { ... }

// ❌ 間違い
fn foo()->Int { ... }
fn foo() ->Int { ... }
fn foo()-> Int { ... }
```

**§4.4 関数本体。** 関数本体と戻り値の型の間はスペース1つで区切る。

```
// ✅ 正しい
fn foo() -> Int { 1 }

// ❌ 間違い（スペース2つ）
fn foo() -> Int  { 1 }
```

---

## §7 関数呼び出し

**§7.1 パラメータリスト。** パラメータの間はカンマで区切り、カンマの後にはスペースを1つ入れる。

```
// ✅ 正しい
foo(1, 2, 3)

// ❌ 間違い
foo(1,2,3)
foo(1 , 2 , 3)
```

**§7.2 名前付き引数。** 名前付き引数は `name = value` 形式を使用する。

```
// ✅ 正しい
foo(x = 1, y = 2)

// ❌ 間違い
foo(x=1, y=2)
```

**§7.3 引数の改行。** 引数リストが行幅を超える場合は、各引数を1行に置き、末尾カンマを付ける。

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

## §12 Lambda式

**§12.1 Lambda形式。** Lambdaは `(params) => body` 形式を使用する。

```
// ✅ 正しい
let f = (x) => x + 1;

// 単一式body
let f = (x) => x * 2;

// 複数文body
let f = (x) => {
    let y = x + 1;
    y * 2
};
```