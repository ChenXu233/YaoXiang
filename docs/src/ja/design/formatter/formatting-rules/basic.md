---
title: '基礎フォーマットルール'
description: インデント、行幅、演算子、コードブロックのフォーマットルール
---

# 基礎フォーマットルール

---

## §1 インデント

**§1.1 インデント幅。** デフォルトでは 4 つのスペースでインデントします。`indent_width`
設定項目で変更可能です。

```
// デフォルトインデント（4 スペース）
fn foo() {
    let x = 1;
    if x > 0 {
        print(x);
    }
}

// 2 スペースインデント（indent_width = 2）
fn foo() {
  let x = 1;
  if x > 0 {
    print(x);
  }
}
```

**§1.2 Tab インデント。** `use_tabs = true` の場合、tab 文字を使用してインデントします。デフォルトは
`false` です。

**§1.3 インデントの一貫性。** 同一ファイル内で tab とスペースを混在させてはいけません。

---

## §2 行幅

**§2.1 最大行幅。** デフォルトの最大行幅は 120 文字です。`line_width` 設定項目で変更可能です。

**§2.2 改行戦略。**
1 行が最大行幅を超える場合、適切な位置で改行しなければなりません。改行位置の優先順位：

1. 低優先度の演算子の後（`+`, `-`, `or`, `and`, `=`）
2. 関数の引数リスト
3. リスト/辞書要素
4. 高優先度の演算子の後（`*`, `/`, `%`, `==`, `!=`）

**§2.3 改行時のインデント。** 改行後の内容は 1 レベルのインデントを追加しなければなりません。

```
// 行幅を超えた場合の改行
let result = very_long_variable_name + another_long_name + yet_another_long_name;

// フォーマット後
let result = very_long_variable_name
    + another_long_name
    + yet_another_long_name;
```

---

## §3 演算子

**§3.1 演算子のスペース。** 二項演算子の両側にはスペースが必要です。

```
// ✅ 正しい
let x = 1 + 2;
let y = a == b;

// ❌ 間違い
let x = 1+2;
let y = a==b;
```

**§3.2 単項演算子。** 単項演算子とオペランドの間にはスペースを入れません。

```
// ✅ 正しい（! は密着する単項演算子なので、スペースを入れない）
let x = -1;
let y = !flag;
let z = *ptr;

// ❌ 間違い
let x = - 1;
let y = ! flag;
```

**§3.3 低優先度演算子での改行。**
式が行幅を超える場合、低優先度の演算子は新しい行の先頭に配置します。

```
// 行幅を超える場合
let result = first_value + second_value + third_value + fourth_value;

// フォーマット後
let result = first_value
    + second_value
    + third_value
    + fourth_value;
```

**§3.4 高優先度演算子での改行。** 高優先度の演算子は新しい行の先頭に配置します。

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

## §3.5 変数参照

**§3.5.1 変数名。** 変数参照は変数名を直接出力し、余分なスペースを追加しません。

```
// ✅ 正しい
let x = my_variable;
let y = camelCaseName;

// ❌ 間違い
let x = my_variable ;  // 余分なスペース
let y = "camelCaseName";  // クォートで囲むべきではない
```

---

## §6 コードブロック

**§6.1 コードブロックの形式。** コードブロックは中括弧 `{}`
で囲み、開始括弧の前に 1 つのスペースを入れます。

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
コードブロックが 1 行のみで全体の長さが行幅を超えない場合、単一行形式を使用できます。

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
