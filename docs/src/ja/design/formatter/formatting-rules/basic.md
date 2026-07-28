---
title: '基礎フォーマットルール'
description: インデント、行幅、演算子、コードブロックのフォーマットルール
---

# 基礎フォーマットルール

---

## §1 インデント

**§1.1 インデント幅。** デフォルトで4スペースのインデントを使用。`indent_width`設定項目で変更可能。

```
// デフォルトインデント（4スペース）
fn foo() {
    let x = 1;
    if x > 0 {
        print(x);
    }
}

// 2スペースインデント（indent_width = 2）
fn foo() {
  let x = 1;
  if x > 0 {
    print(x);
  }
}
```

**§1.2 タブによるインデント。** `use_tabs = true`の場合、タブ文字でインデント。デフォルトは`false`。

**§1.3 インデントの一貫性。** 同一ファイル内でタブとスペースを混用してはならない。

---

## §2 行幅

**§2.1 最大行幅。** デフォルトの最大行幅は120文字。`line_width`設定項目で変更可能。

**§2.2 改行戦略。** 行が最大行幅を超える場合、適切な位置で改行する必要がある。改行位置の優先順位：

1. 低優先度演算子の後（`+`, `-`, `||`, `&&`, `=`）
2. 関数引数リスト
3. リスト/辞書要素
4. 高優先度演算子の後（`*`, `/`, `%`, `==`, `!=`）

**§2.3 改行後のインデント。** 改行後の内容は必ず一段インデントを増やす。

```
// 行幅を超える場合の改行
let result = very_long_variable_name + another_long_name + yet_another_long_name;

// フォーマット後
let result = very_long_variable_name
    + another_long_name
    + yet_another_long_name;
```

---

## §3 演算子

**§3.1 演算子のスペース。** 二項演算子の両側にスペースが必要。

```
// ✅ 正しい
let x = 1 + 2;
let y = a == b;

// ❌ 間違い
let x = 1+2;
let y = a==b;
```

**§3.2 単項演算子。** 単項演算子と被演算子の間にはスペースを入れない。

```
// ✅ 正しい
let x = -1;
let y = !flag;
let z = *ptr;

// ❌ 間違い
let x = - 1;
let y = ! flag;
```

**§3.3 低優先度演算子の改行。** 式が行幅を超える場合、低優先度演算子は新しい行の先頭に置く。

```
// 行幅を超える場合
let result = first_value + second_value + third_value + fourth_value;

// フォーマット後
let result = first_value
    + second_value
    + third_value
    + fourth_value;
```

**§3.4 高優先度演算子の改行。** 高優先度演算子は新しい行の先頭に置く。

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

## §3.5 変数の参照

**§3.5.1 変数名。** 変数の参照は変数名をそのまま出力し、追加のスペースは入れない。

```
// ✅ 正しい
let x = my_variable;
let y = camelCaseName;

// ❌ 間違い
let x = my_variable ;  // 余分なスペース
let y = "camelCaseName";  // 引用符は不要
```

---

## §6 コードブロック

**§6.1 コードブロックの形式。** コードブロックは波括弧`{}`で囲み、開括弧の前にスペースを1つ入れる。

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

**§6.2 単一行コードブロック。**
コードブロックが1行のみで、总横幅が行幅を超えない場合、単一行形式可以使用。

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

**§6.3 空のコードブロック。** 空のコードブロックは`{}`で表す。

```
// ✅ 正しい
fn foo() {}

// ❌ 間違い
fn foo() {
}
```
