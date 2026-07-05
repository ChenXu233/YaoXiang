```markdown
---
title: "RFC-011a: インターフェース実装と動的ディスパッチ"
status: "レビュー中"
author: "晨煦"
created: "2026-06-14"
updated: "2026-07-05"
group: "rfc-011"
---

# RFC-011a: インターフェース実装と動的ディスパッチ

> **親 RFC**: [RFC-011: ジェネリクスシステム設計](../accepted/011-generic-type-system.md)
>
> **本 RFC は RFC-011 §2.1-2.4 のインターフェース制約部分を補完し、置換する。**

## 概要

RFC-011 はジェネリクスシステムを定義しているが、インターフェース実装メカニズムを詳細には規定していない。本ドキュメントは以下を補完する：

1. **インターフェース宣言**：型定義内で `impl` キーワードを使わずインターフェース名を直接記述
2. **メソッド実装**：内部宣言と外部宣言の両方をサポート
3. **オーバーロードルール**：シグネチャが異なる場合はオーバーロードを許可、同じ場合はエラー（オーバーライド禁止）
4. **デフォルト値**：フィールドの後に直接 `= value` と記述
5. **動的ディスパッチ**：コンパイル時の型収集 + インターフェースマッチング、vtable なし

**核となる設計**：

```yaoxiang
# インターフェース定義
Animal: Type = {
    speak: (Self) -> String,
}

# 型定義（内部宣言）
Dog: Type = {
    x: Int = 10,
    Animal,  # インターフェース宣言
    speak: (Self) -> String = "Woof",
}

# 外部宣言（オーバーロード）
Dog.speak: (Self, volume: Int) -> String = "WOOF"

# 異種コンテナ（動的ディスパッチ）
animals: List(Animal) = [Dog.new(), Cat.new()]
animals[0].speak()  # "Woof"
```

**取り除かれた複雑性**：
- ❌ `impl` キーワードなし
- ❌ `dyn Trait + 'a` 注釈なし
- ❌ vtable なし（コンパイル時型収集 + enum ラッピング）
- ❌ オーバーライドなし（オーバーロードルールで統一）

---

## 動機

### RFC-011 の不足

RFC-011 はジェネリクスシステムを定義しているが、以下を詳細に規定していない：

| 問題 | 説明 |
|------|------|
| インターフェース宣言構文 | 型がインターフェースを実装していることをどう宣言するか？ |
| メソッド実装の位置 | 内部宣言か外部宣言か？ |
| オーバーロードルール | 同名メソッドをどう処理するか？ |
| デフォルト値構文 | フィールドにデフォルト値を設定するには？ |
| 動的ディスパッチ | 異種コンテナをどう実現するか？ |

### 設計目標

1. **簡潔**：`impl` キーワードが不要
2. **柔軟**：メソッド実装は内部・外部の両方をサポート
3. **統一**：オーバーロードルールが一貫
4. **便利**：デフォルト値構文が簡潔
5. **ゼロコスト**：vtable なし、コンパイル時型収集

### Rust との比較

| 特性 | Rust | YaoXiang |
|------|------|----------|
| インターフェース宣言 | `impl Animal for Dog { ... }` | `Dog: Type = { Animal, ... }` |
| メソッド実装 | `impl` ブロック内 | 内部または外部 |
| オーバーロード | サポートなし | サポート（シグネチャが異なる場合） |
| デフォルト値 | `#[default]` が必要 | 直接 `= value` と記述 |
| 異種コンテナ | `Vec<Box<dyn Animal + 'a>>` | `List(Animal)` |
| 動的ディスパッチ | vtable ルックアップ | コンパイル時型収集 |

---

## 提案

### 1. インターフェース宣言

**中核ルール**：型定義内でインターフェース名を直接記述し、`impl` キーワードは不要。

```yaoxiang
# インターフェース定義
Animal: Type = {
    speak: (Self) -> String,
}

# 型がインターフェースを実装することを宣言
Dog: Type = {
    x: Int,
    Animal,  # インターフェース宣言
}
```

**コンパイラ処理**：
1. `Animal` がインターフェース型であることを識別
2. `Dog` が `Animal` の要求するすべてのメソッドを持つかチェック
3. 通過 → 実装証明を生成
4. 失敗 → コンパイルエラー

**糖衣構文の等価性**：

```yaoxiang
Dog: Type = {
    x: Int,
    Animal,  # Animal のメソッドを展開するのと同じだが、出典マークを保持
}

# 等価（ただし出典情報を保持）
Dog: Type = {
    x: Int,
    speak: (Self) -> String,  # Animal 由来
}
```

