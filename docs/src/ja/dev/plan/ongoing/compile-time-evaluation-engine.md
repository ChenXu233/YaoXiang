# コンパイル時計算エンジン（CTE Engine）とホーア論理静的検証 — 実装計画

> **タスク**：コンパイル時評価エンジン + ホーア論理静的検証チャネルを実装し、値依存型和とコンパイル時次元検証を支える
> **RFC ベース**：RFC-010（統合型構文）、RFC-011（ジェネリックシステム/値依存型）、RFC-022（ホーア論理静的検証）
> **日付**：2026-05-10
> **状態**：設計中
> **目標マイルストーン**：
> - M1：定数畳み込み + 純度分析スケルトン
> - M2：純粋関数のコンパイル時評価 + 終了検査
> - M3：型レベル計算（`If`/`Assert`/`match` 型族）
> - M4：ホーア論理検証（`//!` 仕様解析 → VC 生成 → SMT 接続）

---

## 摘要

YaoXiang の値依存型（RFC-011）は、型がコンパイル時に既知の値に依存できること（例：`Vec(factorial(5))` → `Vec(120)`）を要求し、ホーア論理静的検証（RFC-022）は純粋関数のコンパイル時仕様検査を要求する。両者は同じ核心的な要件を共有している：**コンパイル時に安全に純粋関数を実行/分析する**。

本計画は**統合コンパイル時計算エンジン（CTE Engine）**を提案し、純度分析、終了検査、式評価を共通インフラストラクチャとして抽象化し、型レベル評価とホーア論理検証という2つのconsumerに対応する。

---

## 核心設計原則

1. **所有権システムを活用した純度分析**：YaoXiang の `&mut` は副作用の印——借用検査が既に何を修正するかを教えてくれる
2. **純粋関数 = コンパイル時に評価可能**：明示的な `const fn` キーワード不要、编译器が自動的に純度を推論
3. **終了証明 = 型安全なベース**：型位置の評価は終了を証明（`decreases` 仕様）する必要がある，否则は型システムが決定不能
4. **部分評価は全量評価より優先**：いくつのパラメータが既知かによる、計算できない部分は実行時に残す
5. **コンパイル時評価とホーア論理はインタープリタを共有**：同じ式評価コアが2つのconsumerを支える
6. **双方向評価モード**：具体的評価（既知パラメータ → `CTValue` 出力）と記号的評価（未知パラメータ → `SMTExpr` 出力）は同じインタープリタフレームワークを共有し、評価環境のみが異なる

---

## アーキテクチャ概要

```
                             ソースファイル + //! 仕様
                                    ↓
                            ┌──────────────┐
                            │   パーサー     │
                            │ (//! コメント認識)│
                            └──────┬───────┘
                                   ↓
                            ┌──────────────┐
                            │  型チェッカー  │
                            │ • 仕様の収集   │
                            │ • 値依存を発見 │
                            └──────┬───────┘
                                   ↓
              ┌────────────────────┴────────────────────┐
              ↓                                         ↓
   ┌──────────────────────┐                ┌──────────────────────┐
   │   CTE エンジン         │                │  ホーア論理検証器    │
   │                       │                │                      │
   │  ┌────────────────┐  │                │  1. //! 仕様の収集    │
   │  │  純度分析器      │  │                │  2. 検証条件(VC)生成  │
   │  │  (所有権ベース)  │  │                │  3. SMT ソルバ(Z3)    │
   │  └───────┬────────┘  │                │  4. 反例レポート      │
   │  ┌───────┴────────┐  │                └──────────┬───────────┘
   │  │  終了チェッカー  │  │                           │
   │  │  (decreases)    │  │                           ↓
   │  └───────┬────────┘  │                ┌──────────────────────┐
   │  ┌───────┴────────┐  │                │  検証結果             │
   │  │  AST インタプリタ│  │                │  • 通過 → キャッシュ  │
   │  │                │  │                │  • 失敗 → release ブロック│
   │  │ ┌────────────┐ │  │                └──────────────────────┘
   │  │ │ 具体的評価  │ │  │                       ↑
   │  │ │ env: 全既知 │ │  │                       │
   │  │ │ → CTValue  │ │  │  共有インタプリタフレームワーク│
   │  │ └────────────┘ │  │  (AST巡回/インライン化/ループ展開)│
   │  │ ┌────────────┐ │  │                       │
   │  │ │ 記号的評価  │─┼──┼───────────────────────┘
   │  │ │ env: 部分既知│ │  │  (ホーア論理consume)
   │  │ │ → SMTExpr  │ │  │  (ホーア論理消費)
   │  │ └────────────┘ │  │  │
   │  └───────┬────────┘  │  │
   │          ↓           │  │
   │  結果を型/単態化に嵌入│  │
   └──────────────────────┘
```

