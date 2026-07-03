---
title: "RFC-011a: インタフェース実装と動的ディスパッチ"
status: "審査中"
author: "晨煦"
created: "2026-06-14"
updated: "2026-07-03"
group: "rfc-011"
---

# RFC-011a: インタフェース実装と動的ディスパッチ

> **親 RFC**: [RFC-011: ジェネリクスシステム設計](../accepted/011-generic-type-system.md)
>
> **本 RFC は RFC-011 §2.1-2.4 のインタフェース制約部分を補完し、置き換えるものである。**

## 概要

RFC-011 はジェネリクスシステムを定義したが、インタフェース実装メカニズムについて詳細な規定がない。本文書では以下を補足する：

1. **インタフェース宣言**：型定義内で `impl` キーワードを使わず直接インタフェース名を記述
2. **メソッド実装**：内部宣言と外部宣言の両方をサポート
3. **オーバーロード規則**：シグネチャが異なればオーバーロード可能、シグネチャが同じならエラー（オーバーライド禁止）
4. **デフォルト値**：フィールドの後ろに直接 `= value` と記述
5. **動的ディスパッチ**：コンパイル時の型収集 + インタフェースマッチング、仮想テーブルなし

**中核設計**：

```yaoxiang
# インタフェース定義
Animal: Type = {
    speak: (Self) -> String,
}

# 型定義（内部宣言）
Dog: Type = {
    x: Int = 10,
    Animal,  # インタフェース宣言
    speak: (Self) -> String = "Woof",
}

# 外部宣言（オーバーロード）
Dog.speak: (Self, volume: Int) -> String = "WOOF"

# 異種コンテナ（動的ディスパッチ）
animals: List(Animal) = [Dog.new(), Cat.new()]
animals[0].speak()  # "Woof"
```

**除去された複雑性**：
- ❌ `impl` キーワード不要
- ❌ `dyn Trait + 'a` 注釈不要
- ❌ 仮想テーブルなし（コンパイル時型収集 + enum包装）
- ❌ オーバーライドなし（オーバーロード規則で統一）

---

## 動機

### RFC-011 の不備

RFC-011 はジェネリクスシステムを定義したが、以下の点が詳述されていない：

| 問題 | 説明 |
|------|------|
| インタフェース宣言構文 | 型がインタフェースを実装していることをどう宣言するか？ |
| メソッド実装の位置 | 内部宣言か外部宣言か？ |
| オーバーロード規則 | 同名のメソッドをどう処理するか？ |
| デフォルト値構文 | フィールドにデフォルト値を設定するには？ |
| 動的ディスパッチ | 異種コンテナをどう実現するか？ |

### 設計目標

1. **簡潔性**：`impl` キーワードが不要
2. **柔軟性**：メソッド実装は内部・外部両方をサポート
3. **統一性**：オーバーロード規則を一貫させる
4. **利便性**：デフォルト値構文が簡潔
5. **ゼロオーバーヘッド**：仮想テーブルなし、コンパイル時型収集

### Rust との比較

| 特性 | Rust | YaoXiang |
|------|------|----------|
| インタフェース宣言 | `impl Animal for Dog { ... }` | `Dog: Type = { Animal, ... }` |
| メソッド実装 | `impl` ブロック内 | 内部または外部 |
| オーバーロード | 非対応 | 対応（シグネチャが異なる場合） |
| デフォルト値 | `#[default]` が必要 | 直接 `= value` と記述 |
| 異種コンテナ | `Vec<Box<dyn Animal + 'a>>` | `List(Animal)` |
| 動的ディスパッチ | 仮想テーブルルックアップ | コンパイル時型収集 |

---

## 提案

### 1. インタフェース宣言

**中核規則**：型定義内でインタフェース名を直接記述し、`impl` キーワードを使わない。

```yaoxiang
# インタフェース定義
Animal: Type = {
    speak: (Self) -> String,
}

# 型がインタフェースを実装することを宣言
Dog: Type = {
    x: Int,
    Animal,  # インタフェース宣言
}
```

**コンパイラ処理**：
1. `Animal` がインタフェース型であることを識別
2. `Dog` が `Animal` の要求する全メソッドを持つかチェック
3. 合格 → 実装証明を生成
4. 失敗 → コンパイルエラー

**糖衣構文の等価性**：

```yaoxiang
Dog: Type = {
    x: Int,
    Animal,  # Animal のメソッドを展開したものと等価だが、出典マークを保持
}

# 等価（出典情報を保持）
Dog: Type = {
    x: Int,
    speak: (Self) -> String,  # Animal 由来
}
```

