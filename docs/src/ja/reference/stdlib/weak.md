---
title: 'std.weak'
description: 'Arc / Weak 弱参照'
---

# std.weak

弱参照モジュール。`Arc` と組み合わせて参照サイクルを断ち切るために使用します。

```yaoxiang
use std.weak
```

> 本モジュールはアトミック参照カウントに依存しており、`wasm32`
> ターゲットでは**エクスポートされません**。

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

`Arc` から対応する弱参照を作成します。

- `arc` —— 強参照の値。値渡しされ、呼び出し後に**ムーブ**される

戻り値：同一の割り当てブロックを指す `Weak` ハンドル。強参照カウントは**増加しない**。

```yaoxiang
use std.assert
use std.weak

main = {
    // ref 创建 Arc[Int]
    p = ref 42

    // Arc → Weak 登记
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

弱参照を強参照へ昇格することを試みます。

- `weak` —— 弱参照ハンドル

戻り値：割り当てブロックが生存している場合は `Option.some(Arc)`、解放済みの場合は
`Option.none()`。**エラーは発生しない**——`Option` を用いて「対象がまだ存在するか」を表現します。

```yaoxiang
use std.assert
use std.weak

main = {
    p = ref 42
    w = weak.new(w)

    // 目标存活：得到 some 变体
    u = weak.upgrade(w)
    assert(true)
}
```

> **構文上の制約**：`Option`
> のバリアント分解（`match some(v)`）構文がまだ実装されていないため、現時点では呼び出しが成功したことしか検証できず、ソースコード内で
> `some` / `none` を分岐処理できません。詳細は `src/std/tests/weak_ops.yx`
> の説明を参照してください。

## 意味論の説明

弱参照は**所有権を保持しない**：`Weak`
が存在しても対象の解放は妨げられません。典型的な用途は循環参照の打破です——親ノードが子ノードへの
`Arc` を保持し、子ノードは親ノードを指す `Weak` のみを保持すれば、循環は断ち切られます。

## 関連

- [言語仕様：型システム](../language-spec/type-system.md) —— `Arc` / `Weak` の所有権セマンティクス
