# コンパイル時計算エンジン（CTE Engine）とホア論理静的検証 — 実装計画

> **タスク**：コンパイル時評価エンジン + ホア論理静的検証チャネルを実装し、値依存型とコンパイル時次元検証を支える
> **RFC ベース**：RFC-010（統一型構文）、RFC-011（ジェネリックシステム/値依存型）、RFC-022（ホア論理静的検証）
> **日付**：2026-05-10
> **ステータス**：設計中
> **目標マイルストーン**：
> - M1：定数畳み込み + 純粋性分析スケルトン
> - M2：純粋関数のコンパイル時評価 + 終了証明
> - M3：型レベル計算（`If`/`Assert`/`match` 型族）
> - M4：ホア論理検証（`//!` 仕様解析 → VC 生成 → SMT 接続）

---

## 概要

YaoXiang の値依存型（RFC-011）は、型がコンパイル時に既知の値に依存できること要求する（例：`Vec(factorial(5))` → `Vec(120)`）。また、ホア論理静的検証（RFC-022）は純粋関数に対するコンパイル時仕様チェックを要求する。両者に共通する中核要件は：**コンパイル時に安全に純粋関数を実行/分析する**こと。

本計画は**統一コンパイル時計算エンジン（CTE Engine）**を提案し、純粋性分析、終了証明、式評価を共通インフラとして抽象化し、型レベル評価とホア論理検証という2つのコンシューマにそれぞれサービスを提供する。

---

## 中核設計原則

1. **所有権システム复用による純粋性分析**：YaoXiang の `&mut` は副作用の印である——借用検査が何を修正するかを既に教えてくれる
2. **純粋関数 = コンパイル時に評価可能**：`const fn` キーワード不要、编译器が自動的に純粋性を推断
3. **終了証明 = 型安全なiftonの基石**：型位置での評価は終了を証明必須（`decreases` 仕様）、否则型システムが決定不能になる
4. **部分評価优于全量評価**：いくつのパラメータが既知かによって評価し、実行時までに計算できないものはそのまま残す
5. **コンパイル時評価とホア論理の共有インタープリタ**：同一个式評価核心が两类コンシューマを支える
6. **双モード評価**：具体評価（既知パラメータ → `CTValue` 生成）と記号評価（未知パラメータ → `SMTExpr` 生成）は同一个インタープリタフレームワークを共有し、評価環境のみが異なる

---

## アーキテクチャ概要

```
                           ソースファイル + //! 仕様
                                    ↓
                           ┌──────────────┐
                           │   パーサー    │
                           │ (//! コメント│
                           │  認識)        │
                           └──────┬───────┘
                                  ↓
                           ┌──────────────┐
                           │  型チェック器 │
                           │ • 仕様の収集  │
                           │ • 値依存発見  │
                           └──────┬───────┘
                                  ↓
              ┌────────────────────┴────────────────────┐
              ↓                                         ↓
   ┌──────────────────────┐                ┌──────────────────────┐
   │   CTE エンジン        │                │  ホア論理検証器       │
   │                      │                │                      │
   │  ┌────────────────┐  │                │  1. //! 仕様の収集     │
   │  │  純粋性分析器    │  │                │  2. 検証条件(VC)生成  │
   │  │  (所有権ベース)  │  │                │  3. SMT ソルバー(Z3)   │
   │  └───────┬────────┘  │                │  4. 反例レポート       │
   │  ┌───────┴────────┐  │                └──────────┬───────────┘
   │  │  終了チェッカー  │  │                           │
   │  │  (decreases)    │  │                           ↓
   │  └───────┬────────┘  │                ┌──────────────────────┐
   │  ┌───────┴────────┐  │                │  検証結果            │
   │  │  AST インタープリタ│ │                │  • 通過 → キャッシュ │
   │  │                │  │                │  • 失敗 → release   │
   │  │ ┌────────────┐ │  │                │    ブロック          │
   │  │ │ 具体評価    │ │  │                └──────────────────────┘
   │  │ │ env: 全既知 │  │  │                       ↑
   │  │ │ → CTValue  │  │  │                       │
   │  │ └────────────┘ │  │  共有インタープリタ     │
   │  │ ┌────────────┐ │  │  フレームワーク          │
   │  │ │ 記号評価    │─┼──┼───────────────────────┘
   │  │ │ env: 部分既知│ │  │  (ホア論理コンシューマ)
   │  │ │ → SMTExpr  │ │  │
   │  │ └────────────┘ │  │
   │  └───────┬────────┘  │
   │          ↓           │
   │  型/単相化に結果を埋め込む│
   └──────────────────────┘
```