**出典マークが必要な理由**：
- 直接展開すると出典情報が失われる
- 出典マークは実装証明の生成に使用される
- ランタイムで証明を通じて正しいメソッドを見つける

### 2. メソッド実装

**中核規則**：メソッド実装は内部宣言と外部宣言の両方をサポート。

#### 2.1 内部宣言

```yaoxiang
Dog: Type = {
    x: Int = 10,
    Animal,
    speak: (Self) -> String = "Woof",  # メソッド実装は内部
}
```

#### 2.2 外部宣言

```yaoxiang
Dog: Type = {
    x: Int,
    Animal,
}

# メソッド実装は外部
Dog.speak: (Self) -> String = "Woof"
```

#### 2.3 混合宣言

```yaoxiang
Dog: Type = {
    x: Int = 10,
    Animal,
    speak: (Self) -> String = "Woof",  # 一部のメソッドは内部
}

# 一部のメソッドは外部
Dog.play: (Self) -> Void = { ... }
```

**コンパイラ処理**：
1. 全定義（内部と外部）を収集
2. シグネチャでグループ化（オーバーロード）
3. オーバーライドがないかチェック（エラー）
4. インタフェース完全性をチェック
5. 実装証明を生成

### 3. オーバーロードとオーバーライド

**中核規則**：
- シグネチャが異なる → オーバーロード → 許可
- シグネチャが同じ → オーバーライド → エラー

#### 3.1 オーバーロード（許可）

```yaoxiang
# 引数の型が異なる、オーバーロード可能
Dog.speak: (Self) -> String = "Woof"
Dog.speak: (Self, volume: Int) -> String = "WOOF"
```

#### 3.2 オーバーライド（禁止）

```yaoxiang
# シグネチャが完全一致、オーバーライド禁止
Dog.speak: (Self) -> String = "Woof"
Dog.speak: (Self) -> String = "Bark"  # ❌ エラー：オーバーライドは許可されない
```

**エラーメッセージ**：

```
エラー：Dog.speak(Self) -> String の重複定義
  --> ファイル2:5:1
  |
5 | Dog.speak: (Self) -> String = "Bark"
  | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ 重複定義
  |
  --> ファイル1:3:1
  |
3 | Dog.speak: (Self) -> String = "Woof"
  | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ 最初の定義
```

#### 3.3 規則の統一

**内部宣言と外部宣言は同じオーバーロード/オーバーライド規則に従う**：

```yaoxiang
# 内部宣言
Dog: Type = {
    x: Int,
    Animal,
    speak: (Self) -> String = "Woof",
}

# 外部宣言（オーバーロード、許可）
Dog.speak: (Self, volume: Int) -> String = "WOOF"

# 外部宣言（オーバーライド、禁止）
Dog.speak: (Self) -> String = "Bark"  # ❌ エラー
```

### 4. デフォルト値

**中核規則**：フィールドの後ろに直接 `= value` と記述し、コンストラクタの記述を省く。

```yaoxiang
Dog: Type = {
    x: Int = 10,  # デフォルト値
    y: Int = 20,  # デフォルト値
    Animal,
}
```

**コンパイラが生成するコンストラクタ**：

```yaoxiang
# 全フィールドにデフォルト値あり → 引数なしコンストラクタを生成
Dog.new: () -> Dog = { x: 10, y: 20 }

# 一部のフィールドにデフォルト値あり → 部分引数コンストラクタを生成
Dog.new: (x: Int) -> Dog = { x: x, y: 20 }
Dog.new: (y: Int) -> Dog = { x: 10, y: y }

# 全引数コンストラクタ
Dog.new: (x: Int, y: Int) -> Dog = { x: x, y: y }
```

**外部宣言でのデフォルト値**：

```yaoxiang
Dog: Type = {
    x: Int,
    y: Int,
    Animal,
}

# 外部でデフォルト値を宣言
Dog.x: Int = 10
Dog.y: Int = 20
```

**内部宣言と等価**。

### 5. コンパイラ実装

#### 5.1 インタフェース記述子

```rust
// コンパイラ内部：インタフェース記述子
struct InterfaceDescriptor {
    name: String,
    methods: Vec<MethodSignature>,
}
```

#### 5.2 型定義