**出典マークが必要な理由**：
- 直接展開では出典情報が失われる
- 出典マークは実装証明の生成に使用される
- 実行時は証明を通じて正しいメソッドを見つける

### 2. メソッド実装

**中核ルール**：メソッド実装は内部宣言と外部宣言の両方をサポート。

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
1. すべての定義（内部と外部）を収集
2. シグネチャでグループ化（オーバーロード）
3. オーバーライドがないかチェック（エラー）
4. インターフェースの完全性をチェック
5. 実装証明を生成

### 3. オーバーロードとオーバーライド

**中核ルール**：
- シグネチャが異なる → オーバーロード → 許可
- シグネチャが同じ → オーバーライド → エラー

#### 3.1 オーバーロード（許可）

```yaoxiang
# 引数の型が異なる場合、オーバーロードを許可
Dog.speak: (Self) -> String = "Woof"
Dog.speak: (Self, volume: Int) -> String = "WOOF"
```

#### 3.2 オーバーライド（禁止）

```yaoxiang
# シグネチャが完全に同じ場合、オーバーライドを禁止
Dog.speak: (Self) -> String = "Woof"
Dog.speak: (Self) -> String = "Bark"  # ❌ エラー：オーバーライドは許可されない
```

**エラーメッセージ**：

```
エラー：Dog.speak(Self) -> String が重複定義されています
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

#### 3.3 ルールの統一

**内部宣言と外部宣言は同じオーバーロード／オーバーライドルールに従う**：

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

**中核ルール**：フィールドの後に直接 `= value` と記述し、コンストラクタを省略。

```yaoxiang
Dog: Type = {
    x: Int = 10,  # デフォルト値
    y: Int = 20,  # デフォルト値
    Animal,
}
```

**コンパイラが生成するコンストラクタ**：

```yaoxiang
# すべてのフィールドにデフォルト値あり → 引数なしコンストラクタを生成
Dog.new: () -> Dog = { x: 10, y: 20 }

# 一部のフィールドにデフォルト値あり → 一部引数コンストラクタを生成
Dog.new: (x: Int) -> Dog = { x: x, y: 20 }
Dog.new: (y: Int) -> Dog = { x: 10, y: y }

# 全引数コンストラクタ
Dog.new: (x: Int, y: Int) -> Dog = { x: x, y: y }
```

**外部宣言によるデフォルト値**：

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

#### 5.1 インターフェースディスクリプタ

```rust
// コンパイラ内部：インターフェースディスクリプタ
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

// インターフェース実装（出典情報を保持）
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
1. 型定義を解析し、インターフェース宣言を収集
2. すべてのメソッド定義（内部と外部）を収集
3. シグネチャでグループ化（オーバーロード）
4. オーバーライドをチェック（エラー）
5. インターフェースの完全性をチェック
6. 実装証明を生成
7. 実行時、値は実装証明を保持
```

### 6. 動的ディスパッチ

**核となる設計**：コンパイル時型収集 + インターフェースマッチング、vtable なし。

#### 6.1 異種コンテナ

```yaoxiang
# インターフェース定義
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

**中核戦略：所有権追跡によるインクリメンタル構築。** コンパイル時にインターフェースを実装するすべての型を走査するのではない——各 `List(Animal)` の**所有権操作点**でインクリメンタルに収集する：

```yaoxiang
// 構築点
animals: List(Animal) = [Dog.new()]       // AnimalGroup = { Dog(Dog) }

// append 点
animals.append(Cat.new())                  // コンパイラは append 箇所で Cat を検出 → { Dog, Cat } に拡張
animals.append(Bird.new())                 // さらに拡張 { Dog, Cat, Bird }
```

**コンパイラ処理**（インクリメンタル）：

1. `List(I)` が初めて構築される箇所に遭遇 → 初期 enum を生成（現在のコンパイルユニット内で既知のすべての構築型）
2. 各 `append` / `push` / インデックス代入 → 値の型が enum に既出かチェック；未登録なら enum 変種を拡張
3. 最終的な enum に対して単態化された `match` ディスパッチコードを生成
4. コンパイルユニット間：リンク時に各ユニットの enum 変種集合をマージ

**自動生成される enum**：

```yaoxiang
# コンパイラが自動生成（ユーザは意識しない）
AnimalGroup: Type = {
    Dog(Dog),
    Cat(Cat),
    Bird(Bird),    # ← append(Bird.new()) がインクリメンタル拡張を引き起こす
}

# List(Animal) は内部的に List(AnimalGroup) と等価
```