---

## 一、コンパイル時値（CTValue）

コンパイル時計算の中核データ型。コンパイラ内部IR値として設計され、実行時値と混同しない。

```rust
/// コンパイル時評価結果
enum CTValue {
    /// 整数（Bool の全コンパイル時用途をカバー）
    Int(i64),

    /// 浮動小数点数
    Float(f64),

    /// 文字列（エラーメッセージ、型名など）
    String(SmolStr),

    /// 型参照——型レベル計算の中核
    /// YaoXiang の型自身が Type1 層の「値」
    Type(TypeId),

    /// 異種タプル
    Tuple(Vec<CTValue>),

    /// 同種配列
    Array(Vec<CTValue>),

    /// 構造化値
    Struct {
        type_id: TypeId,
        fields: HashMap<SmolStr, CTValue>,
    },

    /// 未評価の関数参照（部分評価時に保持）
    /// 全パラメータ既知時にインライン評価、そうでなければ実行時呼び出しとして保持
    Thunk {
        func: FunctionId,
        known_args: Vec<CTValue>,
        unknown_params: Vec<ParamId>,
    },
}
```

**重要な設計**：`CTValue::Type(TypeId)` により型が первой コンパイル時値となる。`If(C, T, E)` の C を評価すると `CTValue::Bool`、T/E を評価すると `CTValue::Type`。

---

## 二、サブシステム 1：純粋性分析器

### 2.1 設計思路

YaoXiang の所有権システム（RFC-009）を复用し、副作用は 型签名によって自然に表現される：

| パラメータモード | 意味 | コンパイル時評価可能？ |
|----------|------|---------------|
| `x: T` (所有権取得) | 所有権を取得、自由に変更可能 | ✅ |
| `x: &T` (共有参照) | 読み取り専用 | ✅ |
| `x: &mut T` (独占参照) | 変更可能 | ⚠️ T の出所による |
| I/O 呼び出し | 外部副作用 | ❌ |
| 非純粋関数の呼び出し | 推移的 | ❌ |

### 2.2 アルゴリズム

```
analyze_purity(func: FunctionId, ctx: &mut PurityContext) -> PurityResult:
    // 1. 高速パス：既にアノテーション済み
    if ctx.has_purity_annotation(func):
        return ctx.get_annotation(func)

    // 2. 直接的な副作用をチェック
    for op in func.body.operations():
        match op:
            Call(callee, _) if is_io_operation(callee):
                return Impure("I/O 操作")
            Call(callee, args) where has_mut_arg(args):
                if arg_escapes_function(args):
                    return Impure("&mut による外部状態の変更")
            Call(callee, _):
                // 推移性：被呼び出し関数も純粋関数である必要がある
                if analyze_purity(callee, ctx).is_impure():
                    return Impure("不純関数を呼び出し: {callee}")

    // 3. デフォルトで純粋関数と推断
    return Pure
```

### 2.3 明示的な純粋性アノテーションを提供しない

**設計判断：明示的な純粋性アノテーション（`//! pure` など）は提供しない。**

所有権システム（RFC-009）は既に型签名を通じて副作用情報を表現している——`&mut T` は変更、`I/O` 操作は外部副作用である。コンパイラは純粋性を自動的に推断有能力である。

コンパイラが純粋関数を不純と誤判断した場合は、それはコンパイラのバグであり、コンパイラを修正すべきであり、ユーザーがパッチを打つべきではない。「この関数は純粋だと信じろ」というアノテーションを提供することは、真の問題を隠蔽するだけである。

> *「互換性、フォールバック、一時的、バックアップ、特定モードでのみ有効なコードを書くな。問題を直接露出させろ。」*

### 2.4 RFC-022 との関係

純粋性分析器は両方にサービスする：