```rust
// コンパイラ内部：型定義
struct TypeDefinition {
    name: String,
    fields: Vec<Field>,
    interface_implementations: Vec<InterfaceImplementation>,
}

// インタフェース実装（出典情報を保持）
struct InterfaceImplementation {
    interface: InterfaceId,
    methods: HashMap<MethodId, FunctionBody>,
}
```

#### 5.3 実装証明

```rust
// コンパイラ内部：実装証明
struct ImplementationProof {
    type_id: TypeId,
    interface_id: InterfaceId,
    methods: Vec<MethodPointer>,
}
```

#### 5.4 コンパイルフロー

```
1. 型定義を解析し、インタフェース宣言を収集
2. 全メソッド定義を収集（内部と外部）
3. シグネチャでグループ化（オーバーロード）
4. オーバーライドをチェック（エラー）
5. インタフェース完全性をチェック
6. 実装証明を生成
7. ランタイムで値が実装証明を保持
```

### 6. 動的ディスパッチ

**中核設計**：コンパイル時型収集 + インタフェースマッチング、仮想テーブルなし。

#### 6.1 異種コンテナ

```yaoxiang
# インタフェース定義
Animal: Type = {
    speak: (Self) -> String,
}

# 型定義
Dog: Type = {
    x: Int,
    Animal,
    speak: (Self) -> String = "Woof",
}

Cat: Type = {
    y: Int,
    Animal,
    speak: (Self) -> String = "Meow",
}

# 異種コンテナ
animals: List(Animal) = [Dog.new(), Cat.new()]
animals[0].speak()  # "Woof"
animals[1].speak()  # "Meow"
```

#### 6.2 コンパイル時型収集

**中核戦略：所有権追跡とインクリメンタル構築。** コンパイル時にインタフェースを実装する全型を走査するのではなく、各 `List(Animal)` の**所有権操作点**でインクリメンタルに収集する：

```yaoxiang
// 構築点
animals: List(Animal) = [Dog.new()]       // AnimalGroup = { Dog(Dog) }

// append 点
animals.append(Cat.new())                  // append で Cat を検出 → { Dog, Cat } に拡張
animals.append(Bird.new())                 // さらに { Dog, Cat, Bird } に拡張
```

**コンパイラ処理（インクリメンタル）**：

1. `List(I)` が初めて構築される → 初期enumを生成（現在のコンパイルユニット内で既知の全構築型）
2. `append` / `push` / インデックス代入のたびに → 値型がenumに含まれているかチェック；未含有ならenum変種を拡張
3. 最終的なenumに対して単態化された `match` ディスパッチコードを生成
4. コンパイルユニットをまたぐ場合：リンク時に各ユニットのenum変種集合をマージ

**自動生成されるenum**：

```yaoxiang
# コンパイラが自動生成（ユーザは意識しない）
AnimalGroup: Type = {
    Dog(Dog),
    Cat(Cat),
    Bird(Bird),    # ← append(Bird.new()) がインクリメンタル拡張をトリガ
}

# List(Animal) は内部的に List(AnimalGroup) と等価
```

#### 6.3 インタフェースマッチング検査

**重要な洞察**：インタフェースマッチングはコンパイル時検査であり、型が動的にロードされるプラグイン由来であっても同様である。

```yaoxiang
# プラグインシステム
plugin = load_plugin("bird.so")

# コンパイラが検査：plugin.create_bird() の戻り型は Animal を実装していなければならない
bird: Animal = plugin.create_bird()  # コンパイル時検査

# 異種コンテナに格納 —— append 点で enum 拡張をトリガ
animals: List(Animal) = [Dog.new(), Cat.new()]
animals.append(bird)                 # コンパイラ：(1) bird が Animal を実装することを検証 (2) enum を拡張
```

**コンパイラ処理**：
1. `append` 引数の戻り型を検査
2. その型が対象インタフェースを実装していることを検証
3. 合格 → enum を拡張、格納を許可
4. 失敗 → コンパイルエラー

#### 6.4 ランタイムディスパッチ

**呼び出しフロー（コンパイル時enum match、実装証明は消去済み）：**

```
animals[0].speak()
  ↓
コンパイラ生成の match:
  match animals[0] {
    AnimalGroup.Dog(d) => d.speak(),
    AnimalGroup.Cat(c) => c.speak(),
    AnimalGroup.Bird(b) => b.speak(),
  }
```

**仮想テーブルとの比較**：