#### 6.3 インターフェースマッチングチェック

**重要な洞察**：型が動的にロードされるプラグイン由来であっても、インターフェースマッチングはコンパイル時チェックである。

```yaoxiang
# プラグインシステム
plugin = load_plugin("bird.so")

# コンパイラチェック：plugin.create_bird() の戻り型は Animal を実装しなければならない
bird: Animal = plugin.create_bird()  # コンパイル時チェック

# 異種コンテナに格納 —— append 点で enum 拡張を引き起こす
animals: List(Animal) = [Dog.new(), Cat.new()]
animals.append(bird)                 # コンパイラ：(1) bird が Animal を実装していることを検証 (2) enum を拡張
```

**コンパイラ処理**：
1. `append` の引数の戻り型をチェック
2. その型が対象インターフェースを実装していることを検証
3. 通過 → enum を拡張、格納を許可
4. 失敗 → コンパイルエラー

#### 6.4 実行時ディスパッチ

**呼び出しフロー（コンパイル時 enum match、ImplementationProof は既に消去済み）：**

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

**vtable との比較**：

| | vtable（Rust） | コンパイル時 enum（YaoXiang） |
|---|---|---|
| 検索方式 | vtable ポインタ → メソッドポインタ | enum match → 直接呼び出し |
| 実行時オーバーヘッド | 1 回の間接参照 | 文字列比較/branch（CPU 分岐予測で最適化可能） |
| コンパイル時生成 | vtable | enum + match |
| ユーザ注釈 | `dyn Trait + 'a` が必要 | 不要 |
| ImplementationProof | 該当なし | コンパイル時に消去、実行時には存在しない |

**YaoXiang の優位性**：
- ブランド注釈が不要
- コンパイル時型安全
- ユーザ透過的（`dyn Animal` を書く必要なし）
- ImplementationProof は純粋にコンパイル時の概念であり、実行時オーバーヘッドゼロ

#### 6.5 制限とスコープ

**当期（単一コンパイルユニット）：** 完全サポート。所有権追跡がすべての `append`/構築点をカバーし、enum がインクリメンタルに構築される。

**コンパイルユニット間：** リンク時に各ユニットの enum 変種集合をマージ。リンク時単態化と同じ機構を共有（各ユニットが部分 enum を生成、リンカがマージ）。

**サポート対象外：** 実行時動的型付け（完全なダックタイピング）。型集合はコンパイル時に完全に既知でなければならない。
---

## ユースケース分析

### 基本的なインターフェース実装

```yaoxiang
# インターフェース定義
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

### 多重インターフェース実装

```yaoxiang
# 複数のインターフェース
Animal: Type = {
    speak: (Self) -> String,
}

Pet: Type = {
    name: (Self) -> String,
}

# 型が複数のインターフェースを実装
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

### ジェネリクスインターフェース

```yaoxiang
# ジェネリクスインターフェース
Container: (T: Type) -> Type = {
    add: (self: &mut Self, item: T) -> Void,
    get: (self: &Self, index: Int) -> T,
}

# ジェネリクスインターフェースを実装
IntList: Type = {
    data: Array(Int),
    Container(Int),
    add: (self: &mut Self, item: Int) -> Void = ...,
    get: (self: &Self, index: Int) -> Int = ...,
}
```

### 異種コンテナ

```yaoxiang
# インターフェース定義
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
# インターフェース定義
Plugin: Type = {
    name: (Self) -> String,
    execute: (Self) -> Void,
}

# メインプログラム
main: () -> Void = {
    # プラグインをロード
    plugin1 = load_plugin("plugin1.so")
    plugin2 = load_plugin("plugin2.so")

    # コンパイラチェック：plugin1 と plugin2 は Plugin インターフェースを実装しなければならない
    plugins: List(Plugin) = [plugin1, plugin2]

    # すべてのプラグインを実行
    for plugin in plugins {
        print(plugin.name())
        plugin.execute()
    }
}
```

---

## トレードオフ

### 利点

1. **簡潔**：`impl` キーワードが不要
2. **柔軟**：メソッド実装は内部・外部の両方をサポート
3. **統一**：オーバーロードルールが一貫
4. **便利**：デフォルト値構文が簡潔
5. **ゼロコスト**：vtable なし、コンパイル時型収集
6. **型安全**：インターフェースマッチングはコンパイル時チェック
7. **ユーザ透過的**：`dyn Animal + 'a` を書く必要なし

