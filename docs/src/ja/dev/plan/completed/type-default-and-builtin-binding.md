# タイプデフォルト値とビルtrin绑定実装計画

> **状態：コア実装済み** — Phase 1-4 コアフレームワーク + レガシー項目強化が完了し、全 1499 テストがパス。

## 概要

本計画では YaoXiang 言語の2つのコア機能を実装し、RFC-004（カリー化多位置bind）と RFC-010（統一タイプ構文）に対応する：

1. **タイプデフォルト値初期化**（RFC-010）：タイプフィールドがデフォルト値を지원し、構築時に任意提供可能
2. **ビルtrin bind**（RFC-004）：タイプ定義体内で直接メソッドをbind（外部関数または無名関数参照）、位置インデックス精确制御パラメータbindをサポート

### コア構文（RFC-010 統一モデル）

```yaoxiang
Point: Type = {
    x: Float = 0,                                    # フィールド + デフォルト値
    y: Float = 0,                                    # フィールド + デフォルト値
    distance = distance[0],                          # 外部関数bind（RFC-004 位置構文）
    norm: ((p: Point) -> Float)[0] = ((p) => ...)    # 無名関数bind
}
```

### コアデータ構造変更

```
タイプ定義体フィールド：
├── フィールド宣言：field: Type
├── フィールドデフォルト値：field: Type = expression          (RFC-010)
├── 外部関数bind：field = function[position]                (RFC-004)
└── 無名関数bind：field: ((params) -> Return)[position] = ((params) => body)  (RFC-004)
```

### 関連モジュール

| モジュール | ファイル | 責務 |
|------|------|------|
| AST | `ast.rs` | `StructField`, `TypeBodyBinding`, `BindingKind` |
| パーサー | `declarations.rs` | `parse_struct_type()` - 4種類のフィールドタイプ解析 |
| 型システム | `mono.rs` | `StructType.field_has_default` |
| 意味解析 | `checking/mod.rs` | `check_type_def`, `check_field_default`, `check_field_binding` |
| IR 生成 | `ir_gen.rs` | `CreateStruct`, デフォルト値塗りつぶし, bindメソッド呼出转发 |
| バイトコード | `bytecode.rs` | `CreateStruct` バイトコードエンコード/デコード |
| ランタイム | `executor.rs` | `CreateStruct` 実行 |

---

## 実装手順

### Phase 1: パーサー強化 ✅

#### 1.1 タイプフィールド AST 拡張

**目標**：通常フィールド、デフォルト値フィールド、bindフィールドを区別するための新規フィールドタイプ

**受入方案**：
- [x] `Point: Type = { x: Float = 0 }` を解析して正しい AST を生成
- [x] `Point: Type = { distance = distance[0] }` を解析して Binding ノードを生成
- [x] `Point: Type = { distance: ((a, b) => Float)[0] = ((a, b) => a + b) }` を解析して AnonBinding ノードを生成

**実装説明**：
- `StructField` に新規 `default: Option<Box<Expr>>` フィールドを追加
- 新規 `TypeBodyBinding` 構造体と `BindingKind` 列挙型を追加（`External` / `Anonymous`）
- `Type::Struct` をタプルvariantから構造体variant `Type::Struct { fields, bindings }` に変更
- `parse_struct_type()` を完全に書き直し、4種類のフィールドタイプ解析をサポート
- 新規ヘルパー関数：`parse_optional_binding_positions()`, `parse_binding_positions()`, `extract_fn_type_info()`

---

### Phase 2: 意味解析強化 ✅

#### 2.1 デフォルト値フィールドタイプ検査

**目標**：デフォルト値式 타입とフィールドタイプの一致を検証

**受入方案**：
- [x] `x: Float = 0` がタイプ検査を通過（Int → Float 暗黙数値昇格をサポート）
- [x] `x: Float = "str"` がタイプ不一致を報告（String ≠ Float）
- [x] `x: Int = 1.0` が報告（Float を Int に代入不可、逆方向昇格不支持）

**実装説明**：
- `BodyChecker::check_type_def()` → 構造体フィールドとbindを巡回して検査
- `check_field_default()` → デフォルト値式に対してタイプ導出を実行し、フィールドタイプと統一
  - Int → Float 暗黙数値昇格をサポート（RFC-010 `x: Float = 0` 使い方に一致）
  - その他タイプ不一致（String → Float, Float → Int）は `type_mismatch` エラーを報告
- `StructType` に新規 `field_has_default: Vec<bool>` フィールドを追加、すべてのタイプ変換箇所で伝播

#### 2.2 bindフィールド意味検査（RFC-004 タイプ安全）

**目標**：bind参照先の関数が存在すること、位置インデックスが有効であること、タイプが一致することを検証

**受入方案**：
- [x] `distance = distance[0]` が `distance: (a: Point, b: Point) -> Float` に対して検証を通過
- [x] `distance = distance[5]` が2パラメータ関数に対してインデックス範囲外を報告
- [x] `distance = distance[0]` が `distance: (a: String, b: String)` に対してタイプ不一致を報告
- [x] 位置インデックスリストが空でないことを検証

