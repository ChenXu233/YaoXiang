---
title: '基本フォーマット規則'
description: インデント、行幅、演算子、コードブロックのフォーマット規則
---

# 基本フォーマット規則

---

## §1 インデント

**§1.1 インデント幅。** デフォルトでは 4 スペースでインデントする。`indent_width`
設定項目で変更可能。

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

**§1.2 タブインデント。** `use_tabs = true` の場合、タブ文字でインデントする。デフォルトは `false`。

**§1.3 インデントの一貫性。** 同一ファイル内でタブとスペースを混在させてはならない。

---

## §2 行幅

**§2.1 最大行幅。** デフォルトの最大行幅は 120 文字。`line_width` 設定項目で変更可能。

**§2.2 改行戦略。**
1 行が最大行幅を超える場合、適切な位置で改行しなければならない。改行位置の優先順位：

1. 低優先度演算子の後（`+`, `-`, `or`, `and`, `=`）
2. 関数の引数リスト
3. リスト／辞書要素
4. 高優先度演算子の後（`*`, `/`, `%`, `==`, `!=`）

**§2.3 改行インデント。** 改行後の内容は 1 レベル多くインデントしなければならない。

```
// 行幅超過時の改行
let result = very_long_variable_name + another_long_name + yet_another_long_name;

// フォーマット後
let result = very_long_variable_name
    + another_long_name
    + yet_another_long_name;
```

---

## §3 演算子

**§3.1 演算子のスペース。** 二項演算子の両側にはスペースを入れなければならない。

```
// ✅ 正しい
let x = 1 + 2;
let y = a == b;

// ❌ 間違い
let x = 1+2;
let y = a==b;
```

**§3.2 単項演算子。** 単項演算子とオペランドの間にはスペースを入れない。

```
// ✅ 正しい（not はキーワード演算子なのでスペースが必要）
let x = -1;
let y = not flag;
let z = *ptr;

// ❌ 間違い
let x = - 1;
let y = not(flag);
```

**§3.3 低優先度演算子の改行。** 式が行幅を超える場合、低優先度演算子を行頭に置く。

```
// 行幅超過時
let result = first_value + second_value + third_value + fourth_value;

// フォーマット後
let result = first_value
    + second_value
    + third_value
    + fourth_value;
```

**§3.4 高優先度演算子の改行。** 高優先度演算子を行頭に置く。

```
// 行幅超過時
let result = first_value * second_value / third_value % fourth_value;

// フォーマット後
let result = first_value
    * second_value
    / third_value
    % fourth_value;
```

---

## §3.5 変数参照

**§3.5.1 変数名。** 変数参照は変数名をそのまま出力し、余分なスペースを追加しない。

```
// ✅ 正しい
let x = my_variable;
let y = camelCaseName;

// ❌ 間違い
let x = my_variable ;  // 余分なスペース
let y = "camelCaseName";  // クォートで囲むべきでない
```

---

## §6 コードブロック

**§6.1 コードブロックの書式。** コードブロックは中括弧 `{}`
で囲み、開始括弧の前にスペースを 1 つ入れる。

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
コードブロックが 1 行のみで全体の長さが行幅に収まる場合、単一行形式を使用できる。

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

**§6.3 空のコードブロック。** 空のコードブロックは `{}` で表す。

```
// ✅ 正しい
fn foo() {}

// ❌ 間違い
fn foo() {
}
```
