---
title: '型システムフォーマットルール'
description: 型注釈、参照と借用、型変換のフォーマットルール
---

# 型システムフォーマットルール

---

## §9 型注釈

**§9.1 変数型注釈。** 型注釈は `: Type` 形式を使用し、コロンの後にスペースを1つ入れます。

```
// ✅ 正しい
let x: Int = 1;

// ❌ 間違い
let x:Int = 1;
let x : Int = 1;
```

**§9.2 関数パラメータ型。** パラメータ名と型の間は `: ` で接続します。

```
// ✅ 正しい
fn foo(x: Int, y: String) { ... }

// ❌ 間違い
fn foo(x:Int, y:String) { ... }
```

**§9.3 ジェネリクス型パラメータ。** ジェネリクス型パラメータは `(T: Constraint)` 形式を使用します。

```
// ✅ 正しい
fn foo<T: Clone>(x: T) { ... }

// ❌ 間違い
fn foo <T:Clone> (x: T) { ... }
```

---

## §15 参照と借用

**§15.1 不変参照。** `&expr` 形式を使用します。

```
// ✅ 正しい
let x = &value;

// ❌ 間違い
let x = & value;
```

**§15.2 可変参照。** `&mut expr` 形式を使用します。

```
// ✅ 正しい
let x = &mut value;

// ❌ 間違い
let x = &mut  value;
let x = & mut value;
```

**§15.3 型の中の参照。** 型の中の参照は `&Type` または `&mut Type` 形式を使用します。

```
// ✅ 正しい
fn foo(x: &Int) { ... }
fn bar(x: &mut Int) { ... }
```

---

## §16 型変換

**§16.1 as 変換。** `expr as Type` 形式を使用します。

```
// ✅ 正しい
let x = value as Int;

// ❌ 間違い
let x = value as Int;
let x = value  as  Int;
```

---

## §17 Ref キーワード

**§17.1 Ref 形式。** `ref` キーワードと式の間にスペースを入れます。

```
// ✅ 正しい
let x = ref value;
let y = ref obj;

// ❌ 間違い
let x = refvalue;  // スペース不足
let y = ref  value;  // 余分なスペース
```

**§17.2 Ref セマンティクス。** `ref` は Arc（アトミック参照カウント）のコピーを作成します。

```
// 共有参照を作成
let shared = ref original;
```