**実装説明**：
- `check_field_binding(type_name, binding, span)` が検証用タイプ名を受け取る
- 外部bind検証フロー（RFC-004 §タイプ検査規則）：
  1. 位置インデックスリストが空でないことを検証
  2. 関数の多相タイプを検索、単相関数タイプにインスタンス化
  3. 各位置インデックス < 関数パラメータ数を検証
  4. bind位置のパラメータタイプと現在タイプが互換であることを検証（`unify` 経由）
- 関数が現在スコープに見つからない場合、深度検査をスキップ（関数は外層または後続で定義されている可能性）

#### 2.3 無名関数bind意味検査

**目標**：無名関数bindのパラメータ位置とタイプを検証

**受入方案**：
- [x] 位置インデックスが空でないことを検証
- [x] 位置インデックスが無名関数パラメータ範囲内であることを検証
- [x] bind位置パラメータタイプと現在タイプが一致することを検証

**実装説明**：
- 無名bind検証：位置が空でない + 位置 < パラメータ数 + bind位置パラメータタイプとタイプ名 `unify`

---

### Phase 3: コード生成強化 ✅

#### 3.1 デフォルト値初期化コード生成

**目標**：デフォルトコンストラクタとデフォルト値overrideコンストラクタ呼出を生成

**受入方案**：
- [x] `Point()` がデフォルト値呼出コードを生成
- [x] `Point(1.0)` が x のみoverride、y はデフォルト値を使用
- [x] `Point(1.0, 2.0)` が全フィールドをoverride

**実装説明**：
- 新規 `CreateStruct` IR 命令（`ir.rs`）とバイトコード命令（`bytecode.rs`）を追加
- 新規 `Opcode::CreateStruct = 0x79`
- `generate_struct_constructor_ir()` を書き直し：全パラメータ読込 → `CreateStruct` → `Ret`
- 呼出側デフォルト値塗りつぶし：`generate_expr_ir` の `Expr::Call` 分岐で、構造体コンストラクタ呼出を検出し、不足パラメータに対してデフォルト値式 IR を生成
- `translate_create_struct()` トランスレーター + バイトコードデコーダー実装
- インタープリタが `CreateStruct` で `HeapValue::Tuple` を割り当て `RuntimeValue::Struct` を作成

#### 3.2 bindメソッドコード生成（RFC-004 パラメータ並べ替え）

**目標**：bindメソッド呼出に対して正しい関数呼出转发を生成

**受入方案**：
- [x] `p1.distance(p2)` + `distance = distance[0]` → `distance(p1, p2)` 呼出を生成
- [x] `p1.distance(p2)` + `distance = distance[1]` → `distance(p2, p1)` 呼出を生成
- [x] 多位置bind `transform = transform[0, 1]` が正しく转发

**実装説明**：
- 新規 `BindingInfo` 構造体（元関数名 + bind位置リストを記録）
- 新規 `type_bindings: HashMap<String, HashMap<String, BindingInfo>>`（タイプ名 → メソッド名 → bind情報）
- `register_type_bindings()` がコンストラクタ IR 生成フェーズで `Type::Struct { bindings }` からbind情報を抽出
- メソッド呼出 IR 生成強化（`Expr::Call` + `FieldAccess` 分岐）：
  1. オブジェクトタイプ名を導出（`get_expr_struct_type_name()` 経由）
  2. そのタイプのbind情報を検索
  3. bindがある場合：RFC-004 位置規則に従ってパラメータを並べ替え
     - サイズ `total_params = positions.len() + method_args.len()` のパラメータスロットを作成
     - obj をbind位置に配置
     - メソッドパラメータを順序대로残りの位置に填充
  4. 元の関数名（非メソッド名）で呼出
- 無名関数bindは `タイプ名.__anon_メソッド名` 命名規則を使用

#### 3.3 フィールドインデックス動的解決

**目標**：`resolve_field_index()` がタイプ情報から動的にフィールドインデックスを検索

**受入方案**：
- [x] ハードコードに依存しなくなった（元 x→0, y→1, z→2）
- [x] `struct_definitions` からフィールド名で精确検索

**実装説明**：
- `resolve_field_index(expr, field_name)` を書き直し：
  1. `get_expr_struct_type_name(expr)` 経由で式の構造体タイプ名を導出
  2. `struct_definitions` からそのタイプのフィールドリストを検索、フィールド名をマッチしてインデックスを返す
  3. フォールバック：タイプ導出が利用できない場合、全構造体定義を巡回してフィールド名を検索
- 新規 `get_expr_struct_type_name(expr)` ヘルパーメソッド：
  - 変数：`type_result.local_var_types`、`bindings`、`local_var_types` から検索
  - コンストラクタ呼出：`Point(...)` から直接 `"Point"` を返す
- 新規 `mono_type_to_struct_name(mono_type)` ヘルパーメソッド：
  - `MonoType::TypeRef(name)` → `Some(name)`
  - `MonoType::Struct(st)` → `Some(st.name)`

