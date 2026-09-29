---
title: 'std.weak'
description: 'Arc / Weak 弱参照'
---

# std.weak

弱参照モジュール。`Arc` と組み合わせて使用することで参照環を断ち切る。

```yaoxiang
use std.weak
```

> 本モジュールはアトミック参照カウントに依存しており、`wasm32`
> ターゲットでは**エクスポートされません**。

## 関数一覧

<!-- stdlib:table:weak start -->

| 関数      | 署名                                         |
| --------- | -------------------------------------------- |
| `new`     | `(T: Type)(arc: Arc(T)) -> Weak(T)`          |
| `upgrade` | `(T: Type)(weak: Weak(T)) -> Option(Arc(T))` |

<!-- stdlib:table:weak end -->

## 関数

### new

<!-- stdlib:sig:weak.new start -->

```yaoxiang
new: (T: Type)(arc: Arc(T)) -> Weak(T)
```

<!-- stdlib:sig:weak.new end -->

`Arc` から対応する弱参照を作成する。

- `arc` —— 強参照値。値渡しされ、呼び出し後に**移動**される

戻り値：同じ割り当てブロックを指す `Weak` ハンドル。強参照カウントは**増加しない**。

```yaoxiang
use std.assert
use std.weak

main: () -> Void = {
    // ref で Arc[Int] を作成
    p = ref 42

    // Arc → Weak 登録
    w = weak.new(p)
    assert(true)
}
```

### upgrade

<!-- stdlib:sig:weak.upgrade start -->

```yaoxiang
upgrade: (T: Type)(weak: Weak(T)) -> Option(Arc(T))
```

<!-- stdlib:sig:weak.upgrade end -->

弱参照を強参照に昇格することを試みる。

- `weak` —— 弱参照ハンドル

戻り値：割り当てブロックが生存している場合は `Option.some(Arc)`、解放済みの場合は
`Option.none()`。**エラーは報告されない**——`Option` で「対象がまだ存在するか」を表現する。

```yaoxiang
use std.weak
use std.option

main: () -> Void = {
    p = ref 42
    w = weak.new(p)

    // upgrade：対象が生存していれば some(v)、解放済みなら none()
    u = weak.upgrade(w)
    match u {
        some(v) => println("alive"),
        none() => println("dropped"),
    }
}
```

> **バリアント分解の前提**：`Option` のバリアント分解にはバリアント集合が必要です——`use std.option`
> でインポートすれば `match some(v)` / `none()` が利用可能になります（言語仕様 §2.8 match を参照）。

## 意味論的説明

弱参照は**所有権を保持しない**：`Weak`
が存在しても対象が解放されるのを防ぎません。典型的な用途は循環参照の解消です——親ノードが子ノードへの
`Arc` を保持し、子ノードは親ノードを指す `Weak` のみを保持することで、環を断ち切ります。

## 関連

- [言語仕様：型システム](../language-spec/type-system.md) —— `Arc` / `Weak` の所有権意味論