- **CTE**：非純粋関数は型位置で使用不可
- **ホア論理**：仕様式（`requires`/`ensures` の右辺）は純粋関数呼び出し必须是

---

## 三、サブシステム 2：終了チェッカー

### 3.1 設計思路

型位置でのコンパイル時評価は終了を保証必须であり、そうでなければ型システムが決定不能になる。YaoXiang は `//! decreases` 仕様で終了を証明する。

```
//! decreases: <expr>
```

ここで `<expr>` は下限を持つ整列値（下限制約を持つ well-founded 値，通常は自然数を表す `Int` 型）。

### 3.2 アルゴリズム

```
check_termination(func: FunctionId, ctx: &mut TermContext) -> TermResult:
    // 1. decreases 仕様を検索
    let decreases_expr = find_decreases_spec(func)
        .or_else(|| infer_decreases(func))

    match decreases_expr:
        None if has_recursive_call(func):
            return TermError::NoDecreasesAnnotation
        None:
            return TermOk  // 再帰なし、証明不要

        Some(decreases):
            // 2. 各再帰呼び出し点で検証
            for call in func.recursive_calls():
                let dec_at_call = eval_decreases_at(call, decreases)
                let dec_at_entry = eval_decreases_at(func.entry, decreases)

                if !strictly_less_than(dec_at_call, dec_at_entry):
                    return TermError::NotDecreasing {
                        at: call.location,
                        expected_less_than: dec_at_entry,
                        actual: dec_at_call,
                    }

            // 3. 下限制約を検証
            if !has_lower_bound(decreases):
                return TermError::NoLowerBound

            return TermOk
```

### 3.3 自動推断

一部の明確な終了ケースはアノテーション不要：

```yaoxiang
// decreases 不要——コンパイラはループが既知の上界 n を持つことを確認
sum: (arr: Array(Int, n)) -> Int = {
    s = 0; i = 0
    while i < n { s += arr[i]; i += 1 }
    return s
}
```

アノテーションが必要なケース：
```yaoxiang
// decreases 必须——再帰呼び出し n-1
factorial: (n: Int) -> Int = {
    //! decreases: n
    if n <= 1 { return 1 }
    return n * factorial(n - 1)
}
```

### 3.4 RFC-022 との関係

終了チェッカーは両方にサービスする：

- **CTE**：decreases はコンパイル時評価の参入门槛
- **ホア論理**：ループ不変式の decreases バリアント（`/*! decreases: n - i !*/`）も終了チェッカーが検証

---

## 四、サブシステム 3：AST インタープリタ

### 4.1 設計思路

インタープリタは AST 走査を基础とし、評価環境（変数名 → CTValue マッピング）を維持する。中核能力は**部分評価**：既知パラメータは計算し、未知パラメータは保持する。

```
eval(expr: &Expr, env: &mut EvalEnv) -> EvalResult<CTValue>:
    match expr:
        // 字面量 → 直接変換
        Literal(lit) => lit.into_ctvalue()

        // 変数 → 環境を検索
        Variable(name) => env.get(name).ok_or(NotInScope)

        // 二項演算
        BinaryOp(l, op, r) =>
            let lv = eval(l, env)?; let rv = eval(r, env)?
            apply_op(op, lv, rv)

        // 条件分岐
        If(cond, then, else) =>
            match eval(cond, env)? {
                Bool(true) => eval(then, env),
                Bool(false) => eval(else, env),
                _ => Err(ExpectedBool),
            }

        // 関数呼び出し——中核ロジック
        Call(func, args) =>
            let known_args = args.filter_map(|a| eval(a, env).ok())
            if known_args.len() == args.len():
                // 全パラメータ既知 → インライン評価
                inline_and_eval(func, known_args, env)
            else if known_args.len() > 0:
                // 部分既知 → 部分評価（単相化コードを生成）
                partial_eval(func, known_args, env)
            else:
                // 全部未知 → Thunk
                CTValue::Thunk { func, known_args: vec![], unknown_params: args }

        // パターンマッチ
        Match(scrutinee, arms) =>
            let val = eval(scrutinee, env)?
            for arm in arms:
                if arm.pattern.matches(val):
                    return eval(arm.body, env.with_bindings(arm.bindings))
            Err(NoMatch)

        // コードブロック
        Block(stmts) =>
            if stmts.is_empty():
                return CTValue::Void
            for stmt in stmts[..len-1]:
                eval(stmt, env)?
            eval(stmts.last(), env)

        // ループ
        While(cond, body) =>
            let mut result = CTValue::Void
            while eval(cond, env)? == Bool(true):
                check_step_limit()?  // 無限ループを防止
                result = eval(body, env)?
            result
```