---

## 一、コンパイル時値（CTValue）

コンパイル時計算の核心データ型。编译器内部IR値として設計され、実行時値と混同しない。

```rust
/// コンパイル時評価結果
enum CTValue {
    /// 整数（Bool のすべてのコンパイル時用途をカバー）
    Int(i64),

    /// 浮動小数点数
    Float(f64),

    /// 文字列（エラーメッセージ、型名など）
    String(SmolStr),

    /// 型参照——型レベル計算の核心
    /// YaoXiang の型自身は Type1 層の"値"
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
    /// すべてのパラメータが既知のときインライン評価、さもなくば実行時呼び出しを保持
    Thunk {
        func: FunctionId,
        known_args: Vec<CTValue>,
        unknown_params: Vec<ParamId>,
    },
}
```

**重要な設計**：`CTValue::Type(TypeId)` により型が一等コンパイル時値になる。`If(C, T, E)` の C 評価結果は `CTValue::Bool`、T/E 評価結果は `CTValue::Type`。

---

## 二、サブシステム 1：純度分析器

### 2.1 設計思路

YaoXiang の所有権システム（RFC-009）を活用し、副作用は型シグネチャで自然に表現される：

| パラメータパターン | 意味 | コンパイル時評価可能？ |
|----------|------|---------------|
| `x: T` (所有権取得) | 所有権を取得、自由に変更可能 | ✅ |
| `x: &T` (共有参照) | 読み取りのみ | ✅ |
| `x: &mut T` (独占参照) | 変更可能 | ⚠️ T の出所に依存 |
| I/O 呼び出し | 外部副作用 | ❌ |
| 非純粋関数の呼び出し | 推移的 | ❌ |

### 2.2 算法

```
analyze_purity(func: FunctionId, ctx: &mut PurityContext) -> PurityResult:
    // 1. 高速パス：既に注釈付き
    if ctx.has_purity_annotation(func):
        return ctx.get_annotation(func)

    // 2. 直接副作用の検査
    for op in func.body.operations():
        match op:
            Call(callee, _) if is_io_operation(callee):
                return Impure("I/O 操作")
            Call(callee, args) where has_mut_arg(args):
                if arg_escapes_function(args):
                    return Impure("&mut による外部状態の変更")
            Call(callee, _):
                // 推移性：呼び出し先も純粋関数でなければならない
                if analyze_purity(callee, ctx).is_impure():
                    return Impure("非純粋関数を呼び出し: {callee}")

    // 3. デフォルトで純粋関数と推論
    return Pure
```

### 2.3 明示的な純度注釈を提供しない

**設計上の決定：明示的な `//! pure` などの注釈を提供しない。**

所有権システム（RFC-009）は既に型シグネチャを通じて副作用情報を表現している——`&mut T` は変更、`I/O` 操作は外部副作用である。编译器は自動的に純度を推論できる能力を持つ。

编译器が純粋関数を非純粋と誤判定した場合、それは编译器のバグであり、用户にパッチを打たせるのではなく、编译器を修正すべきである。「信用しろ、この関数は純粋だ」という注釈を提供する情報は、本当の問題を覆い隠すだけである。

> *"互換性、フォールバック、一時的、バックアップ、特定モード有効なコードを書くな。問題を直接露呈させろ。"*

### 2.4 RFC-022 との関係

純度分析器は以下の両者にサービスする：

- **CTE**：非純粋関数は型位置で使用不可
- **ホーア論理**：仕様式（`requires`/`ensures` の右辺）は純粋関数呼び出しでなければならない

---

## 三、サブシステム 2：終了チェッカー

### 3.1 設計思路

