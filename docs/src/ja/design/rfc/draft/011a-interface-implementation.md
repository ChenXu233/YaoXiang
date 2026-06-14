---
title: "RFC-011a: インターフェース実装と動的ディスパッチ"
status: "草案"
author: "晨煦"
created: "2026-06-14"
updated: "2026-06-14"
group: "rfc-011"
---

# RFC-011a: インターフェース実装と動的ディスパッチ

> **親 RFC**: [RFC-011: ジェネリクスシステム設計](../accepted/011-generic-type-system.md)
>
> **本 RFC は RFC-011 §2.1-2.4 のインターフェース制約部分を補完し、置換する。**

## 要約

RFC-011 はジェネリクスシステムを定義したが、インターフェース実装メカニズムについて詳細な説明がなかった。本文書は以下を補完する：

1. **インターフェース宣言**：型定義内で直接インターフェース名を記述し、`impl` キーワードは不要
2. **メソッド実装**：内部宣言と外部宣言の両方をサポート
3. **オーバーロード規則**：シグネチャが異なればオーバーロードを許可、シグネチャが同じ場合はエラー（オーバーライド禁止）
4. **デフォルト値**：フィールドの直後に `= value` と記述
5. **動的ディスパッチ**：コンパイル時の型収集 + インターフェースマッチング、仮想テーブルなし

**核心設計**：

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

**除去された複雑性**：
- ❌ `impl` キーワードなし
- ❌ `dyn Trait + 'a` 注釈なし
- ❌ 仮想テーブルなし（コンパイル時型収集 + 列挙型ラッパー）
- ❌ オーバーライドなし（オーバーロード規則で統一）

---

## 動機

### RFC-011 の不足

RFC-011 はジェネリクスシステムを定義したが、以下について詳細な説明がなかった：

| 問題 | 説明 |
|------|------|
| インターフェース宣言構文 | 型がインターフェースを実装していることをどう宣言するか？ |
| メソッド実装の位置 | 内部宣言か外部宣言か？ |
| オーバーロード規則 | 同名のメソッドをどう処理するか？ |
| デフォルト値構文 | フィールドにデフォルト値を設定する構文は？ |
| 動的ディスパッチ | 異種コンテナをどう実装するか？ |

### 設計目標

1. **簡潔**：`impl` キーワードが不要
2. **柔軟**：メソッド実装は内部・外部の両方をサポート
3. **統一**：オーバーロード規則が一貫
4. **便利**：デフォルト値構文が簡潔
5. **ゼロオーバーヘッド**：仮想テーブルなし、コンパイル時型収集

### Rust との比較

| 特性 | Rust | YaoXiang |
|------|------|----------|
| インターフェース宣言 | `impl Animal for Dog { ... }` | `Dog: Type = { Animal, ... }` |
| メソッド実装 | `impl` ブロック内 | 内部または外部 |
| オーバーロード | 非対応 | 対応（シグネチャが異なる場合） |
| デフォルト値 | `#[default]` が必要 | `= value` と直接記述 |
| 異種コンテナ | `Vec<Box<dyn Animal + 'a>>` | `List(Animal)` |
| 動的ディスパッチ | 仮想テーブルルックアップ | コンパイル時型収集 |

---

## 提案

### 1. インターフェース宣言

**核心規則**：型定義内で直接インターフェース名を記述し、`impl` キーワードは不要。

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

**コンパイラの処理**：
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
- 直接展開すると出典情報が失われる
- 出典マークは実装証明の生成に使用される
- ランタイムは証明を通じて正しいメソッドを見つける

### 2. メソッド実装

**核心規則**：メソッド実装は内部宣言と外部宣言の両方をサポートする。

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

**コンパイラの処理**：
1. すべての定義を収集（内部と外部）
2. シグネチャでグループ化（オーバーロード）
3. オーバーライドがないかチェック（エラー）
4. インターフェースの完全性をチェック
5. 実装証明を生成

### 3. オーバーロードとオーバーライド

**核心規則**：
- シグネチャが異なる → オーバーロード → 許可
- シグネチャが同じ → オーバーライド → エラー

#### 3.1 オーバーロード（許可）

```yaoxiang
# 引数の型が異なるため、オーバーロードを許可
Dog.speak: (Self) -> String = "Woof"
Dog.speak: (Self, volume: Int) -> String = "WOOF"
```