| | 仮想テーブル（Rust） | コンパイル時enum（YaoXiang） |
|---|---|---|
| 検索方式 | 仮想テーブルポインタ → メソッドポインタ | enum match → 直接呼び出し |
| ランタイムオーバーヘッド | 1 回の間接アドレス参照 | 文字列比較/分岐（CPU 分岐予測で最適化可能） |
| コンパイル時生成 | 仮想テーブル | enum + match |
| ユーザ注釈 | `dyn Trait + 'a` が必要 | 不要 |
| 実装証明 | 該当なし | コンパイル時消去、ランタイムに存在しない |

**YaoXiang の優位性**：
- ブランド注釈が不要
- コンパイル時型安全
- ユーザ透過的（`dyn Animal` と記述する必要がない）
- 実装証明は純粋にコンパイル時の概念であり、ランタイムオーバーヘッドゼロ

#### 6.5 制限と範囲

**当期（単一コンパイルユニット）：** 完全サポート。所有権追跡がすべての `append`/構築点をカバーし、enum をインクリメンタルに構築。

**コンパイルユニットをまたぐ場合：** リンク時に各ユニットのenum変種集合をマージ。リンク時単態化と同じ機構を共有（各ユニットが部分enumを生成、リンカがマージ）。

**非対応：** ランタイム動的型（完全なダックタイピング）。型集合はコンパイル時に完全に既知でなければならない。
---

## ユースケース分析

### 基本的なインタフェース実装

```yaoxiang
# インタフェース定義
Animal: Type = {
    speak: (Self) -> String,
}

# 型定義
Dog: Type = {
    x: Int = 10,
    Animal,
    speak: (Self) -> String = "Woof",
}

# 使用
dog = Dog.new()
dog.speak()  # "Woof"
```

### 複数インタフェース実装

```yaoxiang
# 複数インタフェース
Animal: Type = {
    speak: (Self) -> String,
}

Pet: Type = {
    name: (Self) -> String,
}

# 型が複数のインタフェースを実装
Dog: Type = {
    x: Int = 10,
    Animal,
    Pet,
    speak: (Self) -> String = "Woof",
    name: (Self) -> String = "Buddy",
}

# 使用
dog = Dog.new()
dog.speak()  # "Woof"
dog.name()   # "Buddy"
```

### ジェネリックインタフェース

```yaoxiang
# ジェネリックインタフェース
Container: (T: Type) -> Type = {
    add: (self: &mut Self, item: T) -> Void,
    get: (self: &Self, index: Int) -> T,
}

# ジェネリックインタフェースを実装
IntList: Type = {
    data: Array(Int),
    Container(Int),
    add: (self: &mut Self, item: Int) -> Void = ...,
    get: (self: &Self, index: Int) -> Int = ...,
}
```

### 異種コンテナ

```yaoxiang
# インタフェース定義
Animal: Type = {
    speak: (Self) -> String,
}

# 型定義
Dog: Type = {
    x: Int,
    Animal,
    speak: (Self) -> String = "Woof",
}

Cat: Type = {
    y: Int,
    Animal,
    speak: (Self) -> String = "Meow",
}

# 異種コンテナ
animals: List(Animal) = [Dog.new(), Cat.new()]

# 使用
for animal in animals {
    print(animal.speak())
}
# 出力：
# Woof
# Meow
```

### プラグインシステム

```yaoxiang
# インタフェース定義
Plugin: Type = {
    name: (Self) -> String,
    execute: (Self) -> Void,
}

# メインプログラム
main: () -> Void = {
    # プラグインをロード
    plugin1 = load_plugin("plugin1.so")
    plugin2 = load_plugin("plugin2.so")

    # コンパイラが検査：plugin1 と plugin2 は Plugin インタフェースを実装していなければならない
    plugins: List(Plugin) = [plugin1, plugin2]

    # 全プラグインを実行
    for plugin in plugins {
        print(plugin.name())
        plugin.execute()
    }
}
```

---

## トレードオフ

### 利点

1. **簡潔性**：`impl` キーワードが不要
2. **柔軟性**：メソッド実装は内部・外部両方をサポート
3. **統一性**：オーバーロード規則が一貫
4. **利便性**：デフォルト値構文が簡潔
5. **ゼロオーバーヘッド**：仮想テーブルなし、コンパイル時型収集
6. **型安全**：インタフェースマッチングはコンパイル時検査
7. **ユーザ透過的**：`dyn Animal + 'a` と記述する必要がない

### 欠点