型位置のコンパイル時評価は終了を保証しなければならず、さもなくば型システムは決定不能となる。YaoXiang は `//! decreases` 仕様で終了を証明する。

```
//! decreases: <expr>
```

ここで `<expr>` は下限を持つ整列値（通常は自然数型の `Int`）。

### 3.2 算法

```
check_termination(func: FunctionId, ctx: &mut TermContext) -> TermResult:
    // 1. decreases 仕様の検索
    let decreases_expr = find_decreases_spec(func)
        .or_else(|| infer_decreases(func))

    match decreases_expr:
        None if has_recursive_call(func):
            return TermError::NoDecreasesAnnotation
        None:
            return TermOk  // 再帰なし、証明不要

        Some(decreases):
            // 2. 各再帰呼び出しサイトの検証
            for call in func.recursive_calls():
                let dec_at_call = eval_decreases_at(call, decreases)
                let dec_at_entry = eval_decreases_at(func.entry, decreases)

                if !strictly_less_than(dec_at_call, dec_at_entry):
                    return TermError::NotDecreasing {
                        at: call.location,
                        expected_less_than: dec_at_entry,
                        actual: dec_at_call,
                    }

            // 3. 下限の検証
            if !has_lower_bound(decreases):
                return TermError::NoLowerBound

            return TermOk
```

### 3.3 自動推論

明白な終了状況は注釈不要：

```yaoxiang
// decreases 不要——编译器はループが既知の上界 n を持つことを認識
sum: (arr: Array(Int, n)) -> Int = {
    s = 0; i = 0
    while i < n { s += arr[i]; i += 1 }
    return s
}
```

注釈が必要な状況：
```yaoxiang
// decreases 必須——再帰呼び出し n-1
factorial: (n: Int) -> Int = {
    //! decreases: n
    if n <= 1 { return 1 }
    return n * factorial(n - 1)
}
```

### 3.4 RFC-022 との関係

終了チェッカーは以下の両者にサービスする：

- **CTE**：decreases はコンパイル時評価の准入门槛
- **ホーア論理**：ループ不変式の decreases バリアント（`/*! decreases: n - i !*/`）も終了チェッカーで検証

---

## 四、サブシステム 3：AST インタプリタ

### 4.1 設計思路

インタプリタはAST巡回に基づき、評価環境（変数名 → CTValue マッピング）を維持する。核心的能力は**部分評価**：既知のパラメータは計算し、未知のパラメータは保持する。

```
eval(expr: &Expr, env: &mut EvalEnv) -> EvalResult<CTValue>:
    match expr:
        // リテラル → 直接変換
        Literal(lit) => lit.into_ctvalue()

        // 変数 → 環境の検索
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

        // 関数呼び出し——核心論理
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

        // パターン照合
        Match(scrutinee, arms) =>
            let val = eval(scrutinee, env)?
            for arm in arms:
                if arm.pattern.matches(val):
                    return eval(arm.body, env.with_bindings(arm.bindings))
            Err(NoMatch)

        // コードブロック
        Block(stmts) =>
            for stmt in stmts[..len-1]:
                eval(stmt, env)?
            eval(stmts.last(), env)

        // ループ
        While(cond, body) =>
            let mut result = CTValue::Void
            while eval(cond, env)? == Bool(true):
                check_step_limit()?  // 無限ループ防止
                result = eval(body, env)?
            result
```

### 4.2 インライン評価

すべてのパラメータが既知のとき、インタプリタは関数体を評価コンテキストにインライン化する：

```
inline_and_eval(func, args, env):
    // 1. 純度検査
    purity.check(func)?

    // 2. キャッシュ確認
    if let Some(cached) = cache.get(func, args):
        return cached

    // 3. インライン環境の作成
    let mut inline_env = env.child()
    for (param, arg) in func.params.zip(args):
        inline_env.bind(param.name, arg)

    // 4. 関数本体の評価
    let result = eval(&func.body, &mut inline_env)?

    // 5. 結果のキャッシュ
    cache.insert(func, args, result.clone())
    result
```

### 4.3 ステップ数制限

コンパイル時評価は硬性制限を持たなければならず、`decreases` があっても偶発的なタイムアウトを防ぐ：