#### 3.2 オーバーライド（禁止）

```yaoxiang
# シグネチャが完全一致するため、オーバーライドを禁止
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

**核心規則**：フィールドの直後に `= value` と記述し、コンストラクタ関数を省略する。

```yaoxiang
Dog: Type = {
    x: Int = 10,  # デフォルト値
    y: Int = 20,  # デフォルト値
    Animal,
}
```

**コンパイラ生成のコンストラクタ**：

```yaoxiang
# すべてのフィールドにデフォルト値がある → 引数なしコンストラクタを生成
Dog.new: () -> Dog = { x: 10, y: 20 }

# 一部のフィールドにデフォルト値がある → 一部引数コンストラクタを生成
Dog.new: (x: Int) -> Dog = { x: x, y: 20 }
Dog.new: (y: Int) -> Dog = { x: 10, y: y }

# 全引数コンストラクタ
Dog.new: (x: Int, y: Int) -> Dog = { x: x, y: y }
```

**デフォルト値の外部宣言**：

```yaoxiang
Dog: Type = {
    x: Int,
    y: Int,
    Animal,
}

# デフォルト値の外部宣言
Dog.x: Int = 10
Dog.y: Int = 20
```

**内部宣言と等価**。

### 5. コンパイラ実装

#### 5.1 インターフェース記述子

```rust
// コンパイラ内部：インターフェース記述子
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
2. すべてのメソッド定義を収集（内部と外部）
3. シグネチャでグループ化（オーバーロード）
4. オーバーライドをチェック（エラー）
5. インターフェースの完全性をチェック
6. 実装証明を生成
7. ランタイムで、値は実装証明を保持
```

### 6. 動的ディスパッチ

**核心設計**：コンパイル時型収集 + インターフェースマッチング、仮想テーブルなし。

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

**コンパイラの処理**：

```
1. List(Animal) に格納されるすべての型をスキャン
2. 収集：Dog, Cat
3. AnimalGroup 列挙型を自動生成
4. AnimalGroup 用の単態化コードを生成
5. ランタイムで列挙型マッチングによりディスパッチ
```

**自動生成された列挙型**：

```yaoxiang
# コンパイラが自動生成（ユーザーは意識しない）
AnimalGroup: Type = {
    Dog(Dog),
    Cat(Cat),
}

# List(Animal) は List(AnimalGroup) と等価
animals: List(AnimalGroup) = [
    AnimalGroup.Dog(Dog.new()),
    AnimalGroup.Cat(Cat.new()),
]
```

#### 6.3 インターフェースマッチングチェック

**重要な洞察**：インターフェースマッチングはコンパイル時にチェックされるため、型が動的にロードされるプラグイン由来であっても問題はない。

```yaoxiang
# プラグインシステム
plugin = load_plugin("bird.so")

# コンパイラチェック：plugin.create_bird() の戻り型は Animal を実装しなければならない
bird: Animal = plugin.create_bird()  # コンパイル時チェック

# 異種コンテナに格納
animals: List(Animal) = [Dog.new(), Cat.new(), bird]
```

**コンパイラの処理**：
1. `plugin.create_bird()` の戻り型をチェック
2. その型が `Animal` インターフェースを実装しているか検証
3. 通過 → `List(Animal)` への格納を許可
4. 失敗 → コンパイルエラー

#### 6.4 ランタイムディスパッチ

**呼び出しフロー**：

```
animals[0].speak()
  ↓
animals[0] の実装証明（Animal インターフェース）を見つける
  ↓
証明から speak メソッドのポインタを取得
  ↓
メソッドを呼び出す
```

**仮想テーブルとの比較**：

| | 仮想テーブル（Rust） | 実装証明（YaoXiang） |
|---|---|---|
| 検索方式 | 仮想テーブルポインタ → メソッドポインタ | 実装証明 → メソッドポインタ |
| ランタイムオーバーヘッド | 1 回の間接参照 | 1 回の間接参照 |
| コンパイル時生成 | 仮想テーブル | 実装証明 |
| ブランド注釈 | `dyn Trait + 'a` が必要 | 不要 |

