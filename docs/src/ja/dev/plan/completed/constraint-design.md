# RFC-010 / RFC-011 制約（Constraint）設計決定

> **状態**: 確定
> **作成日**: 2026-02-02
> **最終更新**: 2026-02-02
> **関連 RFC**: [RFC-010 統一型構文](../design/accepted/010-unified-type-syntax.md), [RFC-011 ジェネリックシステム](../design/accepted/011-generic-type-system.md)

---

## 一、コア設計原則

### 1.1 制約 = 能力要件

制約は、型が提供しなければならない能力（フィールドとメソッド）を定義します：

```yaoxiang
# 制約の定義（必要な能力を要求）
type Drawable = {
    draw: (Self, Surface) -> Void,
    bounding_box: (Self) -> Rect
}

type Serializable = {
    serialize: (Self) -> String
}
```

### 1.2 制約はジェネリックコンテキスト内でのみ使用可能

**❌ 不許可**：直接ダックタイピング代入

```yaoxiang
let d: Drawable = Circle(1)  # 不許可！
```

**✅ 許可**：ジェネリック制約

```yaoxiang
draw: [T: Drawable](item: T) -> Void = (item) => {
    item.draw(screen)
}

# 呼び出し時に自動チェック
draw(Circle(1))  # ✅ Circle は draw を持つ、通过
draw(Rect(2))    # ❌ Rect は draw を持たない、コンパイルエラー
```

**理由**：

- 直接代入は「事後チェック」であり、偶然一致する可能性がある
- ジェネリック制約は「事前検証」であり、意図が明確

---

## 二、使用シナリオ

### 2.1 ジェネリック関数パラメータ

```yaoxiang
# 描画可能な任意のオブジェクトを処理
process: [T: Drawable](items: List[T]) -> Void = (items) => {
    for item in items {
        item.draw(screen)
    }
}
```

### 2.2 ジェネリック戻り値型

```yaoxiang
# シリアライズ可能なオブジェクトを返す
serialize_all: [T: Serializable](items: List[T]) -> List[String] = {
    items.map((item) => item.serialize())
}
```

### 2.3 ジェネリックデータコンテナ

```yaoxiang
# コンテナの要素は描画可能でなければならない - 誤った写法
let shapes: List[Drawable] = []  # エラー！制約は非ジェネリックコンテキストで使用不可

# 正しい：ジェネリックパラメータを使用
let shapes: List[Circle] = []
```

---

## 三、構造化サブタイプルール

### 3.1 照合ルール

```yaoxiang
# 制約定義
type Config = {
    load: () -> String,
    save: (String) -> Void,
    name: String
}

# 型定義
type File = {
    filename: String,
    load: () -> String,
    save: (String) -> Void,
    size: Int,          # 追加フィールド、無視
}

# File が Config を満たすかどうかチェック：
#   - load: () -> String ✅ 照合
#   - save: (String) -> Void ✅ 照合
#   - name: String ❌ 不照合（名前は filename で、name ではない）
#
# 結果：❌ File は Config を満たさない
```

### 3.2 照合アルゴリズム

| 要求 | 型が提供 | 結果 |
|------|---------|------|
| `x: Int` | `x: Int` | ✅ 照合 |
| `x: Int` | `y: Int` | ❌ 不照合 |
| `x: Int` | `x: String` | ❌ 不照合 |
| `fn: (A) -> B` | `fn: (Self, A) -> B` | ✅ 照合（Self を除去） |
| 要求されたフィールド/メソッド | 追加のフィールド/メソッド | ✅ 無視 |

### 3.3 コンパイラチェックフロー

```rust
fn check_type_satisfies_constraint(
    typ: &MonoType,
    constraint: &MonoType,
) -> Result<(), ConstraintCheckError> {
    // 1. constraint が有効な制約であることを検証（すべてのフィールドが関数型）
    if !constraint.is_valid_constraint() {
        return Err(ConstraintCheckError::NotAConstraint);
    }

    // 2. 制約のすべての要件を走査
    for (name, required_type) in constraint.required_fields() {
        match lookup_type_field(typ, name) {
            Some(found_type) => {
                // 3. 型互換性をチェック
                if !types_compatible(found_type, required_type, typ) {
                    return Err(ConstraintCheckError::TypeMismatch {
                        field: name,
                        expected: required_type,
                        found: found_type,
                    });
                }
            }
            None => {
                // 4. 必須フィールドが欠落
                return Err(ConstraintCheckError::MissingField {
                    field: name,
                    constraint: constraint.name(),
                });
            }
        }
    }

    Ok(())
}
```

---

## 四、型定義時の制約宣言（オプション）

型定義時にどの制約を実装するかを宣言でき、コードの可読性と IDE ヒントに便利です：

```yaoxiang
# 型を宣言する際に制約を実装
type Point = {
    x: Int,
    y: Int,
    draw: (Point, Surface) -> Void,
    bounding_box: (Point) -> Rect,
    serialize: (Point) -> String,
    Drawable,      # Drawable を実装することを宣言
    Serializable   # Serializable を実装することを宣言
}
```