```rust
const MAX_EVAL_STEPS: u64 = 1_000_000;  // 100万ステップ硬上限

struct EvalEnv {
    variables: HashMap<SmolStr, CTValue>,
    step_count: u64,
    step_limit: u64,
}
```

### 4.4 双方向評価モード：具体的 vs 記号的

インタプリタの核心フレームワーク（AST巡回、インライン展開、パターン照合）は統一だが、**評価環境**が2つのモードを決定する：

#### 4.4.1 具体的評価（Concrete Evaluation）

**consumer**：CTE エンジン → 型レベル評価、単態化

**特徴**：

- 環境のすべての変数が具体的な `CTValue` を持つ
- 関数呼び出しパラメータがすべて既知 → インライン評価
- 出力：`CTValue`（具体的値または型参照）
- 失敗 = コンパイルエラー

```
// シナリオ：Vec(factorial(5))
// env = { factorial → Function(...) }
eval(Call("factorial", [Literal(5)]), env):
    → inline_and_eval(factorial, [CTValue::Int(5)], env)
    → CTValue::Int(120)
// 型置換：Vec(120)
```

#### 4.4.2 記号的評価（Symbolic Evaluation）

**consumer**：ホーア論理検証器 → SMT 求解

**特徴**：

- 環境に**記号的変数**が存在（関数パラメータ `n`、`arr`、コンパイル時に未知）
- 既知の部分的式は具体的値に評価され、未知部分は SMT 記号として保持
- 関数呼び出しはインライン化されない——代わりに論理式に展開
- 出力：`SMTExpr`（一階述語論理式）、Z3 に渡す
- 失敗 = 検証不通過（コンパイルエラーではない）

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

| 次元 | 具体的評価 | 記号的評価 |
|------|----------|----------|
| 環境 | `HashMap<Name, CTValue>` | `HashMap<Name, SMTTerm>` |
| 変数未知時 | エラー | 記号として保持 |
| 関数呼び出し | インライン + 関数本体評価 | 論理定義に展開（実行しない）|
| ループ | 実際の反復（ステップ数制限あり）| ループ不変式 VC に変換 |
| 出力型 | `Result<CTValue, CTError>` | `Result<SMTExpr, SMError>` |
| 失敗のセマンティクス | コンパイルエラー | 検証失敗（Runtime Check に降級可能）|
| 性能特性 | 高速（直接計算）| 低速（SMT 求解）|

#### 4.4.4 共有インタプリタフレームワーク

2つのモードは同一の AST巡回スケルトンを共有：

```rust
/// インタプリタ trait：具体的評価と記号的評価が各自実装
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

/// 統一 AST 巡回器、具体的な実装に委譲
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
        // ... 残りの AST ノードも同様
    }
}
```

**重要な洞察**：具体的評価と記号的評価の AST巡回論理は完全に同じで、異なる点は：

- **値をどう表現するか**（`CTValue` vs `SMTExpr`）
- **関数呼び出しをどう処理するか**（インライン実行 vs 論理展開）
- **未知変数をどう扱うか**（エラー vs 記号保持）

---

### 5.1 CTE consumer：型レベル計算

| 位置 | CTE 使用例 | 結果 |
|------|-----------|------|
| 1. 型注釈位置 | `Vec(factorial(5))` | CTE::eval(factorial(5)) → CTValue::Int(120) → 型を Vec(120) に置換 |
| 2. ジェネリック値パラメータ | `Array(Int, factorial(3))` | CTE::eval(factorial(3)) → CTValue::Int(6) → Array(Int, 6) にインスタンス化 |
| 3. Assert 型 | `Assert(N > 0)` | CTE::eval(N > 0) → CTValue::Bool(true/false) → True → Void、False → compile_error("N must be > 0") |
| 4. If 条件型 | `If(C, T, E)` | CTE::eval(C) → CTValue::Bool(b) → True → T、False → E |
| 5. Match 型族 | `AsString(Int)` | match Int { Int => String, ... } → String |

### 5.2 単態化との相互作用

