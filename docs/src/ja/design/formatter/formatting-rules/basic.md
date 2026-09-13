---
title: '基本フォーマット規則'
description: 'インデント、行幅、演算子、コードブロックのフォーマット規則'
---

# 基本フォーマット規則

---

## §1 インデント

**§1.1 インデント幅。** デフォルトでは 4 つのスペースでインデントします。`indent_width`
設定項目で変更可能です。

```
// 默认缩进（4 空格）
fn foo() {
    let x = 1;
    if x > 0 {
        print(x);
    }
}

// 2 空格缩进（indent_width = 2）
fn foo() {
  let x = 1;
  if x > 0 {
    print(x);
  }
}
```

**§1.2 タブインデント。** `use_tabs = true` の場合、タブ文字でインデントします。デフォルトは `false`
です。

**§1.3 インデントの一貫性。** 同一ファイル内でタブとスペースを混在させてはいけません。

---

## §2 行幅

**§2.1 最大行幅。** デフォルトの最大行幅は 120 文字です。`line_width` 設定項目で変更可能です。

**§2.2 改行戦略。**
1 行が最大行幅を超える場合、適切な位置で改行しなければなりません。改行位置の優先順位：

1. 優先度の低い演算子の後（`+`, `-`, `or`, `and`, `=`）
2. 関数の引数リスト
3. リスト/辞書要素
4. 優先度の高い演算子の後（`*`, `/`, `%`, `==`, `!=`）

**§2.3 改行インデント。** 改行後の内容は 1 レベルのインデントを追加しなければなりません。

```
// 超过行宽时换行
let result = very_long_variable_name + another_long_name + yet_another_long_name;

// 格式化后
let result = very_long_variable_name
    + another_long_name
    + yet_another_long_name;
```

---

## §3 演算子

**§3.1 演算子のスペース。** 二項演算子の両側にはスペースが必要です。

```
// ✅ 正确
let x = 1 + 2;
let y = a == b;

// ❌ 错误
let x = 1+2;
let y = a==b;
```

**§3.2 単項演算子。** 単項演算子とオペランドの間にはスペースを入れません。

```
// ✅ 正确（! 是紧绑定一元运算符，不加空格）
let x = -1;
let y = !flag;
let z = *ptr;

// ❌ 错误
let x = - 1;
let y = ! flag;
```

**§3.3 優先度の低い演算子の改行。**
式が行幅を超える場合、優先度の低い演算子は新しい行の先頭に置きます。

```
// 超过行宽时
let result = first_value + second_value + third_value + fourth_value;

// 格式化后
let result = first_value
    + second_value
    + third_value
    + fourth_value;
```

**§3.4 優先度の高い演算子の改行。** 優先度の高い演算子は新しい行の先頭に置きます。

```
// 超过行宽时
let result = first_value * second_value / third_value % fourth_value;

// 格式化后
let result = first_value
    * second_value
    / third_value
    % fourth_value;
```

---

## §3.5 変数参照

**§3.5.1 変数名。** 変数参照は変数名を直接出力し、余分なスペースを追加しません。

```
// ✅ 正确
let x = my_variable;
let y = camelCaseName;

// ❌ 错误
let x = my_variable ;  // 多余空格
let y = "camelCaseName";  // 不应加引号
```

---

## §6 コードブロック

**§6.1 コードブロックの書式。** コードブロックは中括弧 `{}`
で囲み、開き括弧の前に 1 つのスペースを入れます。

```
// ✅ 正确
fn foo() {
    let x = 1;
}

// ❌ 错误
fn foo(){
    let x = 1;
}
fn foo()
{
    let x = 1;
}
```

**§6.2 単一行コードブロック。**
コードブロックが 1 行のみで全長が行幅を超えない場合、単行形式を使用できます。

```
// ✅ 单行格式
fn foo() { 1 }

// ✅ 多行格式
fn foo() {
    let x = 1;
    let y = 2;
    x + y
}
```

**§6.3 空のコードブロック。** 空のコードブロックは `{}` で表します。

```
// ✅ 正确
fn foo() {}

// ❌ 错误
fn foo() {
}
```