### 4.2 インライン評価

全パラメータが既知の場合、インタープリタは関数体を評価コンテキストにインライン化する：

```
inline_and_eval(func, args, env):
    // 1. 純粋性をチェック
    purity.check(func)?

    // 2. キャッシュをチェック
    if let Some(cached) = cache.get(func, args):
        return cached

    // 3. インライン環境を作成
    let mut inline_env = env.child()
    for (param, arg) in func.params.zip(args):
        inline_env.bind(param.name, arg)

    // 4. 関数本体を評価
    let result = eval(&func.body, &mut inline_env)?

    // 5. 結果をキャッシュ
    cache.insert(func, args, result.clone())
    result
```

### 4.3 ステップ数制限

コンパイル時評価にはハード制限が必要であり、`decreases` があっても予期せぬタイムアウトを防止する：

```
const MAX_EVAL_STEPS: u64 = 1_000_000;  // 100万ステップのハード上限

struct EvalEnv {
    variables: HashMap<SmolStr, CTValue>,
    step_count: u64,
    step_limit: u64,
}
```

### 4.4 双モード評価：具体 vs 記号

インタープリタの中核フレームワーク（AST 走査、インライン展開、パターンマッチ）は統一だが、**評価環境**が2つのモードを決定する：

#### 4.4.1 具体評価（Concrete Evaluation）

**コンシューマ**：CTE エンジン → 型レベル評価、単相化

**特徴**：

- 環境の全変数が具体的な `CTValue` を持つ
- 関数呼び出しパラメータがすべて既知 → インライン評価
- 出力：`CTValue`（具体値または型参照）
- 失敗 = コンパイルエラー

```
// シナリオ：Vec(factorial(5))
// env = { factorial → Function(...) }
eval(Call("factorial", [Literal(5)]), env):
    → inline_and_eval(factorial, [CTValue::Int(5)], env)
    → CTValue::Int(120)
// 型置換：Vec(120)
```

#### 4.4.2 記号評価（Symbolic Evaluation）

**コンシューマ**：ホア論理検証器 → SMT 求解

**特徴**：

- 環境に**記号変数**が存在（関数パラメータ `n`、`arr`、コンパイル時未知）
- 既知の部分式は具体値に評価され、未知部分は SMT 記号として保持
- 関数呼び出しはインライン化されない——代わりに論理式に展開される
- 出力：`SMTExpr`（一階述語論理式）、Z3 に渡す
- 失敗 = 検証失敗（非コンパイルエラー）

```
// シナリオ：max の ensures を検証:
//   //! ensures: GreaterOrEqual(result, arr[0..n]) = result >= forall arr[i]
// env = { result → Symbol("result"), arr → Symbol("arr"), n → Symbol("n") }
eval(BinaryOp(Variable("result"), GtEq, Call("arr_max", [Symbol("arr"), Symbol("n")]))):
    // result は記号 → 保持
    // arr_max(arr, n) は純粋関数だがパラメータ未知 → 論理定義に展開
    → SMTExpr::Forall(i in 0..n, Symbol("result") >= Symbol("arr")[i])
// Z3 に渡す：∀arr, n, result. (n > 0 ∧ ...) → result >= arr[0] ∧ ... ∧ result >= arr[n-1]
```

#### 4.4.3 2つのモードの重要な違い

| 次元 | 具体評価 | 記号評価 |
|------|----------|----------|
| 環境 | `HashMap<Name, CTValue>` | `HashMap<Name, SMTTerm>` |
| 変数が未知のとき | エラー | 記号として保持 |
| 関数呼び出し | インライン化 + 関数本体実行 | 論理定義に展開（実行しない） |
| ループ | 実際の反復（ステップ数制限付き） | ループ不変式 VC に変換 |
| 出力タイプ | `Result<CTValue, CTError>` | `Result<SMTExpr, SMError>` |
| 失敗のセマンティクス | コンパイルエラー | 検証失敗（Runtime Check にデグレード可能） |
| 性能特性 | 高速（直接計算） | 低速（SMT 求解） |