```
単態化は以下の位置で CTE の結果を使用：

1. 既知のジェネリック値パラメータ → 具体的なインスタンスを生成
   List(Int) の push メソッド → push_List_Int を生成

2. 既知の値依存型 → 具体的な型に展開
   Matrix(Float, 3, 3).data → Array(Array(Float, 3), 3)

3. 部分評価 → 単相化コードを生成
   map(Int, String) → map_Int_String を生成（T=Int、R=String は既に固定）
```

### 5.3 ホーア論理検証器との相互作用

```
検証器は以下の位置で CTE を使用：

1. 仕様式の一部評価
   //! requires: n > 0 && factorial(n) < MAX
   CTE::eval(factorial(n)) → n がコンパイル時に既知 → 定数
                            → n が未知 → 記号として保持、SMT に渡す

2. 仕様条件の簡略化
   //! ensures: result >= 0 && result < n
   CTE は既知の部分的式を簡略化し、SMT 求解の負担を軽減

3. 仕様型インスタンス化
   NonEmpty(n) = n > 0
   CTE は仕様型をブール式に展開
```

---

## 六、ホーア論理静的検証（RFC-022 実装設計）

### 6.1 仕様解析

`//!` と `/*! ... !*/` はパーサーによって特殊コメントノードとして認識され、AST に付加される：

```rust
struct SpecAnnotation {
    kind: SpecKind,        // Requires | Ensures | Invariant | Decreases
    name: Option<SmolStr>, // 仕様名（オプションの用户命名）
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

最弱事前条件（Weakest Precondition）計算を使用：

```
generate_vc(func: FunctionId) -> Vec<VerificationCondition>:
    let requires = collect_requires(func)
    let ensures = collect_ensures(func)
    let invariants = collect_invariants(func)
    let decreases = collect_decreases(func)

    let mut vcs = Vec::new()

    // VC1: 事前条件の一貫性
    vcs.push(VC::PreconditionConsistency(requires))

    // VC2: 事後条件の検証（各実行パスに対して）
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
        // 各反復で保持
        vcs.push(VC::InvariantPreservation { loop_, invariant })
        // 退出後に事後条件を蕴含
        vcs.push(VC::InvariantExit { loop_, invariant, post: ensures })

    vcs
```

### 6.3 SMT ソルバ統合

```
┌─────────────┐     SMT-LIB フォーマット      ┌───────────┐
│  VC Generator │ ──────────────────→ │  Z3 ソルバ  │
└─────────────┘                       └─────┬───────┘
                                            │
                          ┌─────────────────┴──────────────┐
                          ↓                                ↓
                      unsat                            sat
                          ↓                                ↓
                    ┌──────────┐                   ┌──────────────┐
                    │ 検証通過  │                   │  反例モデルを抽出│
                    │ 結果をキャッシュ│                   │  読み取り可能な形式に変換│
                    └──────────┘                   └──────┬───────┘
                                                         ↓
                                                  ┌──────────────┐
                                                  │ コンパイルエラーレポート│
                                                  │ • 入力値       │
                                                  │ • 違反した仕様  │
                                                  └──────────────┘
```

### 6.4 コンパイルモード

| モード | 動作 | CLI |
|------|------|-----|
| **Debug Build** | 仕様を解析、VC を生成、Z3 で証明；検証通過後にのみ Release Build | `yaoxiangc --debug` |
| **Release Build** | すべての `//!` コメントを無視、ゼロオーバーヘッド、検証キャッシュをクリア | `yaoxiangc --release` |
| **Runtime Checks** | 仕様を `assert` 文に降級、違反時に panic | `yaoxiangc --runtime-checks` |

---

## 七、実装フェーズ

### Phase 1：定数畳み込み + 純度分析スケルトン

**目標**：CTE インフラストラクチャを確立し、基本的なコンパイル時評価をサポート

**内容**：

- [ ] `CTValue` 列挙型と `EvalEnv` 構造体を定義
- [ ] `eval()` の基本パス реализация：リテラル、変数、二項演算、条件、コードブロック
- [ ] 純度分析器の第一版実装：I/O 呼び出しを非純粋と識別し、他はデフォルトで純粋
- [ ] 型チェッカーに CTE 呼び出し点を挿入（型注釈位置）
- [ ] 定数畳み込み：`1 + 2 * 3` をコンパイル時に `7` として計算
- [ ] 死んだ分岐の消除：`if true { ... } else { ... }` → then 分岐を直接採用
- [ ] ユニットテスト：リテラル評価、简单な式、定数畳み込み