1. **制限**：ランタイム動的型（完全なダックタイピング）は非対応
2. **コンパイル時オーバーヘッド**：各インタフェースに対してenum変種とmatchディスパッチコードを生成する必要がある
3. **型集合**：コンパイル時に完全に既知でなければならない（単一コンパイルユニット内）

### 緩和策

1. **プラグインシステム**：コンパイル時のインタフェースマッチング検査を通じてサポート
2. **型集合**：所有権追跡とインクリメンタル構築——各 `append`/構築点で収集し、グローバル走査ではない
3. **コンパイルユニット間**：リンク時にenum変種集合をマージ、リンク時単態化と機構を共有

---

## 代替案

| 案 | 選択しない理由 |
|------|--------------|
| `impl` キーワード | 構文の複雑性が増す |
| 仮想テーブル（`dyn Trait`） | ブランド注釈（`'a`）が必要 |
| 完全なダックタイピング | ランタイムオーバーヘッド、型安全でない |
| enum 包装（手動） | ユーザの負担が大きい |

---

## RFC-009 との関係

**ブランドとインタフェース実装**：
- インタフェース実装は型層にあり、ブランドには関与しない
- ブランドは借用証明層にある（RFC-009a）
- 両者は直交し、相互に影響しない

**動的ディスパッチとブランド**：
- 動的ディスパッチは実装証明を使用し、ブランド注釈は不要
- 実装証明はコンパイル時に生成され、ランタイムのルックアップはゼロ
- `dyn Trait + 'a` の複雑性を回避


## インタフェース継承

インタフェースは他のインタフェースを含むことができる。**新しい構文を導入しない**——型のインタフェース宣言と完全に同じ構文位置を使用する：

```yaoxiang
Animal: Type = {
    speak: (Self) -> String,
}

Pet: Type = {
    Animal,                       # Pet は Animal を継承 — 新キーワードなし
    name: (Self) -> String,
}

# Dog が Pet を実装する際、Animal と Pet の全メソッドを同時に満たす必要がある
Dog: Type = {
    x: Int,
    Pet,
    speak: (Self) -> String = "Woof",  # Animal 由来
    name: (Self) -> String = "Buddy",  # Pet 由来
}
```

**設計原則：** 継承は存在するが、乱用は推奨されない。主な合成方法は複数のインタフェース宣言による（`Dog: Type = { Animal, Pet, ... }`）。ある型は、継承木で表現する必要なく、自身が満たすすべてのインタフェースを直接宣言できる。インタフェース継承は明確な「is-a」階層が存在する場合にのみ使用する。

**コンパイラ処理：** 継承チェーンを展開する。`Pet` を `{ Animal の全メソッド, name: ... }` に展開。`Dog` が `Pet` を宣言する際、コンパイラは `Dog` が `Animal` と `Pet` の全メソッドを同時に満たしていることを検証する。

## デフォルトメソッド実装

インタフェースはメソッドのデフォルト実装を提供できる。実装型は上書きするかデフォルト実装を継承するかを選択できる：

```yaoxiang
fmt: Type = {
    display: (Self) -> String,                      # 必ず実装
    debug: (Self) -> String = Self.display(),       # ✅ 同じインタフェースのメソッドを参照
    summary: (Self) -> String = f"<{Self.name}>",   # ❌ コンパイルエラー：Self.name は fmt に存在しない
}
```

**中核制約：インタフェースは上位実装を仮定できない。** デフォルトメソッドは同じインタフェース内で既に宣言されているメソッドのみを参照できる。具体型のフィールドや他のインタフェースのメソッドはデフォルトメソッドからは見えない——インタフェースは閉じた契約であり、実装型のポケットに手を突っ込んではならない。この制約違反は**インタフェース定義時**に直接エラーになる。

**継承は下位実装を仮定できる：** インタフェース `Pet` が `Animal` を継承するとき、`Pet` のデフォルトメソッドは `Animal` で宣言されたメソッドを使用できる——継承しているため、それらが存在することが保証される。

```yaoxiang
Animal: Type = {
    speak: (Self) -> String,
}

Pet: Type = {
    Animal,                                              # 継承
    name: (Self) -> String,
    introduce: (Self) -> String = Self.name() + " says " + Self.speak(),  # ✅ speak は継承した Animal 由来
}
```

**コンパイル時の動作：** 型がインタフェースを実装する際、各メソッドについて：
1. 型が提供 → 型のメソッドを使用
2. 型が未提供、インタフェースにデフォルトあり → コンパイラがデフォルト実装を型にインライン化（仮想テーブルオーバーヘッドゼロ）
3. 型が未提供、インタフェースにデフォルトなし → コンパイルエラー

