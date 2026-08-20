---
title: 'RFC-011a: インターフェース実装と動的ディスパッチ'
status: '審査中'
author: '晨煦'
created: '2026-06-14'
updated: '2026-08-19'
group: 'rfc-011'
---

# RFC-011a: インターフェース実装と動的ディスパッチ

> **親 RFC**: [RFC-011: ジェネリック型システム設計](../accepted/011-generic-type-system.md)
>
> **本 RFC は RFC-011 §2.1-2.4 のインターフェース制約部分を補完し、置換する。**

## 要約

RFC-011 はジェネリック型システムを定義したが、インターフェース実装メカニズムを詳述していない。本文書では以下を補足する：

1. **インターフェース宣言**：インターフェースはパラメータ化された型——`(Self: Type) -> Type`
   であり、実装時に具体的な型を渡す
2. **メソッド実装**：内部宣言と外部宣言の両方をサポート
3. **オーバーロード規則**：シグネチャが異なる場合はオーバーロードを許可し、シグネチャが同じ場合はエラー（オーバーライド禁止）
4. **デフォルト値**：フィールドの直後に `= value` と記述
5. **動的ディスパッチ**：コンパイル時の型収集 + インターフェースマッチング、仮想テーブルなし

**コアデザイン**：

```yaoxiang
# インターフェース定義（パラメータ化された型、Self は明示的な型パラメータ）
Animal: (Self: Type) -> Type = {
    speak: (self: Self) -> String,
}

# 型定義（内部宣言）
Dog: Type = {
    x: Int = 10,
    Animal(Dog),  # インターフェースのインスタンス化、Self ↦ Dog
    speak: (self: Dog) -> String = "Woof",
}

# 外部宣言（オーバーロード）
Dog.speak: (self: Dog, volume: Int) -> String = "WOOF"

# 異種コンテナ（動的ディスパッチ）
animals: List(Animal) = [Dog.new(), Cat.new()]
animals[0].speak()  # "Woof"
```

**取り除かれた複雑さ**：

- ❌ `impl` キーワードなし
- ❌ `Self` マジックキーワードなし（`Self` は明示的な型パラメータであり、`T` と変わらない）
- ❌ `dyn Trait + 'a` 注釈なし
- ❌ 仮想テーブルなし（コンパイル時の型収集 + enum ラッピング）
- ❌ オーバーライドなし（オーバーロード規則で統一）

---

## 動機

### RFC-011 の不備

RFC-011 はジェネリック型システムを定義したが、以下を詳述していない：

| 問題                       | 説明                                                   |
| -------------------------- | ------------------------------------------------------ |
| インターフェース宣言の構文 | 型がインターフェースを実装していることをどう宣言するか |
| メソッド実装の位置         | 内部宣言か外部宣言か？                                 |
| オーバーロード規則         | 同名のメソッドをどう処理するか                         |
| デフォルト値の構文         | フィールドのデフォルト値をどう設定するか               |
| 動的ディスパッチ           | 異種コンテナをどう実現するか                           |

### 設計目標

1. **簡潔**：`impl` キーワードが不要
2. **柔軟**：メソッド実装は内部・外部の両方をサポート
3. **統一**：オーバーロード規則が一貫
4. **便利**：デフォルト値の構文が簡潔
5. **ゼロコスト**：仮想テーブルなし、コンパイル時の型収集

### Rust との比較

| 特性                 | Rust                           | YaoXiang                           |
| -------------------- | ------------------------------ | ---------------------------------- |
| インターフェース宣言 | `impl Animal for Dog { ... }`  | `Dog: Type = { Animal(Dog), ... }` |
| メソッド実装         | `impl` ブロック内              | 内部または外部                     |
| オーバーロード       | サポートなし                   | サポート（シグネチャが異なる場合） |
| デフォルト値         | `#[default]` が必要            | `= value` と直接記述               |
| 異種コンテナ         | `Vec<Box<dyn Animal + 'a>>`    | `List(Animal)`                     |
| 動的ディスパッチ     | 仮想テーブルルックアップ       | コンパイル時の型収集               |
| Self キーワード      | マジックキーワード、暗黙の量化 | 明示的な型パラメータ、T と同等     |

---

## 提案

### 1. インターフェース宣言

**中核規則**：インターフェースはパラメータ化された型 `(Self: Type) -> Type` であり、`Self`
は明示的な型パラメータであり、マジックキーワードではない。実装時にはインターフェースを呼び出し、具体的な型を渡す。