**YaoXiang の利点**：
- ブランド注釈が不要（実装証明は `'a` を必要としない）
- コンパイル時の型安全（インターフェースマッチングはコンパイル時チェック）
- ユーザーに透明（`dyn Animal` と書く必要がない）

#### 6.5 制限

**ランタイム動的型は非対応**：
- 型の集合はコンパイル時に完全に既知である必要がある
- プラグインシステムはコンパイル時のインターフェースマッチングチェックが必要
- 完全なダックタイピングは非対応（ランタイムでのメソッド存在チェック）

**型の集合の爆発は問題にならない**：
- 線形収集で十分
- 各型に対して列挙型のバリアントを生成
- 各組み合わせごとにコードを生成する必要はない

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

### 複数インターフェースの実装

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

### ジェネリックインターフェース

```yaoxiang
# ジェネリックインターフェース
Container: (T: Type) -> Type = {
    add: (self: &mut Self, item: T) -> Void,
    get: (self: &Self, index: Int) -> T,
}

# ジェネリックインターフェースの実装
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
3. **統一**：オーバーロード規則が一貫
4. **便利**：デフォルト値構文が簡潔
5. **ゼロオーバーヘッド**：仮想テーブルなし、コンパイル時型収集
6. **型安全**：インターフェースマッチングはコンパイル時チェック
7. **ユーザーに透明**：`dyn Animal + 'a` と書く必要がない

### 欠点

1. **制限**：ランタイム動的型は非対応（完全なダックタイピング）
2. **コンパイル時コスト**：各インターフェースに対して実装証明を生成する必要がある
3. **型の集合**：コンパイル時に完全に既知である必要がある

### 緩和策

1. **プラグインシステム**：コンパイル時のインターフェースマッチングチェックでサポート
2. **コンパイル時コスト**：実装証明は軽量なデータ構造
3. **型の集合**：線形収集であり、指数爆発ではない

---

## 代替案

| 案 | 選択しない理由 |
|------|--------------|
| `impl` キーワード | 構文の複雑性が増す |
| 仮想テーブル（`dyn Trait`） | ブランド注釈（`'a`）が必要 |
| 完全なダックタイピング | ランタイムオーバーヘッド、型安全でない |
| 列挙型ラッパー（手動） | ユーザーの負担が大きい |

---

## RFC-009 との関係

**ブランドとインターフェース実装**：
- インターフェース実装は型層にあり、ブランドには関与しない
- ブランドは借用証明層にある（RFC-009a）
- 両者は直交しており、互いに影響しない

**動的ディスパッチとブランド**：
- 動的ディスパッチは実装証明を使用し、ブランド注釈が不要
- 実装証明はコンパイル時に生成され、ランタイム検索はゼロ
- `dyn Trait + 'a` の複雑性を回避

---

## 実装フェーズ

| フェーズ | 内容 | 依存 |
|------|------|------|
| Phase 1 | インターフェース宣言構文 | RFC-011 |
| Phase 2 | メソッド実装の内部/外部宣言 | Phase 1 |
| Phase 3 | オーバーロードとオーバーライド規則 | Phase 2 |
| Phase 4 | デフォルト値構文 | Phase 2 |
| Phase 5 | 実装証明生成 | Phase 3 |
| Phase 6 | コンパイル時型収集 | Phase 5 |
| Phase 7 | 動的ディスパッチ実装 | Phase 6 |

---

## 未解決の問題

- [ ] インターフェース継承（インターフェースが他のインターフェースを継承する）
- [ ] デフォルトメソッド実装（インターフェースがデフォルト実装を提供）
- [ ] インターフェース制約の高度な使用法（関連型、GAT）
- [ ] クロージャとの相互作用（クロージャがインターフェースを実装）

---

## 参考文献

- [RFC-011: ジェネリクスシステム設計](../accepted/011-generic-type-system.md) — 親 RFC
- [RFC-009: 所有権モデル設計](../accepted/009-ownership-model.md) — 所有権システム
- [RFC-009a: 借用証明パイプライン](../accepted/009a-borrow-proof-pipeline.md) — ブランド機構
- [RFC-010: 統一型構文](../accepted/010-unified-type-syntax.md) — 統一構文

---

## ライフサイクルと帰趣

| 状態 | 場所 | 説明 |
|------|------|------|
| **草案** | `docs/design/rfc/` | 著者の草稿、レビューの提出待ち |