**成果**：`src/middle/cte/` モジュール、`value.rs`、`eval.rs`、`purity.rs` を含む

### Phase 2：純粋関数のコンパイル時評価 + 終了検査

**目標**：純粋関数のコンパイル時の完全な評価をサポート

**内容**：

- [ ] 関数インライン評価 реализация：すべてのパラメータが既知 → 関数体を展開して評価
- [ ] `//! decreases` 解析と終了検証 реализация
- [ ] 再帰関数のコンパイル時評価 реализация（ステップ数制限付き）
- [ ] 評価結果のキャッシュ（Memoization） реализация
- [ ] 純度分析器の改良：所有権情報を活用して `&mut` 副作用を識別
- [ ] 部分評価：一部パラメータが既知のときのコード生成最適化
- [ ] 統合テスト：`factorial(5)` が型位置で `120` に評価される

**成果**：`src/middle/cte/interpreter.rs`、`src/middle/cte/termination.rs`

### Phase 3：型レベル計算

**目標**：`If`/`Assert`/`match` 型族をサポート

**内容**：

- [ ] `CTValue::Type(TypeId)` の型レベル操作 реализация
- [ ] `If: (C: Bool, T: Type, E: Type) -> Type` の条件型評価 реализация
- [ ] `Assert(C)` → `True → Void, False → compile_error` 实现
- [ ] 型レベル `match`：`AsString: (T: Type) -> Type = match T { ... }` 实现
- [ ] 値依存型の完全なインスタンス化：`Matrix(Float, 3, 3)` → 具体的な型
- [ ] コンパイル時の次元検証：行列乗算の次元不一致 → コンパイルエラー
- [ ] 単態化（mono pass）との統合

**成果**：`src/middle/cte/type_level.rs`、`src/middle/passes/mono/` の更新

### Phase 4：ホーア論理静的検証

**目標**：完全な仕様解析、VC 生成、SMT 検証チャネル

**内容**：

- [ ] パーサー拡張：`//!` と `/*! ... !*/` を仕様ノードとして認識
- [ ] 仕様型の定義（`NonEmpty`、`Sorted`、`GreaterOrEqual` など標準ライブラリ仕様型）
- [ ] ユーザー定義仕様型のサポート
- [ ] VC 生成器：最弱事前条件計算
- [ ] Z3 SMT ソルバ統合（`z3` crate を使用）
- [ ] SMT-LIB フォーマット翻訳
- [ ] 反例抽出と読み取り可能なレポート
- [ ] Debug/Release/RuntimeChecks の3つのコンパイルモード切り替え
- [ ] 統合テスト：`max`、`binary_search` などの関数の仕様を検証

**成果**：`src/middle/verification/` モジュール

---

## 八、モジュール構成

```
src/middle/
├── cte/                          # コンパイル時計算エンジン
│   ├── mod.rs                    # CTE 入口、3つのサブシステムを調整
│   ├── value.rs                  # CTValue 定義 + 基本操作
│   ├── eval.rs                   # 統一 AST 巡回器（Interpreter trait + eval_ast）
│   ├── concrete.rs               # 具体的評価実装（ConcreteInterpreter → CTValue）
│   ├── symbolic.rs               # 記号的評価実装（SymbolicInterpreter → SMTExpr）
│   ├── env.rs                    # EvalEnv（評価環境 + ステップ数制限）
│   ├── purity.rs                 # 純度分析器
│   ├── termination.rs            # 終了チェッカー（decreases 検証）
│   ├── type_level.rs             # 型レベル計算（If/Assert/match 型族）
│   └── cache.rs                  # 評価結果キャッシュ
│
├── verification/                 # ホーア論理静的検証
│   ├── mod.rs                    # 検証入口
│   ├── spec_parser.rs            # //! 仕様解析
│   ├── spec_types.rs             # 組み込み仕様型定義
│   ├── vcgen.rs                  # 検証条件生成（WP 計算）
│   ├── smt.rs                    # Z3 SMT ソルバインターフェース
│   └── counterexample.rs         # 反例フォーマット
│
└── passes/
    └── mono/                     # 既存の単態化（CTE 統合強化）
        └── ...                   # CTE 結果を使用してインスタンス化
```