#### 4.4.4 共有インタープリタフレームワーク

両モードが同じ AST 走査スケルトンを共有：

```rust
/// インタープリタ trait：具体評価と記号評価が各自実装
trait Interpreter {
    type Value;       // CTValue または SMTExpr
    type Error;       // CTError または SMError

    fn eval_literal(&mut self, lit: &Literal) -> Result<Self::Value, Self::Error>;
    fn eval_variable(&mut self, name: &str) -> Result<Self::Value, Self::Error>;
    fn eval_binary_op(&mut self, op: BinOp, l: Self::Value, r: Self::Value) -> Result<Self::Value, Self::Error>;
    fn eval_call(&mut self, func: FunctionId, args: &[Expr]) -> Result<Self::Value, Self::Error>;
    fn eval_if(&mut self, cond: &Expr, then: &Expr, else_: &Expr) -> Result<Self::Value, Self::Error>;
    fn eval_match(&mut self, scrutinee: &Expr, arms: &[MatchArm]) -> Result<Self::Value, Self::Error>;
    fn eval_while(&mut self, cond: &Expr, body: &Expr) -> Result<Self::Value, Self::Error>;
}

/// 統一 AST 走査器、具象実装に委譲
fn eval_ast<I: Interpreter>(interp: &mut I, expr: &Expr) -> Result<I::Value, I::Error> {
    match expr {
        Expr::Literal(lit) => interp.eval_literal(lit),
        Expr::Variable(name) => interp.eval_variable(name),
        Expr::BinaryOp { op, left, right } => {
            let l = eval_ast(interp, left)?;
            let r = eval_ast(interp, right)?;
            interp.eval_binary_op(*op, l, r)
        }
        Expr::Call { func, args } => interp.eval_call(*func, args),
        Expr::If { cond, then, else_ } => interp.eval_if(cond, then, else_),
        // ... 残りの AST ノードも同理
    }
}
```

**重要な洞察**：具体評価と記号評価の AST 走査ロジックは完全に同じであり、異なるのは：

- **値をどう表現するか**（`CTValue` vs `SMTExpr`）
- **関数呼び出しをどう处理するか**（インライン実行 vs 論理展開）
- **未知変数をどう处理するか**（エラー vs 記号保持）

---

## 五、CTE と型システム/単相化との統合

### 5.1 型レベル計算

CTE エンジンが型システムに提供するサービス：

1. 型注釈位置
   `Vec(factorial(5))` → CTE::eval(factorial(5)) → CTValue::Int(120)
   型置換を Vec(120)

2. ジェネリック値パラメータ
   `Array(Int, factorial(3))` → CTE::eval(factorial(3)) → CTValue::Int(6)
   インスタンス化 `Array(Int, 6)`

3. Assert 型
   `Assert(N > 0)` → CTE::eval(N > 0) → CTValue::Bool(true/false)
   True → Void、False → compile_error("N must be > 0")

4. If 条件型
   `If(C, T, E)` → CTE::eval(C) → CTValue::Bool(b)
   True → T、False → E

5. Match 型族
   `AsString(Int)` → match Int { Int => String, ... } → String

### 5.2 単相化와의 相互作用

単相化は CTE 結果を以下で使用する：

```
1. 既知のジェネリック値パラメータ → 具象インスタンスを生成
   List(Int) の push メソッド → push_List_Int を生成

2. 既知の値依存型 → 具象型に展開
   Matrix(Float, 3, 3).data → Array(Array(Float, 3), 3)

3. 部分評価 → 単相化コードを生成
   map(Int, String) → map_Int_String を生成（T=Int、R=String は固定済み）
```

### 5.3 ホア論理検証器와의 相互作用

検証器は CTE を以下で使用する：

```
1. 仕様式の一部評価
   //! requires: n > 0 && factorial(n) < MAX
   CTE::eval(factorial(n)) → n がコンパイル時既知 → 定数
                            → n が未知 → 記号として保持、SMT に渡す

2. 仕様条件の簡略化
   //! ensures: result >= 0 && result < n
   CTE は既知の部分式を簡略化 пытається 軽減し、SMT 求解の負担を減らす

3. 仕様型のインスタンス化
   NonEmpty(n) = n > 0
   CTE は仕様型をブール式に展開
```