---

### Phase 4: ランタイムサポート ✅

#### 4.1 デフォルト値式評価

**目標**：ランタイムでデフォルト値式を正しく評価

**受入方案**：
- [x] 简单リテラルデフォルト値 `0`, `"hello"` が正しく評価
- [x] 式デフォルト値 `x: Int = 1 + 2` が3として正しく評価

**実装説明**：
- デフォルト値は呼出側 IR 生成フェーズで評価（`generate_expr_ir` でデフォルト値式を処理）
- 任意式をデフォルト値としてサポート（リテラル、算術式、関数呼出など）
- インタープリタが `CreateStruct` バイトコード経由で実際の構造体作成とフィールド初期化を実行

---

## テスト計画

### 单元テスト

| テストカテゴリ | テスト内容 | 状態 |
|----------|----------|------|
| 解析テスト | 各フィールド構文解析 | ✅ |
| タイプ検査 | デフォルト値タイプ一致（Int→Float昇格포함） | ✅ |
| タイプ検査 | bind位置有効性（範囲外、タイプ一致） | ✅ |
| タイプ検査 | 無名関数bind位置検証 | ✅ |
| コード生成 | デフォルト値生成 | ✅ |
| コード生成 | bindメソッド呼出转发 | ✅ |

### 統合テスト

| テストケース | 期待結果 | 状態 |
|----------|----------|------|
| `Point: Type = { x: Float = 0, y: Float = 0 }` + `Point()` | 構築成功 | ✅ |
| `Point(1.0, 2.0)` 位置パラメータ構築 | フィールド正しく代入 | ✅ |
| `Point(1.0)` 部分パラメータ + デフォルト値 | x=1.0, y=0 | ✅ |
| bindメソッド呼出（RFC-004 位置並べ替え） | パラメータ正しく转发 | ✅ |

### 回帰テスト

- [x] 全 1464 lib テストがパス
- [x] 全 30 integration テストがパス
- [x] 全 5 runtime テストがパス
- 既存のタイプ定義構文に影響なし
- 既存のbind構文に影響なし
- 既存のコンストラクタ呼出に影響なし

---

## リスクと依存

### 依存

- **RFC-004**（カリー化多位置bind）：`[position]` 位置bind構文、パラメータ並べ替え規則
- **RFC-010**（統一タイプ構文）：`name: Type = { ... }` 統一モデル、フィールドデフォルト値構文

### リスク

| リスク | 影響 | 緩和策 |
|------|------|------|
| 解析曖昧性 | `field = value` が代入またはbindの可能性 | `=` の右辺構文で区別（関数参照+位置 vs Lambda） |
| タイプ導出不完了 | `resolve_field_index` フォールバック巡回の精度問題 | タイプ検査結果を優先 |

---

## マイルストーン

1. **M1**: ✅ パーサーが全フィールドタイプをサポート（デフォルト値フィールド、外部bind、無名bind）
2. **M2**: ✅ 意味検査フィールドルール（デフォルト値タイプ検査+数値昇格、bind位置範囲外+タイプ一致検証）
3. **M3**: ✅ コード生成がデフォルト値をサポート（`CreateStruct` 命令、呼出側デフォルト値填充）
4. **M4**: ✅ bindメソッドコード生成（RFC-004 パラメータ並べ替え、`BindingInfo` + `type_bindings` マッピング）
5. **M5**: ✅ フィールドインデックス動的解決（`resolve_field_index` が `struct_definitions` から検索）

### 后续任意強化

以下機能はすべて実装済み（テスト覆盖は `binding_enhancements` テストモジュール内）：

| 機能 | 対応 RFC | 状態 | 説明 |
|------|---------|------|------|
| 名前付きパラメータ構築 | RFC-010 | ✅ 実装済み | `Point(x=1, y=2)` パーサーが `name=expr` パターンを検出し `named_args` に分離；IR 生成器がフィールド名でパラメータを並べ替え |
| 負数インデックスbind | RFC-004 | ✅ 実装済み | `func[-1]` 位置タイプを `Vec<i64>` に変更、パーサーが `Minus` + `IntLiteral` パターンをサポート |
| デフォルトbind | RFC-004 | ✅ 実装済み | `Type.method = function`（位置なし）が `BindingKind::DefaultExternal` を生成、IR 生成器がデフォルト位置 0 を使用 |
| 外部bind文 | RFC-004 | ✅ 実装済み | `Point.distance = distance[0]` を `StmtKind::ExternalBindingStmt` として解析、IR 生成器が `type_bindings` に登録 |
| インターフェース制約 | RFC-010 | ✅ 実装済み | `StructType` に新規 `interfaces: Vec<String>` フィールドを追加、パーサーが大文字識別子をインターフェース制約として認識 |
| 無名関数 IR 生成 | RFC-004 | ✅ 実装済み | `generate_anon_binding_ir` メソッドが独立した `FunctionIR` を生成、モジュールレベル `anon_function_irs` に登録 |