### 欠点

1. **制限**：実行時動的型付け（完全なダックタイピング）をサポートしない
2. **コンパイル時オーバーヘッド**：各インターフェースに対して enum 変種と match ディスパッチコードを生成する必要がある
3. **型集合**：コンパイル時に完全に既知でなければならない（単一コンパイルユニット内）

### 緩和策

1. **プラグインシステム**：コンパイル時のインターフェースマッチングチェックによりサポート
2. **型集合**：所有権追跡によるインクリメンタル構築——各 `append`/構築点で収集、グローバル走査ではない
3. **コンパイルユニット間**：リンク時に enum 変種集合をマージ、リンク時単態化と同じ機構を共有

---

## 代替案

| 案 | 選択しない理由 |
|------|--------------|
| `impl` キーワード | 構文の複雑性が増す |
| vtable（`dyn Trait`） | ブランド注釈（`'a`）が必要 |
| 完全なダックタイピング | 実行時オーバーヘッド、型安全でない |
| enum ラッピング（手動） | ユーザの負担が大きい |

---

## RFC-009 との関係

**ブランドとインターフェース実装**：
- インターフェース実装は型レイヤにあり、ブランドには関与しない
- ブランドは借用証明レイヤ（RFC-009a）にある
- 両者は直交しており、互いに影響しない

**動的ディスパッチとブランド**：
- 動的ディスパッチは実装証明を使用し、ブランド注釈は不要
- 実装証明はコンパイル時に生成され、実行時ルックアップはゼロ
- `dyn Trait + 'a` の複雑性を回避


## インターフェース継承

インターフェースは別のインターフェースを含むことができる。**新しい構文を導入しない**——インターフェースを含む構文位置は、型がインターフェースを宣言する位置と完全に同じである：

```yaoxiang
Animal: Type = {
    speak: (Self) -> String,
}

Pet: Type = {
    Animal,                       # Pet は Animal を継承 — 新しいキーワードなし
    name: (Self) -> String,
}

# Dog が Pet を実装する際、Animal と Pet の両方のすべてのメソッドを満たす必要がある
Dog: Type = {
    x: Int,
    Pet,
    speak: (Self) -> String = "Woof",  # Animal 由来
    name: (Self) -> String = "Buddy",  # Pet 由来
}
```

**設計原則：** 継承は存在するが、乱用は推奨されない。主な合成手段は複数のインターフェース宣言（`Dog: Type = { Animal, Pet, ... }`）によるものである。型は継承木を通じて表現する必要なく、実装するすべてのインターフェースを直接宣言できる。インターフェース継承は明確な "is-a" 階層がある場合にのみ使用する。

**コンパイラ処理：** 継承連鎖を展開する。`Pet` は `{ Animal のすべてのメソッド, name: ... }` に展開される。`Dog` が `Pet` を宣言する際、コンパイラは `Dog` が `Animal` と `Pet` のすべてのメソッドを同時に満たしていることを検証する。

## デフォルトメソッド実装

インターフェースはメソッドのデフォルト実装を提供できる。実装する型は上書きするか、デフォルト実装を継承するかを選択できる：

```yaoxiang
fmt: Type = {
    display: (Self) -> String,                      # 実装必須
    debug: (Self) -> String = Self.display(),       # ✅ 同じインターフェースのメソッドを参照
    summary: (Self) -> String = f"<{Self.name}>",   # ❌ コンパイルエラー：Self.name は fmt に存在しない
}
```

**中核制約：インターフェースは上位の実装を仮定できない。** デフォルトメソッドは同じインターフェース内で既に宣言されたメソッドのみを参照できる。具体的な型のフィールドや他のインターフェースのメソッドはデフォルトメソッドからは不可視である——インターフェースは閉じた契約であり、実装型のポケットに手を伸ばすことはできない。この制約に違反した場合、**インターフェース定義時**に直接エラーとなる。

**継承は下位の実装を仮定できる：** インターフェース `Pet` が `Animal` を継承するとき、`Pet` のデフォルトメソッドは `Animal` が宣言したメソッドを使用できる——継承しているので保証されている。

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

**コンパイル時挙動：** 型がインターフェースを実装する際、各メソッドについて：
1. 型が提供している → 型のメソッドを使用
2. 型が提供していないが、インターフェースにデフォルトあり → デフォルト実装をコンパイラが型にインライン化（vtable オーバーヘッドゼロ）
3. 型が提供しておらず、インターフェースにもデフォルトなし → コンパイルエラー