```yaoxiang
# インターフェース定義（RFC-011 のジェネリック型と完全に一致）
Animal: (Self: Type) -> Type = {
    speak: (self: Self) -> String,
}

# 型がインターフェースの実装を宣言
Dog: Type = {
    x: Int,
    Animal(Dog),  # インターフェースをインスタンス化、Self ↦ Dog
}
```

**コンパイラの処理**：

1. `Animal(Dog)` が `(Self: Type) -> Type` のインスタンス化呼び出しであることを識別
2. `Self ↦ Dog` の置換を実行：`Animal(Dog)` を `{ speak: (self: Dog) -> String }` に展開
3. `Dog` が必要なすべてのメソッドを提供しているかチェック（シグネチャの一致）
4. 通過 → 実装証明を生成
5. 失敗 → コンパイルエラー

**展開の等価性**：

```yaoxiang
Dog: Type = {
    x: Int,
    Animal(Dog),  # Animal のメソッドに展開、出典マーカーを保持
}

# 以下の等価（出典情報を保持）
Dog: Type = {
    x: Int,
    speak: (self: Dog) -> String,  # Animal 由来、Self は既に Dog に置換済み
}
```

**なぜ出典マーカーが必要か**：

- 直接展開すると出典情報が失われる
- 出典マーカーは実装証明の生成に使用される
- ランタイムでは証明を通じて正しいメソッドを見つける

#### 1.1 Self 型パラメータと型チェックのタイミング

`Self`
はインターフェースの明示的な型パラメータであり、マジックキーワードではない。`Animal: (Self: Type) -> Type`
と `List: (T: Type) -> Type` は同じもの——`(Type) -> Type` 型コンストラクタ。

**型チェックのタイミング**：

- **インターフェース定義時**：`{ speak: (self: Self) -> String }` 内の `Self`
  は抽象的な型パラメータであり、構文チェックのみ行われる。
- **インスタンス化時点**：`Animal(Dog)` の時点で `Self ↦ Dog`
  を実行し、展開後に完全な型チェック（シグネチャの一致、メソッドの存在）を行う。

これにより、RFC-011 における `Self` が暗黙のマジックキーワードであった問題を回避する——`Self`
は型定義には現れず、インターフェースのパラメータリストに一度だけ現れ、`T` と完全に同等である。

#### 1.2 フィールド名とメソッド名の名前空間

型のフィールド名とメソッド名は同じ名前空間を共有する。インターフェース展開後、インターフェースメソッド名と型フィールド名が衝突した場合、**コンパイルエラー**となる：

```yaoxiang
Drawable: (Self: Type) -> Type = {
    x: (self: Self) -> Int,    // メソッド名が x
}

Point: Type = {
    x: Int,                     // フィールド名も x
    Drawable(Point),            // ❌ コンパイルエラー：Drawable はメソッド x を要求するが、フィールド x と衝突
}
```

フィールドアクセス `point.x` とメソッド呼び出し `point.x()`
は構文上区別できない。名前空間を統一することで曖昧さを排除する。

### 2. メソッド実装

**中核規則**：メソッド実装は内部宣言と外部宣言の両方をサポートする。

#### 2.1 内部宣言

```yaoxiang
Dog: Type = {
    x: Int = 10,
    Animal(Dog),
    speak: (self: Dog) -> String = "Woof",  # メソッド実装は内部
}
```

#### 2.2 外部宣言

```yaoxiang
Dog: Type = {
    x: Int,
    Animal(Dog),
}

# メソッド実装は外部
Dog.speak: (self: Dog) -> String = "Woof"
```

#### 2.3 混合宣言

```yaoxiang
Dog: Type = {
    x: Int = 10,
    Animal(Dog),
    speak: (self: Dog) -> String = "Woof",  # 一部のメソッドは内部
}

# 一部のメソッドは外部
Dog.play: (self: Dog) -> Void = { ... }
```

**コンパイラの処理**：

1. すべての定義（内部と外部）を収集
2. シグネチャでグループ化（オーバーロード）
3. オーバーライドがないかチェック（エラー）
4. インターフェースの完全性をチェック
5. 実装証明を生成

### 3. オーバーロードとオーバーライド

**中核規則**：

- シグネチャが異なる → オーバーロード → 許可
- シグネチャが同じ → オーバーライド → エラー

