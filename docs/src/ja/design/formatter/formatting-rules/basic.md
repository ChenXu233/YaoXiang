---
title: "基礎フォーマット規則"
description: インデント、行幅、演算子、コードブロックのフォーマット規則
---

# 基礎フォーマット規則

---

## §1 インデント

**§1.1 インデント幅。** デフォルトでは4つのスペースを使用します。`indent_width`設定項目で変更可能です。

```
// デフォルトのインデント（4スペース）
fn foo() {
    let x = 1;
    if x > 0 {
        print(x);
    }
}

// 2スペースのインデント（indent_width = 2）
fn foo() {
  let x = 1;
  if x > 0 {
    print(x);
  }
}
```

**§1.2 Tab インデント。** `use_tabs = true` の場合、Tab文字を使用します。デフォルトは `false` です。

**§1.3 インデント一貫性。** 同一ファイル内でTabとスペースを混在させることはできません。

---

## §2 行幅

**§2.1 最大行幅。** デフォルトの最大行幅は120文字です。`line_width`設定項目で変更可能です。

**§2.2 折り返し戦略。** 行が最大行幅を超える場合、適切な位置で折り返す必要があります。折り返し位置の優先順位：

1. 低優先度演算子の後（`+`, `-`, `||`, `&&`, `=`）
2. 関数引数リスト
3. リスト/辞書要素
4. 高優先度演算子の後（`*`, `/`, `%`, `==`, `!=`）

**§2.3 折り返しインデント。** 折り返した後は、インデントを1レベル増加させる必要があります。

```
// 行幅を超える場合の折り返し
let result = very_long_variable_name + another_long_name + yet_another_long_name;

// フォーマット後
let result = very_long_variable_name
    + another_long_name
    + yet_another_long_name;
```

---

## §3 演算子

**§3.1 演算子のスペース。** 二項演算子の両側にスペースが必要です。

```
// ✅ 正しい
let x = 1 + 2;
let y = a == b;

// ❌ 間違い
let x = 1+2;
let y = a==b;
```

**§3.2 単項演算子。** 単項演算子と被演算子の間にはスペースを入れません。

```
// ✅ 正しい
let x = -1;
let y = !flag;
let z = *ptr;

// ❌ 間違い
let x = - 1;
let y = ! flag;
```

**§3.3 低優先度演算子の折り返し。** 式が行幅を超える場合、低優先度演算子を新しい行の先頭に配置します。

```
// 行幅を超える場合
let result = first_value + second_value + third_value + fourth_value;

// フォーマット後
let result = first_value
    + second_value
    + third_value
    + fourth_value;
```

**§3.4 高優先度演算子の折り返し。** 高優先度演算子を新しい行の先頭に配置します。

```
// 行幅を超える場合
let result = first_value * second_value / third_value % fourth_value;

// フォーマット後
let result = first_value
    * second_value
    / third_value
    % fourth_value;
```

---

## §6 コードブロック

**§6.1 コードブロックの形式。** コードブロックは波括弧 `{}` で囲み、開括弧の前にスペースを1つ入れます。

```
// ✅ 正しい
fn foo() {
    let x = 1;
}

// ❌ 間違い
fn foo(){
    let x = 1;
}
fn foo()
{
    let x = 1;
}
```

**§6.2 単一行コードブロック。** コードブロックが1行のみで、合計の長さが行幅を超えない場合、単一行形式を使用できます。

```
// ✅ 単一行形式
fn foo() { 1 }

// ✅ 複数行形式
fn foo() {
    let x = 1;
    let y = 2;
    x + y
}
```

**§6.3 空のコードブロック。** 空のコードブロックは `{}` で表します。

```
// ✅ 正しい
fn foo() {}

// ❌ 間違い
fn foo() {
}
```