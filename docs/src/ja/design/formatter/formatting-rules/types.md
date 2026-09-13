---
title: '型システムのフォーマット規則'
description: '型注釈、参照と借用、型変換のフォーマット規則'
---

# 型システムのフォーマット規則

---

## §9 型注釈

**§9.1 変数型注釈。** 型注釈は `: Type` 形式を使用し、コロンの後にスペースを1つ入れる。

```
// ✅ 正确
let x: Int = 1;

// ❌ 错误
let x:Int = 1;
let x : Int = 1;
```

**§9.2 関数パラメータ型。** パラメータ名と型の間は `: ` で接続する。

```
// ✅ 正确
fn foo(x: Int, y: String) { ... }

// ❌ 错误
fn foo(x:Int, y:String) { ... }
```

**§9.3 ジェネリクスパラメータ。** ジェネリクスパラメータは `(T: Constraint)` 形式を使用する。

```
// ✅ 正确
fn foo<T: Clone>(x: T) { ... }

// ❌ 错误
fn foo <T:Clone> (x: T) { ... }
```

---

## §15 参照と借用

**§15.1 不変参照。** `&expr` 形式を使用する。

```
// ✅ 正确
let x = &value;

// ❌ 错误
let x = & value;
```

**§15.2 可変参照。** `&mut expr` 形式を使用する。

```
// ✅ 正确
let x = &mut value;

// ❌ 错误
let x = &mut  value;
let x = & mut value;
```

**§15.3 型における参照。** 型における参照は `&Type` または `&mut Type` 形式を使用する。

```
// ✅ 正确
fn foo(x: &Int) { ... }
fn bar(x: &mut Int) { ... }
```

---

## §16 型変換

**§16.1 as 変換。** `expr as Type` 形式を使用する。

```
// ✅ 正确
let x = value as Int;

// ❌ 错误
let x = value as Int;
let x = value  as  Int;
```

---

## §17 Ref キーワード

**§17.1 Ref のフォーマット。** `ref` キーワードと式の間はスペースで区切る。

```
// ✅ 正确
let x = ref value;
let y = ref obj;

// ❌ 错误
let x = refvalue;  // 缺少空格
let y = ref  value;  // 多余空格
```

**§17.2 Ref のセマンティクス。** `ref` は Arc（アトミック参照カウント）のコピーを作成する。

```
// 创建共享引用
let shared = ref original;
```