#### 3.1 オーバーロード（許可）

```yaoxiang
# 引数の型が異なるため、オーバーロードが許可される
Dog.speak: (self: Dog) -> String = "Woof"
Dog.speak: (self: Dog, volume: Int) -> String = "WOOF"
```

#### 3.2 オーバーライド（禁止）

```yaoxiang
# シグネチャが完全に同じため、オーバーライドは禁止
Dog.speak: (self: Dog) -> String = "Woof"
Dog.speak: (self: Dog) -> String = "Bark"  # ❌ エラー：オーバーライドは許可されない
```

**エラーメッセージ**：

```
エラー：Dog.speak(self: Dog) -> String の重複定義
  --> ファイル2:5:1
  |
5 | Dog.speak: (self: Dog) -> String = "Bark"
  | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ 重複定義
  |
  --> ファイル1:3:1
  |
3 | Dog.speak: (self: Dog) -> String = "Woof"
  | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ 最初の定義
```

#### 3.3 規則の統一

**内部宣言と外部宣言は同じオーバーロード/オーバーライド規則に従う**：

```yaoxiang
# 内部宣言
Dog: Type = {
    x: Int,
    Animal(Dog),
    speak: (self: Dog) -> String = "Woof",
}

# 外部宣言（オーバーロード、許可）
Dog.speak: (self: Dog, volume: Int) -> String = "WOOF"

# 外部宣言（オーバーライド、禁止）
Dog.speak: (self: Dog) -> String = "Bark"  # ❌ エラー
```

### 4. デフォルト値

**中核規則**：フィールドの直後に `= value` と記述し、コンストラクタ関数を省略する。

```yaoxiang
Dog: Type = {
    x: Int = 10,  # デフォルト値
    y: Int = 20,  # デフォルト値
    Animal(Dog),
}
```

**コンパイラが生成するコンストラクタ**：

```yaoxiang
# すべてのフィールドにデフォルト値がある → 引数なしコンストラクタを生成
Dog.new: () -> Dog = { x: 10, y: 20 }

# 一部のフィールドにデフォルト値がある → 一部引数コンストラクタを生成
Dog.new: (x: Int) -> Dog = { x: x, y: 20 }
Dog.new: (y: Int) -> Dog = { x: 10, y: y }

# 全引数コンストラクタ
Dog.new: (x: Int, y: Int) -> Dog = { x: x, y: y }
```

**外部宣言のデフォルト値**：

```yaoxiang
Dog: Type = {
    x: Int,
    y: Int,
    Animal(Dog),
}

# 外部でデフォルト値を宣言
Dog.x: Int = 10
Dog.y: Int = 20
```

**内部宣言と等価**。

### 5. コンパイラの実装

#### 5.1 インターフェース記述子

```rust
// コンパイラ内部：インターフェース記述子
struct InterfaceDescriptor {
    name: String,
    self_param: TypeParam,     // Self 型パラメータ
    methods: Vec<MethodSignature>,
}
```

#### 5.2 型定義

```rust
// コンパイラ内部：型定義
struct TypeDefinition {
    name: String,
    fields: Vec<Field>,
    interface_instantiations: Vec<InterfaceInstantiation>,
}

// インターフェースインスタンス化（Self ↦ ConcreteType）
struct InterfaceInstantiation {
    interface: InterfaceId,
    self_type: TypeId,          // Self が置換される具体的な型
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
1. 型定義を解析し、インターフェースインスタンス化宣言を収集（Animal(Dog)）
2. 各インターフェースインスタンス化に対して Self ↦ ConcreteType 置換を実行
3. インターフェースメソッドシグネチャを展開し、シグネチャ一致をチェック
4. すべてのメソッド定義（内部と外部）を収集
5. シグネチャでグループ化（オーバーロード）
6. オーバーライドをチェック（エラー）
7. インターフェースの完全性をチェック
8. 実装証明を生成
```

### 6. 動的ディスパッチ

**コアデザイン**：コンパイル時の型収集 + インターフェースマッチング、仮想テーブルなし。

#### 6.1 異種コンテナ

`Animal` は `(Self: Type) -> Type` である。`List(Animal)`
は未インスタンス化のインターフェース型コンストラクタを**存在型**（existential）として使用する：`∃S. Animal(S)`——「ある型 S が存在し、S は Animal(S) を実装する」。