---

## 六、ホア論理静的検証（RFC-022 実装設計）

### 6.1 仕様解析

`//!` と `/*! ... !*/` はパーサーによって特殊コメントノードとして認識され、AST に附加される：

```rust
struct SpecAnnotation {
    kind: SpecKind,        // Requires | Ensures | Invariant | Decreases
    name: Option<SmolStr>, // 仕様名（オプションのユーザー命名）
    spec_type: TypeExpr,   // 仕様型式
    expr: Expr,            // ブール式
    span: Span,
}

enum SpecKind {
    Requires,
    Ensures,
    Invariant,
    Decreases,
}
```

### 6.2 検証条件生成（VCGen）

最弱前置条件（Weakest Precondition）計算を採用：

```
generate_vc(func: FunctionId) -> Vec<VerificationCondition>:
    let requires = collect_requires(func)
    let ensures = collect_ensures(func)
    let invariants = collect_invariants(func)
    let decreases = collect_decreases(func)

    let mut vcs = Vec::new()

    // VC1: 前置条件の一貫性
    vcs.push(VC::PreconditionConsistency(requires))

    // VC2: 後置条件の検証（各実行パスに対して）
    for path in func.paths():
        let wp = compute_wp(path.body, ensures)
        vcs.push(VC::Postcondition {
            path: path.id,
            formula: implies(requires, wp),
        })

    // VC3: ループ不変式
    for (loop_, invariant) in invariants:
        // ループ突入前に成立
        vcs.push(VC::InvariantEntry { loop_, invariant })
        // 各反復で維持
        vcs.push(VC::InvariantPreservation { loop_, invariant })
        // 退出後に後置条件を蕴含
        vcs.push(VC::InvariantExit { loop_, invariant, post: ensures })

    vcs
```

### 6.3 SMT ソルバー統合

```
┌─────────────┐     SMT-LIB フォーマット      ┌───────────┐
│  VC Generator │ ────────────────────────→ │  Z3 ソルバー│
└─────────────┘                               └─────┬─────┘
                                                    │
                          ┌─────────────────────────┴──────────────┐
                          ↓                                ↓
                      unsat                            sat
                          ↓                                ↓
                    ┌──────────┐                   ┌──────────────┐
                    │ 検証通過  │                   │ 反例モデル抽出 │
                    │ 結果キャッシュ│                   │ 人間可読形式に変換│
                    └──────────┘                   └──────┬───────┘
                                                         ↓
                                                  ┌──────────────┐
                                                  │ コンパイルエラー│
                                                  │ レポート       │
                                                  │ • 入力値       │
                                                  │ • 違反した仕様  │
                                                  └──────────────┘
```

### 6.4 コンパイルモード

| モード | 動作 | CLI |
|------|------|-----|
| **Debug Build** | 仕様を解析し、VC を生成し、Z3 で証明；検証通過後にのみ Release Build 可能 | `yaoxiangc --debug` |
| **Release Build** | 全 `//!` コメントを無視、ゼロオーバーヘッド、検証キャッシュをクリア | `yaoxiangc --release` |
| **Runtime Checks** | 仕様を `assert` 文にデグレード、違反時に panic | `yaoxiangc --runtime-checks` |

---

## 七、実装フェーズ

### Phase 1：定数畳み込み + 純粋性分析スケルトン

**目標**：CTE インフラを確立し、もっとも基本的なコンパイル時評価をサポート

**内容**：

- [ ] `CTValue` 列挙型と `EvalEnv` 構造体を定義
- [ ] `eval()` の基本パスを実装：字面量、変数、二項演算、条件、コードブロック
- [ ] 純粋性分析器の第一版を実装：I/O 呼び出しを不純と識別し、他はデフォルトで純粋
- [ ] 型チェック器に CTE 呼び出し点を挿入（型注釈位置）
- [ ] 定数畳み込み：`1 + 2 * 3` をコンパイル時に `7` に計算
- [ ] デッドブランチ除去：`if true { ... } else { ... }` → then ブランチを直接採用
- [ ] ユニットテスト：字面量評価、简单な式、定数畳み込み