**効果**：

- ✅ コードが自己文書化される
- ✅ IDE が「Point は Drawable を実装」とヒントを表示できる
- ✅ コンパイラが宣言の正しさを検証する

---

## 五、エラー処理

### 5.1 エラー型

```rust
pub enum ConstraintCheckError {
    #[error("'{0}' is not a valid constraint (must have function fields only)")]
    NotAConstraint(String),

    #[error("Type '{type}' does not satisfy constraint '{constraint}': missing field '{field}'")]
    MissingField {
        type_name: String,
        constraint: String,
        field: String,
        span: Span,
    },

    #[error("Type '{type}' does not satisfy constraint '{constraint}': field '{field}' type mismatch")]
    TypeMismatch {
        type_name: String,
        constraint: String,
        field: String,
        expected: String,
        found: String,
        span: Span,
    },

    #[error("Constraint '{0}' can only be used in generic context")]
    NotInGenericContext {
        constraint_name: String,
        span: Span,
    },
}
```

### 5.2 エラーの例

```
Error: Type 'Rect' does not satisfy constraint 'Drawable'
  Required method 'draw: (Rect, Surface) -> Void' not found
  Note: Add 'draw' method to Rect to satisfy Drawable

Error: Constraint 'Serializable' can only be used in generic context
  Did you mean: 'serialize_all: [T: Serializable](List[T]) -> List[String]'
```

---

## 六、なぜこの設計なのか？

### 6.1 他の方案との比較

| 方案 | 問題点 |
|------|------|
| `let d: Drawable = Circle(1)` | 偶然一致、ダックタイピングが寛容すぎる |
| `impl Drawable for Circle` | 新しいキーワードが必要、RFC-010 設計原則に反する |
| `as Config` 変換 | 構文複雑度が増加 |
| **現在方案：ジェネリック制約** | 意図清晰、コンパイル時チェック、偶然一致なし |

### 6.2 設計原則

1. **制約 = 能力要件**：必要なものを定義するのみ
2. **ジェネリック = 事前検証**：呼び出し前にチェック、偶然一致を許さない
3. **キーワード追加ゼロ**：既存構文を再利用
4. **コンパイル時安全**：すべてのチェックがコンパイル時に完了

---

## 七、ファイル構造

```
src/frontend/
├── core/
│   └── type_system/
│       └── mono.rs              # MonoType 拡張（is_constraint）
│
└── typecheck/
    ├── checking/
    │   ├── mod.rs               # constraint モジュールのエクスポート
    │   └── constraint.rs        # ⬅️ 制約チェッカー（新規追加）
    ├── errors.rs                # TypeError 拡張
    └── ...
        └── tests/
            └── test_constraint.rs # ⬅️ 制約チェックテスト（新規追加）
```

---

## 八、受け入れる基準

- [ ] 制約定義構文が正常に動作
- [ ] ジェネリック制約 `[T: Drawable]` が正常に動作
- [ ] `let d: Drawable = ...` が正しく拒否される
- [ ] 構造化照合ルールが正しく実装
- [ ] エラーメッセージが明確で正確
- [ ] すべての既存テストが通過
- [ ] 10+ 件の新規ユニットテスト追加

---

## 九、Q&A

### Q: なぜ `let d: Drawable = Circle(1)` を許可しないのか？

A: これは「事後検証」であり、Circle がたまたま draw メソッドを持っているだけで受け入れられてしまいます。意図した設計ではない可能性があります。ジェネリック制約は「事前検証」であり、コードが明確に「Drawable 能力が必要」と言います。

### Q: Drawable オブジェクトのグループを保存するにはどうすればいいですか？

A: ジェネリックコンテナまたはインターフェースパターンを使用します：

```yaoxiang
# 方法1：統一具体的な型
let shapes: List[Circle] = []
shapes.push(Circle(1))

# 方法2：トレイト/インターフェースパターンの使用（ランタイムディスパッチが必要）
# 異種集合真にが必要な場合は、将来 trait object サポートを追加可能
```

### Q: これは Rust の Trait と何が違うのですか？

A: 本質的には類似していますが：

- `impl` キーワードがない
- 明示的な宣言要求がない（オプション）
- ジェネリックコンテキスト内でのみ使用

### Q: 制約にはデータフィールドを含めることができますか？

A: はい。YaoXiang の制約はメソッドに限定されません：

```yaoxiang
type HasPosition = {
    x: Int,
    y: Int
}

move: [T: HasPosition](item: T, dx: Int, dy: Int) -> T = (item, dx, dy) => {
    # item.x と item.y が存在しなければならない
    item
}
```

---

## 十、関連ドキュメント

- [RFC-010 統一型構文](../design/accepted/010-unified-type-syntax.md)
- [RFC-011 ジェネリックシステム](../design/accepted/011-generic-type-system.md)