```yaoxiang
# インターフェース定義
Animal: (Self: Type) -> Type = {
    speak: (self: Self) -> String,
}

# 型定義
Dog: Type = {
    x: Int,
    Animal(Dog),
    speak: (self: Dog) -> String = "Woof",
}

Cat: Type = {
    y: Int,
    Animal(Cat),
    speak: (self: Cat) -> String = "Meow",
}

# 異種コンテナ — Animal が未インスタンス化 = 存在型
animals: List(Animal) = [Dog.new(), Cat.new()]
animals[0].speak()  # "Woof"
animals[1].speak()  # "Meow"
```

**所有権セマンティクス**：異種コンテナへの格納は Move セマンティクス（RFC-009）。`Dog.new()` は
`AnimalGroup::Dog` enum variant に move され、元の変数は使用不可となる。

```yaoxiang
dog = Dog.new()
animals: List(Animal) = [dog]
# dog.speak()  ← ❌ コンパイルエラー：dog は既に move されている
```

#### 6.2 コンパイル時の型収集

**中核戦略：所有権追跡によるインクリメンタル構築。**
コンパイル時にインターフェースを実装するすべての型を走査するのではなく、各 `List(Animal)`
の**所有権操作点**でインクリメンタルに収集する：

```yaoxiang
// 構築点
animals: List(Animal) = [Dog.new()]       // AnimalGroup = { Dog(Dog) }

// append 点
animals.append(Cat.new())                  // コンパイラが append 箇所で Cat を検出 → { Dog, Cat } に拡張
animals.append(Bird.new())                 // さらに { Dog, Cat, Bird } に拡張
```

**コンパイラの処理**（インクリメンタル）：

1. `List(Animal)`
   が最初に構築される箇所に遭遇 → 初期 enum を生成（現在のコンパイルユニット内で既知のすべての構築型）
2. `append` / `push`
   / インデックス代入のたびに、値の型がすでに enum に存在するかチェック；存在しなければ enum
   variant を拡張
3. 最終的な enum に対して単態化された `match` ディスパッチコードを生成
4. コンパイルユニット間：LTO（リンク時最適化）に依存して enum variant をマージ。`Animal`
   は存在型としてコンパイルユニットの境界を渡る際、各ユニットは部分的な enum
   variant を生成し、リンク段階で完全な enum にマージされる。

**自動生成される enum**：

```yaoxiang
# コンパイラが自動生成（ユーザは認識しない）
AnimalGroup: Type = {
    Dog(Dog),
    Cat(Cat),
    Bird(Bird),    # ← append(Bird.new()) がインクリメンタル拡張を引き起こす
}

# List(Animal) は内部的に List(AnimalGroup) と等価
```

#### 6.3 インターフェースマッチングチェック

**重要な洞察**：インターフェースマッチングはコンパイル時にチェックされる、動的にロードされるプラグインからの型であっても。

```yaoxiang
# プラグインシステム
plugin = load_plugin("bird.so")

# コンパイラがチェック：plugin.create_bird() の戻り型は Animal を実装しなければならない
bird: Animal = plugin.create_bird()  # コンパイル時チェック、存在型

# 異種コンテナに格納 —— append 点で enum 拡張がトリガーされる
animals: List(Animal) = [Dog.new(), Cat.new()]
animals.append(bird)                 # コンパイラ：(1) bird が Animal を実装していることを検証 (2) enum を拡張
```

**コンパイラの処理**：

1. `append` 引数の戻り型をチェック
2. その型が対象インターフェースを実装しているかを検証
3. 通過 → enum を拡張し、格納を許可
4. 失敗 → コンパイルエラー

#### 6.4 ランタイムディスパッチ

**呼び出しフロー（コンパイル時に生成された enum match、ImplementationProof は既に消去済み）：**

```
animals[0].speak()
  ↓
コンパイラが生成した match:
  match animals[0] {
    AnimalGroup.Dog(d) => d.speak(),
    AnimalGroup.Cat(c) => c.speak(),
    AnimalGroup.Bird(b) => b.speak(),
  }
```

**ブランド投影**（RFC-009a との相互作用）：match のパターン束縛 `AnimalGroup.Dog(d)`
はブランドツリーに `#animals[0].Dog`
サブブランドを生成し、フィールド投影（`#42.field_x`）と等価である。`d.speak()` が作成する
`ReadToken(d)` のブランドチェーンは `animals → animals[0] → d → ReadToken(d)`
であり、借用チェッカーはブランドツリーのプレフィックスマッチングで競合を検証する。