**設計原則：** デフォルトメソッドは `Copy`/`Clone` の自動派生メカニズムに類似——コンパイラが必要に応じて自動生成し、ユーザは上書き可能。`virtual`/`override`/`super` キーワードを導入しない。
---

## 実装フェーズ

| フェーズ | 内容 | 依存 |
|------|------|------|
| Phase 1 | インタフェース宣言構文 | RFC-011 |
| Phase 2 | メソッド実装の内部/外部宣言 | Phase 1 |
| Phase 3 | オーバーロードとオーバーライド規則 | Phase 2 |
| Phase 4 | デフォルト値構文 | Phase 2 |
| Phase 5 | インタフェース継承 | Phase 3 |
| Phase 6 | デフォルトメソッド実装 | Phase 5 |
| Phase 7 | 実装証明生成 | Phase 6 |
| Phase 8 | コンパイル時型収集 | Phase 7 |
| Phase 9 | 動的ディスパッチ実装 | Phase 8 |

---

## 設計決定記録

| 決定 | 結果 | 理由 | 日付 |
|------|------|------|------|
| インタフェース宣言構文 | 型体内でインタフェース名を直接記述 | `impl` キーワードを除去、インタフェース宣言は型定義の自然な構成要素 | 2026-06-14 |
| 動的ディスパッチ | コンパイル時型収集 + 自動enum生成 | 仮想テーブルなし、ランタイムルックアップゼロ、ユーザ透過的 | 2026-06-14 |
| 外部メソッド宣言 | サポート | 内部宣言と等価の柔軟性、コンパイラがファイル横断収集を担当 | 2026-06-14 |
| オーバーライド | 禁止（同シグネチャでエラー） | オーバーライドは予測不能な動作を引き起こす、オーバーロードが全ケースをカバー | 2026-06-14 |
| インタフェース継承 | サポート、新構文なし | 型のインタフェース宣言と同じ構文位置。合成（複数インタフェース宣言）を推奨、深い継承木は非推奨 | 2026-07-03 |
| デフォルトメソッド実装 | サポート、Copy/Clone 自動派生に類似 | インタフェースが本体を提供、コンパイラが必要に応じて実装型にインライン化；ユーザは上書き可能。virtual/override は導入しない | 2026-07-03 |
| デフォルトメソッド制約 | インタフェース定義時に検証：同インタフェースのメソッドのみ参照可、上位実装を仮定不可 | インタフェースは閉じた契約。継承は下位実装を仮定できるが、インタフェースは実装型のフィールド/メソッドを仮定できない | 2026-07-03 |
| 型収集戦略 | 所有権追跡とインクリメンタル構築——各 append/構築点で収集 | 全実装者のグローバル走査ではなく、所有権操作点ごとのenumインクリメンタル拡張 | 2026-07-03 |
| 実装証明 | 純粋にコンパイル時の概念、ランタイムで消去 | ランタイムはenum matchディスパッチで動作、証明はコンパイル時検証のみに使用 | 2026-07-03 |
| コンパイルユニット間 | リンク時に各ユニットのenum変種をマージ | リンク時単態化と機構を共有、各ユニットが部分enumを生成、リンカがマージ | 2026-07-03 |

## オープンな問題

- [x] ~~インタフェース継承（インタフェースは他のインタフェースを継承可能）~~ → サポート、新構文なし。`Pet: Type = { Animal, ... }`
- [x] ~~デフォルトメソッド実装（インタフェースはデフォルト実装を提供可能）~~ → サポート、Copy 自動派生に類似。インタフェースが本体を提供、コンパイラが必要に応じてインライン化
- [ ] インタフェース制約の高度な使用法（関連型、GAT）
- [ ] クロージャとの相互作用（クロージャがインタフェースを実装）

---

## 参考文献

- [RFC-011: ジェネリクスシステム設計](../accepted/011-generic-type-system.md) — 親 RFC
- [RFC-009: 所有権モデル設計](../accepted/009-ownership-model.md) — 所有権システム
- [RFC-009a: 借用証明パイプライン](../accepted/009a-borrow-proof-pipeline.md) — ブランド機構
- [RFC-010: 統一型構文](../accepted/010-unified-type-syntax.md) — 統一構文

---

## ライフサイクルと帰結

| 状態 | 位置 | 説明 |
|------|------|------|
| **審査中** | `docs/design/rfc/review/` | コミュニティでの議論を公開中 |