**成果物**：`src/middle/cte/` モジュール、`value.rs`、`eval.rs`、`purity.rs` を含む

### Phase 2：純粋関数のコンパイル時評価 + 終了証明

**目標**：純粋関数のコンパイル時完全評価をサポート

**内容**：

- [ ] 関数インライン評価を実装：全パラメータ既知 → 関数体を展開して評価
- [ ] `//! decreases` 解析と終了検証を実装
- [ ] 再帰関数のコンパイル時評価を実装（ステップ数制限付き）
- [ ] 評価結果キャッシュ（メモ化）を実装
- [ ] 純粋性分析を完善：所有権情報を利用し、`&mut` 副作用を識別
- [ ] 部分評価：一部パラメータ既知時のコード生成最適化
- [ ] 統合テスト：`factorial(5)` が型位置で `120` に評価される

**成果物**：`src/middle/cte/interpreter.rs`、`src/middle/cte/termination.rs`

### Phase 3：型レベル計算

**目標**：`If`/`Assert`/`match` 型族をサポート

**内容**：

- [ ] `CTValue::Type(TypeId)` の型レベル操作を実装
- [ ] `If: (C: Bool, T: Type, E: Type) -> Type` の条件型評価を実装
- [ ] `Assert(C)` → `True → Void, False → compile_error` を実装
- [ ] 型レベル `match` を実装：`AsString: (T: Type) -> Type = match T { ... }`
- [ ] 値依存型の完全なインスタンス化：`Matrix(Float, 3, 3)` → 具象型
- [ ] コンパイル時次元検証：行列積の次元不一致 → コンパイルエラー
- [ ] 単相化（mono pass）との統合

**成果物**：`src/middle/cte/type_level.rs`、`src/middle/passes/mono/` を更新

### Phase 4：ホア論理静的検証

**目標**：完全な仕様解析、VC 生成、SMT 検証チャネル

**内容**：

- [ ] パーサー拡張：`//!` と `/*! ... !*/` を仕様ノードとして認識
- [ ] 仕様型定義（`NonEmpty`、`Sorted`、`GreaterOrEqual` などの標準ライブラリ仕様型）
- [ ] ユーザー定義仕様型サポート
- [ ] VC 生成器：最弱前置条件計算
- [ ] Z3 SMT ソルバー統合（`z3` crate 経由）
- [ ] SMT-LIB フォーマット翻訳
- [ ] 反例抽出と可読化レポート
- [ ] Debug/Release/RuntimeChecks の3つのコンパイルモード切り替え
- [ ] 統合テスト：`max`、`binary_search` などの関数の仕様を検証

**成果物**：`src/middle/verification/` モジュール

---

## 八、モジュール計画

```
src/middle/
├── cte/                          # コンパイル時計算エンジン
│   ├── mod.rs                    # CTE 入口、3サブシステムを調整
│   ├── value.rs                  # CTValue 定義 + 基本操作
│   ├── eval.rs                   # 統一 AST 走査器（Interpreter trait + eval_ast）
│   ├── concrete.rs               # 具体評価実装（ConcreteInterpreter → CTValue）
│   ├── symbolic.rs               # 記号評価実装（SymbolicInterpreter → SMTExpr）
│   ├── env.rs                    # EvalEnv（評価環境 + ステップ数制限）
│   ├── purity.rs                 # 純粋性分析器
│   ├── termination.rs            # 終了チェッカー（decreases 検証）
│   ├── type_level.rs             # 型レベル計算（If/Assert/match 型族）
│   └── cache.rs                  # 評価結果キャッシュ
│
├── verification/                  # ホア論理静的検証
│   ├── mod.rs                    # 検証入口
│   ├── spec_parser.rs            # //! 仕様解析
│   ├── spec_types.rs             # 組み込み仕様型定義
│   ├── vcgen.rs                  # 検証条件生成（WP 計算）
│   ├── smt.rs                    # Z3 SMT ソルバーインターフェース
│   └── counterexample.rs         # 反例フォーマット
│
└── passes/
    └── mono/                     # 既存の単相化（CTE 統合強化）
        └── ...                   # CTE 結果を使用したインスタンス化
```