**インデックスアクセスの型**：`animals[0]` は
`&AnimalGroup`（コンパイラが生成した enum 型）を返し、ユーザは直接 `&mut Animal`
を取得できない。可変アクセスはインターフェースメソッドを介して間接的に実現される（例えば
`animals[0].mutate()` は内部的に `AnimalGroup::Dog(d) => d.mutate()` に展開される）。

**仮想テーブルとの比較**：

|                     | 仮想テーブル（Rust）                    | コンパイル時 enum（YaoXiang）                    |
| ------------------- | --------------------------------------- | ------------------------------------------------ |
| ルックアップ方式    | 仮想テーブルポインタ → メソッドポインタ | enum match → 直接呼び出し                        |
| ランタイムコスト    | 1 回の間接アドレス指定                  | 分岐（CPU の分岐予測で最適化可能）               |
| コンパイル時生成    | 仮想テーブル                            | enum + match                                     |
| ユーザ注釈          | `dyn Trait + 'a` が必要                 | 不要                                             |
| ImplementationProof | 該当なし                                | コンパイル時に消去され、ランタイムには存在しない |

**YaoXiang の利点**：

- ブランド注釈が不要
- コンパイル時の型安全性
- ユーザにとって透過的（`dyn Animal` と書く必要がない）
- ImplementationProof は純粋にコンパイル時の概念であり、ランタイムコストはゼロ

#### 6.5 制限とスコープ

**当期（単一コンパイルユニット）：** 完全サポート。所有権追跡がすべての
`append`/構築点をカバーし、enum をインクリメンタルに構築する。

**コンパイルユニット間：** LTO（リンク時最適化）に依存して enum variant をマージ。`Animal`
は存在型（`∃S. Animal(S)`）としてコンパイルユニットの境界を渡る。各ユニットは部分的な enum
variant を生成し、リンク段階でマージされる。

**サポート対象外：**
ランタイム動的型（完全なダックタイピング）。型の集合はコンパイル時に完全に既知でなければならない。

---

## ユースケース分析

### 基本的なインターフェース実装

```yaoxiang
# インターフェース定義
Animal: (Self: Type) -> Type = {
    speak: (self: Self) -> String,
}

# 型定義
Dog: Type = {
    x: Int = 10,
    Animal(Dog),
    speak: (self: Dog) -> String = "Woof",
}

# 使用
dog = Dog.new()
dog.speak()  # "Woof"
```

### 複数インターフェースの実装

```yaoxiang
# 複数のインターフェース
Animal: (Self: Type) -> Type = {
    speak: (self: Self) -> String,
}

Pet: (Self: Type) -> Type = {
    name: (self: Self) -> String,
}

# 型が複数のインターフェースを実装
Dog: Type = {
    x: Int = 10,
    Animal(Dog),
    Pet(Dog),
    speak: (self: Dog) -> String = "Woof",
    name: (self: Dog) -> String = "Buddy",
}

# 使用
dog = Dog.new()
dog.speak()  # "Woof"
dog.name()   # "Buddy"
```

### ジェネリックインターフェース

```yaoxiang
# ジェネリックインターフェース
Container: (Self: Type, T: Type) -> Type = {
    add: (self: &mut Self, item: T) -> Void,
    get: (self: &Self, index: Int) -> T,
}

# ジェネリックインターフェースを実装
IntList: Type = {
    data: Array(Int),
    Container(IntList, Int),
    add: (self: &mut IntList, item: Int) -> Void = ...,
    get: (self: &IntList, index: Int) -> Int = ...,
}
```

### 異種コンテナ

```yaoxiang
# インターフェース定義
Animal: (Self: Type) -> Type = {
    speak: (self: Self) -> String,
}

# 型定義
Dog: Type = {
    x: Int,
    Animal(Dog),
    speak: (self: Dog) -> String = "Woof",
}

Cat: Type = {
    y: Int,
    Animal(Cat),
    speak: (self: Cat) -> String = "Meow",
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
Plugin: (Self: Type) -> Type = {
    name: (self: Self) -> String,
    execute: (self: Self) -> Void,
}

# メインプログラム
main: () -> Void = {
    # プラグインのロード
    plugin1 = load_plugin("plugin1.so")
    plugin2 = load_plugin("plugin2.so")

    # コンパイラがチェック：plugin1 と plugin2 は Plugin インターフェースを実装しなければならない
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
3. **統一**：オーバーロード規則が一貫
4. **便利**：デフォルト値の構文が簡潔
5. **ゼロコスト**：仮想テーブルなし、コンパイル時の型収集
6. **型安全**：インターフェースマッチングはコンパイル時にチェック
7. **ユーザにとって透過的**：`dyn Animal + 'a` と書く必要がない