---

## 九、主要な設計決定記録

| 決定 | オプション | 選択 | 理由 |
|------|------|------|------|
| 純度判定方式 | 明示的注釈 vs 自動推論 vs 両者の組み合わせ | **自動推論** | 所有権システムが十分な情報を提供；明示的注釈の逃げ道を提供しない |
| コンパイル時評価器 | 制限付きサブセット vs 完全な言語 | **完全な言語（ステップ数制限付き）** | 統合型構文の「すべては `name: type = value`」と一貫性あり |
| 終了証明 | 強制注釈 vs 自動推論 | **型位置は強制、他は自動** | 型位置が決定不能 = コンパイルエラー、他は緩和可能 |
| VC 生成 | WP 計算 vs SP 計算 | **WP 計算** | WP はよりシンプルで直接的、エラー位置特定が明確 |
| SMT ソルバ | Z3 vs CVC5 vs 自作 | **Z3** | 最も成熟、Rust binding が最も完善、社区最大 |
| キャッシュ戦略 | キャッシュなし vs モジュール間キャッシュ | **LRU キャッシュ + 増分失效** | コンパイル時評価結果は決定性純粋関数で、キャッシュに最適 |
| コンパイルモード | 統一モード vs Debug/Release 分離 | **Debug 検証 → Release ゼロオーバーヘッド** | 検証コストが高く、Release は負担すべきではない |

---

## 十、リスクと緩和

| リスク | 影響 | 緩和策 |
|------|------|------|
| Z3 統合の複雑さ | Phase 4 遅延 | 成熟した `z3` crate を使用；まずは简单な算術をサポートし，逐步的に拡張 |
| コンパイル時評価タイムアウト | ユーザー体験の低下 | ステップ数制限 + 明確なタイムアウトエラーメッセージ + 式簡略化の提案 |
| 純度誤判定 | コンパイル時評価結果と実行時が不整合 | 所有権システムが強力な保証を提供；誤判定した場合は编译器の bug で、编译器を修正すべき |
| SMT 検証失敗がデバッグ困難 | ユーザーがなぜ仕様が成立しないかわからない | 反例抽出 + 具体的な入力値表示 + 実行パスハイライト |
| コンパイル時間が著しく増加 | CI が遅くなる | 増分検証 + モジュールレベルキャッシュ + 検証結果ファイル（`.o` ファイル类似）|

---

## 十一、既存の RFC への相互参照

| RFC | 関係 | 本計画がどう対応するか |
|------|------|---------------|
| RFC-010 §統合型構文 | CTValue はすべての型式をサポートする必要がある | `CTValue::Type(TypeId)` + `CTValue::Struct` でカバー |
| RFC-011 §4.2 コンパイル時計算 | 値依存型の核心機構 | Phase 2/3 で実装 |
| RFC-011 §6 型レベル計算 | `If`/`Assert`/`match` 型族 | Phase 3 で実装 |
| RFC-011 §終了検査機構 | decreases 仕様 | Phase 2 の終了チェッカー実装 |
| RFC-022 §1 仕様コメント構文 | `//!` 解析 + 仕様型 | Phase 4 で実装 |
| RFC-022 §3 検証機構 | VC 生成 + SMT 接続 | Phase 4 の VCGen + SMT モジュール |
| RFC-009 §所有権モデル | 純度分析の基礎 | Phase 1/2 で所有権情報を活用 |

---

## 参考文献

- [RFC-010: 統合型構文](../design/rfc/accepted/010-unified-type-syntax.md)
- [RFC-011: ジェネリックシステム設計](../design/rfc/accepted/011-generic-type-system.md)
- [RFC-022: ホーア論理静的検証](../design/rfc/draft/022-hoare-logic-static-verification.md)
- [RFC-009: 所有権モデル](../design/rfc/accepted/009-ownership-model.md)
- [Z3 Prover](https://github.com/Z3Prover/z3)
- [SMT-LIB Standard](https://smtlib.cs.uiowa.edu/)
- [Weakest Precondition Calculus (Dijkstra)](https://en.wikipedia.org/wiki/Predicate_transformer_semantics)