**設計原則：** デフォルトメソッドは `Copy`/`Clone` の自動導出メカニズムに類似している——コンパイラが必要に応じて自動生成し、ユーザは上書き可能。`virtual`/`override`/`super` キーワードは導入しない。
---

## 実装フェーズ

| フェーズ | 内容 | 依存 |
|------|------|------|
| Phase 1 | インターフェース宣言構文 | RFC-011 |
| Phase 2 | メソッド実装の内部/外部宣言 | Phase 1 |
| Phase 3 | オーバーロードとオーバーライドルール | Phase 2 |
| Phase 4 | デフォルト値構文 | Phase 2 |
| Phase 5 | インターフェース継承 | Phase 3 |
| Phase 6 | デフォルトメソッド実装 | Phase 5 |
| Phase 7 | 実装証明生成 | Phase 6 |
| Phase 8 | コンパイル時型収集 | Phase 7 |
| Phase 9 | 動的ディスパッチ実装 | Phase 8 |

---

## 設計決定記録

| 決定 | 結論 | 理由 | 日付 |
|------|------|------|------|
| インターフェース宣言構文 | 型体内でインターフェース名を直接記述 | `impl` キーワードを排除、インターフェース宣言は型定義の自然な構成要素 | 2026-06-14 |
| 動的ディスパッチ | コンパイル時型収集 + 自動 enum 生成 | vtable なし、実行時ルックアップゼロ、ユーザ透過的 | 2026-06-14 |
| 外部メソッド宣言 | サポート | 内部宣言と同等の柔軟性、コンパイラがファイル横断収集を担当 | 2026-06-14 |
| オーバーライド | 禁止（同シグネチャはエラー） | オーバーライドは予測不能な挙動を引き起こす、オーバーロードが全ケースをカバー | 2026-06-14 |
| インターフェース継承 | サポート、新しい構文なし | インターフェースを宣言する構文位置と同じ。多重宣言による合成を推奨、深い継承木は推奨しない | 2026-07-03 |
| デフォルトメソッド実装 | サポート、`Copy`/`Clone` 自動導出に類似 | インターフェースが本体を提供、コンパイラが実装型にインライン化；ユーザは上書き可能。`virtual`/`override` は導入しない | 2026-07-03 |
| デフォルトメソッド制約 | インターフェース定義時に検証：同じインターフェースのメソッドのみ参照可能、上位の実装は仮定不可 | インターフェースは閉じた契約である。継承は下位の実装を仮定できるが、インターフェースは実装型のフィールド/メソッドを仮定できない | 2026-07-03 |
| 型収集戦略 | 所有権追跡によるインクリメンタル構築——各 append/構築点で収集 | すべての実装者をグローバル走査するのでは、所有権操作点ごとにインクリメンタルに enum を拡張する | 2026-07-03 |
| ImplementationProof | 純粋にコンパイル時の概念、実行時に消去 | 実行時は enum match ディスパッチで動作、証明はコンパイル時検証のみに使用 | 2026-07-03 |
| コンパイルユニット間 | リンク時に各ユニットの enum 変種をマージ | リンク時単態化と同じ機構を共有、各ユニットが部分 enum を生成、リンカがマージ | 2026-07-03 |

## 未解決問題

- [x] ~~インターフェース継承（インターフェースが他のインターフェースを継承できる）~~ → サポート、新しい構文なし。`Pet: Type = { Animal, ... }`
- [x] ~~デフォルトメソッド実装（インターフェースがデフォルト実装を提供できる）~~ → サポート、`Copy` 自動導出に類似。インターフェースが本体を提供、コンパイラが必要に応じてインライン化
- [ ] インターフェース制約の高度な使用方法（関連型、GAT）
- [ ] クロージャとの相互作用（クロージャがインターフェースを実装）

---

## 参考文献

- [RFC-011: ジェネリクスシステム設計](../accepted/011-generic-type-system.md) — 親 RFC
- [RFC-009: 所有権モデル設計](../accepted/009-ownership-model.md) — 所有権システム
- [RFC-009a: 借用証明パイプライン](../accepted/009a-borrow-proof-pipeline.md) — ブランドメカニズム
- [RFC-010: 統一型構文](../accepted/010-unified-type-syntax.md) — 統一構文

---

## ライフサイクルと帰趣

| 状態 | 位置 | 説明 |
|------|------|------|
| **レビュー中** | `docs/design/rfc/review/` | オープンなコミュニティ議論 |
```