### 欠点

1. **制限**：ランタイム動的型（完全なダックタイピング）をサポートしない
2. **コンパイル時のコスト**：各インターフェースに対して enum
   variant と match ディスパッチコードを生成する必要がある
3. **型の集合**：コンパイル時に完全に既知でなければならない（単一コンパイルユニット内）

### 緩和策

1. **プラグインシステム**：コンパイル時のインターフェースマッチングチェックによってサポート
2. **型の集合**：所有権追跡によるインクリメンタル構築——各 `append`/構築点で収集し、全体走査ではない
3. **コンパイルユニット間**：リンク時に enum
   variant 集合をマージ。リンク時単態化と共通のメカニズムを使用

---

## 代替案

| 案                          | 選択しない理由                   |
| --------------------------- | -------------------------------- |
| `impl` キーワード           | 構文の複雑さが増す               |
| 仮想テーブル（`dyn Trait`） | ブランド注釈（`'a`）が必要       |
| 完全なダックタイピング      | ランタイムコスト、型が安全でない |
| enum ラッピング（手動）     | ユーザの負担が大きい             |

---

## RFC-009 との関係

**ブランドとインターフェース実装**：

- インターフェース実装は型層にあり、ブランドには関与しない
- ブランドは借用証明層にある（RFC-009a）
- 両者は直交し、相互に影響しない

**動的ディスパッチとブランド**：

- 動的ディスパッチは実装証明を使用し、ブランド注釈は不要
- 実装証明はコンパイル時に生成され、ランタイムのルックアップはゼロ
- `dyn Trait + 'a` の複雑さを回避

**異種コンテナの所有権**：

- `List(Animal)` への格納は Move セマンティクス（RFC-009）であり、元の変数はアクセス不可
- インデックスアクセス `animals[0]` は
  `&AnimalGroup`（コンパイラ生成の enum）を返し、ブランド投影チェーンは
  `animals → animals[0] → enum_variant → field`
- 可変アクセスはインターフェースメソッドを介して間接的に実現され、ユーザに `&mut AnimalGroup`
  を露出させない

## インターフェース継承

インターフェースは別のインターフェースを含むことができる。**新しい構文を導入しない**——型がインターフェースを宣言するのと同じ構文位置を使用する：

```yaoxiang
Animal: (Self: Type) -> Type = {
    speak: (self: Self) -> String,
}

Pet: (Self: Type) -> Type = {
    Animal(Self),                       # Pet は Animal を継承 — 新しいキーワードなし
    name: (self: Self) -> String,
}

# Dog が Pet を実装する際、Animal と Pet のすべてのメソッドを同時に満たす必要がある
Dog: Type = {
    x: Int,
    Pet(Dog),
    speak: (self: Dog) -> String = "Woof",  # Animal 由来
    name: (self: Dog) -> String = "Buddy",  # Pet 由来
}
```

**設計原則：**
継承は存在するが、乱用は推奨されない。主な構成方法は複数のインターフェースインスタンス化による（`Dog: Type = { Animal(Dog), Pet(Dog), ... }`）。ある型は、継承ツリーを通じて表現することなく、それが満たすすべてのインターフェースを直接宣言できる。インターフェース継承は明確な「is-a」階層がある場合にのみ使用する。

**コンパイラの処理：** 継承チェーンを展開する。`Pet(Self)` を
`{ Animal(Self) のすべてのメソッド, name: ... }` に展開する。`Dog` が `Pet(Dog)`
を宣言する際、`Self ↦ Dog` が行われ、コンパイラは `Dog` が `Animal(Dog)` と `Pet(Dog)`
のすべてのメソッドを同時に満たしていることを検証する。

**インターフェース継承における Self 置換**：`Pet: (Self: Type) -> Type = { Animal(Self), ... }`
において、`Animal(Self)` の `Self` は `Pet` の `Self` パラメータであり、遅延的に置換される。`Dog` が
`Pet(Dog)` を実装する際、`Self ↦ Dog` が行われ、`Animal(Self)` は `Animal(Dog)`
になる。これはジェネリック関数の引数渡しセマンティクスと完全に一致している。