---

## 九、主要設計判断記録

| 判断 | オプション | 選択 | 理由 |
|------|------|------|------|
| 純粋性判断方式 | 明示的アノテーション vs 自動推断 vs 两者結合 | **自動推断** | 所有権システムが既に十分な情報を 提供；明示的アノテーションの逃げ道を提供しない |
| コンパイル時評価器 | 制限付きサブセット vs 完全言語 | **完全言語（ステップ数制限付き）** | 統一型構文の「すべてが `name: type = value`」と一貫性がある |
| 終了証明 | 强制アノテーション vs 自動推断 | **型位置は强制、他は自動** | 型位置が决定不能 = コンパイルエラー、他は寛容になれる |
| VC 生成 | WP 計算 vs SP 計算 | **WP 計算** | WP の方が简单で直接的、エラーローカライズが明確 |
| SMT ソルバー | Z3 vs CVC5 vs 自前開発 | **Z3** | 最も成熟、Rust バインディングが最も完善、社区最大 |
| キャッシュ戦略 | キャッシュなし vs 跨モジュールキャッシュ | **LRU キャッシュ + 增量失效** | コンパイル時評価結果は决定的純粋関数であり、本质的にキャッシュに適している |
| コンパイルモード | 統一モード vs Debug/Release 分離 | **Debug 検証 → Release ゼロオーバーヘッド** | 検証コストが高い、Release は負担不应担 |

---

## 十、リスクと緩和策

| リスク | 影響 | 緩和策 |
|------|------|------|
| Z3 統合の複雑さ | Phase 4 遅延 | 成熟した `z3` crate を使用；简单な算術から徐々に拡張 |
| コンパイル時評価タイムアウト | ユーザー体験の低下 | ステップ数制限 + 明確なタイムアウトエラーメッセージ + 式簡略化の提案 |
| 純粋性誤判断 | コンパイル時評価結果と実行時が不一致 | 所有権システムが強力な保証を提供；誤判断の場合はコンパイラのバグであり、コンパイラを修正すべき |
| SMT 検証失敗のデバッグ困難 | ユーザーがなぜ仕様が成立しないかわからない | 反例抽出 + 具体的な入力値表示 + 実行パスハイライト |
| コンパイル時間の著しい増加 | CI が遅くなる | 增量検証 + モジュール级キャッシュ + 検証結果ファイル（`.o` ファイル类似） |

---

## 十一、既存 RFC との相互参照

| RFC | 関係 | 本計画はどう満たすか |
|------|------|---------------|
| RFC-010 §統一型構文 | CTValue はすべての型式をサポートする必要がある | `CTValue::Type(TypeId)` + `CTValue::Struct` でカバー |
| RFC-011 §4.2 コンパイル時計算 | 値依存型の中核メカニズム | Phase 2/3 で実装 |
| RFC-011 §6 型レベル計算 | `If`/`Assert`/`match` 型族 | Phase 3 で実装 |
| RFC-011 §終了証明メカニズム | decreases 仕様 | Phase 2 の終了チェッカーで実装 |
| RFC-022 §1 仕様コメント構文 | `//!` 解析 + 仕様型 | Phase 4 で実装 |
| RFC-022 §3 検証メカニズム | VC 生成 + SMT 接続 | Phase 4 の VCGen + SMT モジュール |
| RFC-009 §所有権モデル | 純粋性分析の基礎 | Phase 1/2 で所有権情報を复用 |

---

## 参考文献

- [RFC-010: 統一型構文](../design/rfc/accepted/010-unified-type-syntax.md)
- [RFC-011: ジェネリックシステム設計](../design/rfc/accepted/011-generic-type-system.md)
- [RFC-022: ホア論理静的検証](../design/rfc/draft/022-hoare-logic-static-verification.md)
- [RFC-009: 所有権モデル](../design/rfc/accepted/009-ownership-model.md)
- [Z3 Prover](https://github.com/Z3Prover/z3)
- [SMT-LIB Standard](https://smtlib.cs.uiowa.edu/)
- [Weakest Precondition Calculus (Dijkstra)](https://en.wikipedia.org/wiki/Predicate_transformer_semantics)