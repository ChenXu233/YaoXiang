---
title: 'std.weak'
description: 'Arc / Weak 弱参照'
---

# std.weak

弱参照モジュール。`Arc` と組み合わせて使用し、参照サイクルを断ち切る。

```yaoxiang
use std.weak
```

> 本モジュールはアトミック参照カウントに依存しており、`wasm32`
> ターゲット上には**エクスポートされない**。

## 関数一覧

<!-- stdlib:table:weak start -->

| 関数      | シグネチャ                                   |
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

- `arc` —— 強参照値。値渡しで渡され、呼び出し後に**ムーブ**される。

戻り値：同一の割り当てブロックを指す `Weak` ハンドル。強参照カウントは**増やさない**。

```yaoxiang
use std.assert
use std.weak

main: () -> Void = {
    // ref は Arc[Int] を作成する
    p = ref 42

    // Arc → Weak への登録
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

弱参照を強参照への昇格を試みる。

- `weak` —— 弱参照ハンドル

戻り値：割り当てブロックが生存している場合は `Option.some(Arc)`、既に解放されている場合は
`Option.none()`。**エラーは発生しない**——「対象がまだ存在するか」を `Option` で表現する。

```yaoxiang
use std.assert
use std.weak

main: () -> Void = {
    p = ref 42
    w = weak.new(p)

    // 対象が生存している場合：some バリアントが得られる
    u = weak.upgrade(w)
    assert(true)
}
```

> **構文制限**：`Option`
> のバリアント分解（`match some(v)`）構文はまだ実装されていないため、現時点では呼び出しが成功したことしか検証できず、ソースコード内で
> `some` / `none` を分岐処理することはできない。詳細は `src/std/tests/weak_ops.yx` の説明を参照。

## セマンティクス説明

弱参照は所有権を**保持しない**：`Weak`
が存在しても対象の解放を妨げない。典型的な用途は循環参照を断ち切ることである——親ノードが子ノードを指す
`Arc` を保持し、子ノードは親ノードを指す `Weak` のみを保持することで、サイクルが断ち切られる。

## 関連

- [言語仕様：型システム](../language-spec/type-system.md) —— `Arc` / `Weak` の所有権セマンティクス