## デフォルトメソッド実装

インターフェースはメソッドのデフォルト実装を提供できる。実装型はオーバーライドするか、デフォルト実装を継承するかを選択できる：

```yaoxiang
fmt: (Self: Type) -> Type = {
    display: (self: Self) -> String,                      # 必ず実装
    debug: (self: Self) -> String = self.display(),       # ✅ 同じインターフェースのメソッドを参照
    summary: (self: Self) -> String = f"<{self.name}>",  # ❌ コンパイルエラー：self.name は fmt に存在しない
}
```

**中核制約：インターフェースは上位の実装を想定できない。**
デフォルトメソッドは同じインターフェースですでに宣言されているメソッドのみを参照できる。具体的な型のフィールドや他のインターフェースメソッドはデフォルトメソッドからは見えない——インターフェースは閉じた契約であり、実装型のポケットに手を伸ばすことはできない。この制約に違反すると**インターフェース定義時**に直接エラーとなる。

**継承は下位の実装を想定できる：** インターフェース `Pet(Self)` が `Animal(Self)`
を継承する際、`Pet` のデフォルトメソッドは `Animal`
が宣言したメソッドを使用できる——継承しているため、存在が保証されている。

```yaoxiang
Animal: (Self: Type) -> Type = {
    speak: (self: Self) -> String,
}

Pet: (Self: Type) -> Type = {
    Animal(Self),                                              # 継承
    name: (self: Self) -> String,
    introduce: (self: Self) -> String = self.name() + " says " + self.speak(),  # ✅ speak は継承した Animal 由来
}
```

**コンパイル時の振る舞い：** 型がインターフェースを実装する際、各メソッドについて：

1. 型が提供 → 型のメソッドを使用
2. 型が未提供、インターフェースにデフォルトあり → デフォルト実装を型にインライン化（仮想テーブルコストゼロ）
3. 型が未提供、インターフェースにデフォルトなし → コンパイルエラー

**設計原則：** デフォルトメソッドは `Copy`/`Clone`
の自動導出機構に似ている——コンパイラが必要に応じて自動生成し、ユーザはオーバーライドできる。`virtual`/`override`/`super`
キーワードを導入しない。
---

## 実装フェーズ

| フェーズ | 内容                                                                       | 依存    |
| -------- | -------------------------------------------------------------------------- | ------- |
| Phase 1  | インターフェース宣言構文（`(Self: Type) -> Type`） + Self 型パラメータ     | RFC-011 |
| Phase 2  | インターフェースインスタンス化（`Animal(Dog)`） + Self ↦ ConcreteType 置換 | Phase 1 |
| Phase 3  | メソッド実装の内部/外部宣言                                                | Phase 2 |
| Phase 4  | オーバーロードとオーバーライド規則                                         | Phase 3 |
| Phase 5  | デフォルト値構文                                                           | Phase 3 |
| Phase 6  | インターフェース継承                                                       | Phase 4 |
| Phase 7  | デフォルトメソッド実装                                                     | Phase 6 |
| Phase 8  | 実装証明の生成                                                             | Phase 7 |
| Phase 9  | コンパイル時の型収集                                                       | Phase 8 |
| Phase 10 | 動的ディスパッチの実装                                                     | Phase 9 |

---

## 設計決定記録

