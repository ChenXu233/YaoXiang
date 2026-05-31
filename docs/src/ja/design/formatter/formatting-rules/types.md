```yaml
---
title: "型システムフォーマット規則"
description: 型注釈、参照と借用、型変換のフォーマット規則
---

# 型システムフォーマット規則

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

**§9.2 関数パラメータの型。** パラメータ名と型の間は `: ` で接続します。

```
// ✅ 正しい
fn foo(x: Int, y: String) { ... }

// ❌ 間違い
fn foo(x:Int, y:String) { ... }
```

**§9.3 ジェネリックパラメータ。** ジェネリックパラメータは `(T: Constraint)` 形式を使用します。

```
// ✅ 正しい
fn foo<T: Clone>(x: T) { ... }

// ❌ 間違い
fn foo <T:Clone> (x: T) { ... }
```

---

## §15 参照と借用

**§15.1 変更不能な参照。** `&expr` 形式を使用します。

```
// ✅ 正しい
let x = &value;

// ❌ 間違い
let x = & value;
```

**§15.2 変更可能な参照。** `&mut expr` 形式を使用します。

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

**§16.1 as変換。** `expr as Type` 形式を使用します。

```
// ✅ 正しい
let x = value as Int;

// ❌ 間違い
let x = value as Int;
let x = value  as  Int;
```