| 決定                        | 決定内容                                                                                       | 理由                                                                                                                                 | 日付       |
| --------------------------- | ---------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------ | ---------- |
| インターフェース宣言構文    | インターフェースはパラメータ化された型 `(Self: Type) -> Type` で、実装時にインスタンス化       | `Self` マジックキーワードを排除し、RFC-011 のジェネリック型システムと完全に統一                                                      | 2026-06-14 |
| Self 型パラメータ           | 明示的な型パラメータ。インターフェース定義時は構文チェックのみ、インスタンス化点で完全チェック | HM 推論における自由型変数を避ける                                                                                                    | 2026-06-14 |
| 動的ディスパッチ            | コンパイル時の型収集 + 自動 enum 生成                                                          | 仮想テーブルなし、ランタイムルックアップゼロ、ユーザにとって透過的                                                                   | 2026-06-14 |
| 外部メソッド宣言            | サポート                                                                                       | 内部宣言と同等の柔軟性。コンパイラがファイル横断収集を担当                                                                           | 2026-06-14 |
| オーバーライド              | 禁止（同シグネチャはエラー）                                                                   | オーバーライドは予測不能な動作を引き起こす。オーバーロードが全ケースをカバー                                                         | 2026-06-14 |
| インターフェース継承        | サポート、新しい構文なし                                                                       | 型がインターフェースを宣言するのと同じ構文位置。組み合わせ（複数インターフェースインスタンス化）を推奨し、深い継承ツリーは推奨しない | 2026-07-03 |
| デフォルトメソッド実装      | サポート、Copy/Clone 自動導出に類似                                                            | インターフェースがデフォルト本体を提供。コンパイラは実装型にインライン化。ユーザはオーバーライド可能。virtual/override は導入しない  | 2026-07-03 |
| デフォルトメソッド制約      | インターフェース定義時に検証：同じインターフェースのメソッドのみ参照可能、上位実装を想定不可   | インターフェースは閉じた契約。継承は下位実装を想定できるが、インターフェースは実装型のフィールド/メソッドを想定できない              | 2026-07-03 |
| 型収集戦略                  | 所有権追跡によるインクリメンタル構築——各 append/構築点で収集                                   | すべての実装者を全体走査するのではなく、所有権操作点で enum をインクリメンタルに拡張する                                             | 2026-07-03 |
| ImplementationProof         | 純粋にコンパイル時の概念、ランタイムに消去                                                     | ランタイムは enum match ディスパッチを使用。証明はコンパイル時検証にのみ使用                                                         | 2026-07-03 |
| コンパイルユニット間        | LTO による enum variant のマージ                                                               | 存在型がコンパイルユニットの境界を渡る際、各ユニットは部分的な enum を生成し、LTO 段階でマージ                                       | 2026-07-03 |
| フィールド/メソッド名前空間 | 統一名前空間、衝突はエラー                                                                     | フィールドアクセス `point.x` とメソッド呼び出し `point.x()` は構文上区別できないため、統一で曖昧さを排除                             | 2026-07-03 |
| 異種コンテナの所有権        | Move セマンティクス。コンテナ格納後は元の変数は使用不可                                        | RFC-009 の所有権モデルと一致                                                                                                         | 2026-07-03 |
| ブランド投影                | match パターン束縛がサブブランドを生成し、フィールド投影と等価                                 | RFC-009a のブランドツリーメカニズムと一致。enum variant 投影はブランドツリーの正当なパス                                             | 2026-07-03 |

## オープン問題

- [x] ~~インターフェース継承（インターフェースが他のインターフェースを継承できる）~~
      → サポート、新しい構文なし。`Pet: (Self: Type) -> Type = { Animal(Self), ... }`
- [x] ~~デフォルトメソッド実装（インターフェースがデフォルト実装を提供できる）~~
      → サポート、Copy 自動導出に類似。インターフェースが本体を提供し、コンパイラは必要に応じてインライン化
- [x] ~~Self を暗黙のマジックキーワードにすること~~ → 排除。`Self`
      は明示的な型パラメータであり、インターフェースは `(Self: Type) -> Type`
- [ ] インターフェース制約の高度な使い方（関連型、GAT）—— 関連型はジェネリックインターフェースパラメータで実現（`Container: (Self: Type, T: Type) -> Type`）。GAT はさらなる設計が必要
- [ ] クロージャとの相互作用（クロージャがインターフェースを実装する）—— 初期戦略：クロージャは直接インターフェースを実装できず、wrapper 型が必要。匿名型のインターフェース実装は今後の RFC に委ねる

---

## 参考文献

- [RFC-011: ジェネリック型システム設計](../accepted/011-generic-type-system.md) — 親 RFC
- [RFC-009: 所有権モデル設計](../accepted/009-ownership-model.md) — 所有権システム
- [RFC-009a: 借用証明パイプライン](../accepted/009a-borrow-proof-pipeline.md) — ブランドメカニズム
- [RFC-010: 統一型構文](../accepted/010-unified-type-syntax.md) — 統一構文

---

## ライフサイクルと帰結

| 状態       | 位置                      | 説明                       |
| ---------- | ------------------------- | -------------------------- |
| **審査中** | `docs/design/rfc/review/` | オープンなコミュニティ議論 |
