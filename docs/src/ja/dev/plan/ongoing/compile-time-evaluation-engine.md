# コンパイル時計算エンジン（CTE Engine）とホーア論理静的検証 — 実装計画

> **タスク**：コンパイル時評価エンジン＋ホーア論理静的検証通道を実装し、値依存型とコンパイル時次元検証を支える  
> **RFC ベース**：RFC-010（統一型構文）、RFC-011（ジェネリクスシステム/値依存型）、RFC-022（ホーア論理静的検証）  
> **日付**：2026-05-10  
> **状態**：設計中  
> **目標マイルストーン**：
> - M1：定数畳み込み＋純粋性分析スケルトン
> - M2：純粋関数のコンパイル時評価＋終了判定
> - M3：型レベル計算（`If`/`Assert`/`match` 型族）
> - M4：ホーア論理検証（`//!` 仕様解析 → VC 生成 → SMT 連携）

---

## 摘要

YaoXiang の値依存型（RFC-011）は、型がコンパイル時に既知の値に依存できることを要求する（例：`Vec(factorial(5))` → `Vec(120)`）。また、ホーア論理静的検証（RFC-022）は純粋関数に対するコンパイル時仕様チェックを要求する。両者に共通する中核的需求は、**コンパイル時に純粋関数を安全に実行/分析すること**である。

本計画は**統一コンパイル時計算エンジン（CTE Engine）**を提案し、純粋性分析、終了判定、式評価を共通インフラストラクチャとして抽象化し、型レベル評価とホーア論理検証という2つの消費者にそれぞれサービスを提供する。

---

## 中核的设计原则

1. **所有権システム再利用による純粋性分析**：YaoXiang の `&mut` は副作用の印——借用チェックにより何が変更されるかがすでにわかる
2. **純粋関数＝コンパイル時評価可能**：`const fn` キーワードは不要、编译器が純粋性を自動推論
3. **終了証明＝型安全の土台**：型位置の評価は終了を証明必須（`decreases` 仕様）、否则型システムは判定不能
4. **部分評価优于全量評価**：パラメータがいくつ知られているかだけ評価し、実行时可出ないものはランタイムに委譲
5. **コンパイル時評価とホーア論理の共用インタープリタ**：同一の式評価コアが2つの消費者を支える
6. **双モード評価**：具体評価（既知パラメータ → `CTValue` 出力）と記号評価（未知パラメータ → `SMTExpr` 出力）は同一のインタープリタフレームワークを共有し、評価環境のみ異なる

---

## アーキテクチャ総覧

```
                             ソースファイル + //! 仕様
                                    ↓
                            ┌──────────────┐
                            │   パーサー     │
                            │ (//! コメント  │
                            │  識別)        │
                            └──────┬───────┘
                                   ↓
                            ┌──────────────┐
                            │  型チェッカー  │
                            │ • 仕様収集    │
                            │ • 値依存発見  │
                            └──────┬───────┘
                                   ↓
              ┌────────────────────┴────────────────────┐
              ↓                                         ↓
   ┌──────────────────────┐                ┌──────────────────────┐
   │   CTE エンジン        │                │  ホーア論理検証器     │
   │                      │                │                      │
   │  ┌────────────────┐  │                │  1. //! 仕様収集      │
   │  │  純粋性分析器    │  │                │  2. 検証条件(VC)生成  │
   │  │  (所有権ベース)  │  │                │  3. SMT ソルバ(Z3)    │
   │  └───────┬────────┘  │                │  4. 反例レポート      │
   │  ┌───────┴────────┐  │                └──────────┬───────────┘
   │  │  終了判定器      │  │                           │
   │  │  (decreases)    │  │                           ↓
   │  └───────┬────────┘  │                ┌──────────────────────┐
   │  ┌───────┴────────┐  │                │  検証結果             │
   │  │  AST インタープリタ│                │  • 通過 → キャッシュ  │
   │  │                │  │                │  • 失敗 → release    │
   │  │ ┌────────────┐ │  │                │     ブロック         │
   │  │ │  具体評価    │ │  │                └──────────────────────┘
   │  │ │ env: 全既知  │ │  │                       ↑
   │  │ │ → CTValue   │ │  │                       │
   │  │ └────────────┘ │  │  共用インタープリタ      │
   │  │ ┌────────────┐ │  │  フレームワーク         │
   │  │ │  記号評価    │─┼──┼───────────────────────┘
   │  │ │ env: 部分既知│ │  │  (ホーア論理消費)
   │  │ │ → SMTExpr   │ │  │
   │  │ └────────────┘ │  │
   │  └───────┬────────┘  │
   │          ↓           │
   │  結果を型/単態化に嵌入│
   └──────────────────────┘
```

---

## 一、コンパイル時値（CTValue）

コンパイル時計算の中核データ型。コンパイラ内部 IR 値として設計され、ランタイム値と混同しない。

```rust
/// コンパイル時評価結果
enum CTValue {
    /// 整数（Bool のすべてのコンパイル時用途をカバー）
    Int(i64),

    /// 浮動小数点数
    Float(f64),

    /// 文字列（エラーメッセージ、型名など）
    String(SmolStr),

    /// 型参照——型レベル計算の中核
    /// YaoXiang の型は Type1 層の"値"そのもの
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
    /// すべてのパラメータが既知ならインライン評価、そうでなければランタイム呼び出しを保持
    Thunk {
        func: FunctionId,
        known_args: Vec<CTValue>,
        unknown_params: Vec<ParamId>,
    },
}
```

**中核的设计**：`CTValue::Type(TypeId)` により型が一等コンパイル時値になる。`If(C, T, E)` の C を評価すると `CTValue::Bool`、T/E を評価すると `CTValue::Type`。

---

## 二、サブシステム 1：純粋性分析器

### 2.1 設計思路

YaoXiang の所有権システム（RFC-009）を再利用し、副作用は型シグネチャで自然に表現される：

| パラメータパターン | 意味 | コンパイル時評価可能？ |
|----------|------|---------------|
| `x: T` (所有権取得) | 所有権を取得、自由に変更可能 | ✅ |
| `x: &T` (共有参照) | 読み取り専用 | ✅ |
| `x: &mut T` (独占参照) | 変更可能 | ⚠️ T の出所に依存 |
| I/O 呼び出し | 外部副作用 | ❌ |
| 非純粋関数呼び出し | 推移性 | ❌ |

### 2.2 アルゴリズム

```
analyze_purity(func: FunctionId, ctx: &mut PurityContext) -> PurityResult:
    // 1. 高速パス：すでに注釈付き
    if ctx.has_purity_annotation(func):
        return ctx.get_annotation(func)

    // 2. 直接副作用のチェック
    for op in func.body.operations():
        match op:
            Call(callee, _) if is_io_operation(callee):
                return Impure("I/O operation")
            Call(callee, args) where has_mut_arg(args):
                if arg_escapes_function(args):
                    return Impure("mutation of external state via &mut")
            Call(callee, _):
                // 推移性：被呼び出し関数も純粋関数である必要がある
                if analyze_purity(callee, ctx).is_impure():
                    return Impure("calls impure function: {callee}")

    // 3. デフォルトで純粋関数と推論
    return Pure
```

### 2.3 明示的な純粋性注釈の提供なし

**設計上の決定：`//! pure` などの明示的注釈を提供しない。**

所有権システム（RFC-009）はすでに型シグネチャを通じて副作用情報を表現している——`&mut T` は変更、`I/O` 操作は外部副作用。コンパイラは純粋性を自動推論できる能力を持つ。

コンパイラが純粋関数を非純粋と誤判定した場合、それはコンパイラのバグであり、修正すべきはコンパイラであり、ユーザーがパッチを打つことではない。「この関数は純粋だと信じろ」という注釈を提供することは、真の問題を隠蔽するだけである。

> *"互換性、フォールバック、一時的、代替、特定モードでのみ有効なコードを書くな。問題を直接露呈させろ。"*

### 2.4 RFC-022 との関係

純粋性分析器は同時に以下に貢献する：

- **CTE**：非純粋関数は型位置で使用不可
- **ホーア論理**：仕様式（`requires`/`ensures` の右辺）は純粋関数呼び出しである必要がある

---

## 三、サブシステム 2：終了判定器

### 3.1 設計思路

型位置のコンパイル時評価は終了を保証する必要がある、さもなくば型システムは判定不能となる。YaoXiang は `//! decreases` 仕様で終了を証明する。

```
//! decreases: <expr>
```

ここで `<expr>` は下限を持つ整列値（通常は自然数である `Int` 型）。

### 3.2 アルゴリズム

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
            // 2. 各再帰呼び出し地点の検証
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

一部の明確な終了ケースは注釈不要：

```yaoxiang
// decreases 不要——コンパイラはループに上界 n があることを認識
sum: (arr: Array(Int, n)) -> Int = {
    s = 0; i = 0
    while i < n { s += arr[i]; i += 1 }
    return s
}
```

注釈が必要なケース：
```yaoxiang
// decreases 注釈必須——再帰呼び出し n-1
factorial: (n: Int) -> Int = {
    //! decreases: n
    if n <= 1 { return 1 }
    return n * factorial(n - 1)
}
```

### 3.4 RFC-022 との関係

終了判定器は同時に以下に貢献する：

- **CTE**：decreases はコンパイル時評価の参入门槛
- **ホーア論理**：ループ不変式の decreases バリアント（`/*! decreases: n - i !*/`）も終了判定器で検証

---

## 四、サブシステム 3：AST インタープリタ

### 4.1 設計思路

インタープリタは AST 走査に基づき、評価環境（変数名 → CTValue マッピング）を維持。中核能力は**部分評価**：既知パラメータは計算し、未知パラメータは保持する。

```
eval(expr: &Expr, env: &mut EvalEnv) -> EvalResult<CTValue>:
    match expr:
        // リテラル → 直接変換
        Literal(lit) => lit.into_ctvalue()

        // 変数 → 環境で検索
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
                // 一部既知 → 部分評価（単相化コード生成）
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

すべてのパラメータが既知な場合、インタープリタは関数体を評価コンテキストにインライン化する：

```
inline_and_eval(func, args, env):
    // 1. 純粋性チェック
    purity.check(func)?

    // 2. キャッシュチェック
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

コンパイル時評価には硬限が必要で、`decreases` があっても予期せぬタイムアウトを防止：

```
const MAX_EVAL_STEPS: u64 = 1_000_000;  // 100万ステップ硬限

struct EvalEnv {
    variables: HashMap<SmolStr, CTValue>,
    step_count: u64,
    step_limit: u64,
}
```

### 4.4 双モード評価：具体 vs 記号

インタープリタの中核フレームワーク（AST 走査、インライン展開、パターン照合）は統一だが、**評価環境**が2つのモードを決定する：

#### 4.4.1 具体評価（Concrete Evaluation）

**消費者**：CTE エンジン → 型レベル評価、単態化

**特徴**：

- 環境のすべての変数に具体的な `CTValue` がある
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

**消費者**：ホーア論理検証器 → SMT 求解

**特徴**：

- 環境に**記号変数**が存在（関数パラメータ `n`、`arr`、コンパイル時未知）
- 既知な部分式は具体値で評価し、未知部分は SMT 記号として保持
- 関数呼び出しはインライン化されない——代わりに論理式に展開
- 出力：`SMTExpr`（一階述語論理式）、Z3 へ渡す
- 失敗 = 検証不通過（コンパイルエラーではない）

```
// シナリオ：max の ensures 検証:
//   //! ensures: GreaterOrEqual(result, arr[0..n]) = result >= forall arr[i]
// env = { result → Symbol("result"), arr → Symbol("arr"), n → Symbol("n") }
eval(BinaryOp(Variable("result"), GtEq, Call("arr_max", [Symbol("arr"), Symbol("n")]))):
    // result は記号 → 保持
    // arr_max(arr, n) は純粋関数だがパラメータ未知 → 論理定義に展開
    → SMTExpr::Forall(i in 0..n, Symbol("result") >= Symbol("arr")[i])
// Z3 へ：∀arr, n, result. (n > 0 ∧ ...) → result >= arr[0] ∧ ... ∧ result >= arr[n-1]
```

#### 4.4.3 2つのモードの主要区别

| 次元 | 具体評価 | 記号評価 |
|------|----------|----------|
| 環境 | `HashMap<Name, CTValue>` | `HashMap<Name, SMTTerm>` |
| 変数未知時 | エラー | 記号として保持 |
| 関数呼び出し | インライン化 + 関数本体実行 | 論理定義に展開（実行しない） |
| ループ | 実際の反復（ステップ数制限付き） | ループ不変式 VC に変換 |
| 出力型 | `Result<CTValue, CTError>` | `Result<SMTExpr, SMError>` |
| 失敗の语义 | コンパイルエラー | 検証失敗（Runtime Check に降級可能） |
| 性能特性 | 高速（直接計算） | 低速（SMT 求解） |

#### 4.4.4 共用インタープリタフレームワーク

2つのモードは同一の AST 走查スケルトンを共有：

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

/// 統一 AST 走査器、具体的な実装に委譲
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
        // ... 其余の AST ノード同理
    }
}
```

**中核の洞察**：具体評価と記号評価の AST 走査ロジックは完全に同一で、差异は以下のみ：

- **値用什么で表現するか**（`CTValue` vs `SMTExpr`）
- **関数呼び出し怎么处理**（インライン実行 vs 論理展開）
- **未知変数怎么处理**（エラー vs 記号保持）

---

## 五、CTE 引擎与其他组件的交互

### 5.1 CTE の消費者

```
CTE は以下を生成する：

1. 型注釈位置
   Vec(factorial(5))        → CTE::eval(factorial(5)) → CTValue::Int(120)
   型置換：Vec(120)

2. ジェネリック値パラメータ
   Array(Int, factorial(3)) → CTE::eval(factorial(3)) → CTValue::Int(6)
   インスタンス化：Array(Int, 6)

3. Assert 型
   Assert(N > 0)            → CTE::eval(N > 0) → CTValue::Bool(true/false)
   True → Void, False → compile_error("N must be > 0")

4. If 条件型
   If(C, T, E)              → CTE::eval(C) → CTValue::Bool(b)
   True → T, False → E

5. Match 型族
   AsString(Int)            → match Int { Int => String, ... } → String
```

### 5.2 単態化との相互作用

```
単態化は以下の位置で CTE 結果を使用する：

1. 既知なジェネリック値パラメータ → 具体インスタンス生成
   List(Int) の push メソッド → push_List_Int 生成

2. 既知な値依存型 → 具体型に展開
   Matrix(Float, 3, 3).data → Array(Array(Float, 3), 3)

3. 部分評価 → 単相化コード生成
   map(Int, String) → map_Int_String 生成（T=Int, R=String はすでに固定）
```

### 5.3 ホーア論理検証器との相互作用

```
検証器は以下の位置で CTE を使用する：

1. 仕様式の一部評価
   //! requires: n > 0 && factorial(n) < MAX
   CTE::eval(factorial(n)) → n がコンパイル時既知 → 定数
                            → n が未知 → 記号として保持、SMT へ渡す

2. 仕様条件の簡略化
   //! ensures: result >= 0 && result < n
   CTE は既知な部分式を簡略化 пытается、SMT 求解の負担を軽減

3. 仕様型インスタンス化
   NonEmpty(n) = n > 0
   CTE は仕様型をブール式に展開
```

---

## 六、ホーア論理静的検証（RFC-022 実装設計）

### 6.1 仕様解析

`//!` と `/*! ... !*/` はパーサーにより特殊コメントノードとして認識され、AST に附加される：

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
┌─────────────┐     SMT-LIB 形式      ┌───────────┐
│  VC 生成器    │ ──────────────────→ │  Z3 ソルバ  │
└─────────────┘                       └─────┬─────┘
                                            │
                          ┌─────────────────┴──────────────┐
                          ↓                                ↓
                      unsat                            sat
                          ↓                                ↓
                    ┌──────────┐                   ┌──────────────┐
                    │ 検証通過  │                   │  反例モデル    │
                    │ 結果キャッシュ│                   │  抽出         │
                    └──────────┘                   │ 可読形式に変換│
                                                 └──────┬───────┘
                                                         ↓
                                                  ┌──────────────┐
                                                  │  コンパイル   │
                                                  │  エラーレポート│
                                                  │ • 入力値       │
                                                  │ • 違反した仕様  │
                                                  └──────────────┘
```

### 6.4 コンパイルモード

| モード | 動作 | CLI |
|------|------|-----|
| **Debug Build** | 仕様解析、VC 生成、Z3 証明呼び出し；検証通過後のみ Release Build 可能 | `yaoxiangc --debug` |
| **Release Build** | すべての `//!` コメントを無視、ゼロオーバーヘッド、検証キャッシュクリア | `yaoxiangc --release` |
| **Runtime Checks** | 仕様を `assert` 文に降級、違反時は panic | `yaoxiangc --runtime-checks` |

---

## 七、実装フェーズ

### Phase 1：定数畳み込み＋純粋性分析スケルトン

**目標**：最も基本的なコンパイル時評価をサポートし、CTE インフラを確立

**内容**：

- [ ] `CTValue` enum と `EvalEnv` 構造体の定義
- [ ] `eval()` の基本パス実装：リテラル、変数、二項演算、条件、コードブロック
- [ ] 純粋性分析器第一版実装：I/O 呼び出しを非純粋として識別その他はデフォルトで純粋
- [ ] 型チェッカーへの CTE 呼び出し点挿入（型注釈位置）
- [ ] 定数畳み込み：`1 + 2 * 3` をコンパイル時に `7` として計算
- [ ] 死分支除去：`if true { ... } else { ... }` → then 枝を直接採用
- [ ] ユニットテスト：リテラル評価、简单な式、定数畳み込み

**成果**：`src/middle/cte/` モジュール、`value.rs`、`eval.rs`、`purity.rs` を含む

### Phase 2：純粋関数コンパイル時評価＋終了判定

**目標**：純粋関数のコンパイル時完全評価をサポート

**内容**：

- [ ] 関数インライン評価実装：すべてのパラメータが既知 → 関数体を展開して評価
- [ ] `//! decreases` 解析と終了検証実装
- [ ] 再帰関数のコンパイル時評価実装（ステップ数制限付き）
- [ ] 評価結果キャッシュ（メモ化）実装
- [ ] 純粋性分析の整備：所有権情報を活用して `&mut` 副作用を識別
- [ ] 部分評価：一部パラメータが既知の場合のコード生成最適化
- [ ] 統合テスト：型位置での `factorial(5)` 評価 → `120`

**成果**：`src/middle/cte/interpreter.rs`、`src/middle/cte/termination.rs`

### Phase 3：型レベル計算

**目標**：`If`/`Assert`/`match` 型族をサポート

**内容**：

- [ ] `CTValue::Type(TypeId)` の型レベル操作実装
- [ ] `If: (C: Bool, T: Type, E: Type) -> Type` の条件型評価実装
- [ ] `Assert(C)` → `True → Void, False → compile_error` 実装
- [ ] 型レベル `match` 実装：`AsString: (T: Type) -> Type = match T { ... }`
- [ ] 値依存型の完全インスタンス化：`Matrix(Float, 3, 3)` → 具体型
- [ ] コンパイル時次元検証：行列乗算の次元不一致 → コンパイルエラー
- [ ] 単態化（mono pass）との統合

**成果**：`src/middle/cte/type_level.rs`、`src/middle/passes/mono/` 更新

### Phase 4：ホーア論理静的検証

**目標**：完全な仕様解析、VC 生成、SMT 検証通道

**内容**：

- [ ] パーサー拡張：`//!` と `/*! ... !*/` を仕様ノードとして認識
- [ ] 仕様型定義（`NonEmpty`、`Sorted`、`GreaterOrEqual` など標準ライブラリ仕様型）
- [ ] ユーザー定義仕様型サポート
- [ ] VC 生成器：最弱事前条件計算
- [ ] Z3 SMT ソルバ統合（`z3` crate 経由）
- [ ] SMT-LIB 形式翻訳
- [ ] 反例抽出と可読化レポート
- [ ] Debug/Release/RuntimeChecks の3つのコンパイルモード切り替え
- [ ] 統合テスト：`max`、`binary_search` などの関数の仕様検証

**成果**：`src/middle/verification/` モジュール

---

## 八、モジュール構成

```
src/middle/
├── cte/                          # コンパイル時計算エンジン
│   ├── mod.rs                    # CTE 入口、3サブシステム調整
│   ├── value.rs                  # CTValue 定義 + 基本操作
│   ├── eval.rs                   # 統一 AST 走査器（Interpreter trait + eval_ast）
│   ├── concrete.rs               # 具体評価実装（ConcreteInterpreter → CTValue）
│   ├── symbolic.rs               # 記号評価実装（SymbolicInterpreter → SMTExpr）
│   ├── env.rs                    # EvalEnv（評価環境 + ステップ数制限）
│   ├── purity.rs                 # 純粋性分析器
│   ├── termination.rs            # 終了判定器（decreases 検証）
│   ├── type_level.rs             # 型レベル計算（If/Assert/match 型族）
│   └── cache.rs                  # 評価結果キャッシュ
│
├── verification/                 # ホーア論理静的検証
│   ├── mod.rs                    # 検証入口
│   ├── spec_parser.rs            # //! 仕様解析
│   ├── spec_types.rs             # 組み込み仕様型定義
│   ├── vcgen.rs                  # 検証条件生成（WP 計算）
│   ├── smt.rs                    # Z3 SMT ソルバインターフェース
│   └── counterexample.rs         # 反例フォーマットの制定
│
└── passes/
    └── mono/                     # 既存の単態化（CTE 統合強化）
        └── ...                   # CTE 結果を使用したインスタンス化
```

---

## 九、主要な設計上の決定記録

| 決定 | オプション | 選択 | 理由 |
|------|------|------|------|
| 純粋性判定方式 | 明示的注釈 vs 自動推論 vs 两者結合 | **自動推論** | 所有権システムが十分な情報を提供；明示的注釈の逃げ道を提供しない |
| コンパイル時評価器 | 制約付きサブセット vs 完全言語 | **完全言語（ステップ数制限付き）** | 統一型構文の"すべてが `name: type = value`"と一貫性あり |
| 終了証明 | 强制注釈 vs 自動推論 | **型位置は强制、其他は自動** | 型位置が判定不能 = コンパイルエラー、他の场所は寛容可能 |
| VC 生成 | WP 計算 vs SP 計算 | **WP 計算** | WP の方がシンプルで直接的、エラー位置特定が明確 |
| SMT ソルバ | Z3 vs CVC5 vs 自前開発 | **Z3** | 最も成熟、Rust バインディングが最も完善、社区最大 |
| キャッシュ戦略 | キャッシュなし vs 跨モジュールキャッシュ | **LRU キャッシュ + 增量失效** | コンパイル時評価結果は決定性純粋関数であり、キャッシュに天然的适合 |
| コンパイルモード | 統一方針 vs Debug/Release 分離 | **Debug 検証 → Release ゼロオーバーヘッド** | 検証コストは高く、Release は負担不应 |

---

## 十、リスクと緩和

| リスク | 影響 | 緩和 |
|------|------|------|
| Z3 統合の複雑さ | Phase 4 遅延 | 成熟した `z3` crate を使用；まず簡単な算術からサポートし逐步的に расширять |
| コンパイル時評価タイムアウト | ユーザー体験の低下 | ステップ数制限 + 明確なタイムアウトエラーメッセージ + 式簡略化の提案 |
| 純粋性誤判定 | コンパイル時評価結果とランタイムの不一致 | 所有権システムが強力な保証を提供；誤判定した場合はコンパイラのバグであり、コンパイラを修正すべき |
| SMT 検証失敗のデバッグ困難 | ユーザーが仕様が成立しない理由を理解できない | 反例抽出 + 具体的な入力値表示 + 実行パスハイライト |
| コンパイル時間の 著増 | CI スローダウン | 增量検証 + モジュールレベルキャッシュ + 検証結果ファイル（`.o` ファイル类似） |

---

## 十一、既存 RFC との相互参照

| RFC | 関係 | 本計画如何に満足するか |
|------|------|---------------|
| RFC-010 §統一型構文 | CTValue はすべての型式をサポートする必要がある | `CTValue::Type(TypeId)` + `CTValue::Struct` でカバー |
| RFC-011 §4.2 コンパイル時計算 | 値依存型の中核メカニズム | Phase 2/3 で実装 |
| RFC-011 §6 型レベル計算 | `If`/`Assert`/`match` 型族 | Phase 3 で実装 |
| RFC-011 §終了判定メカニズム | decreases 仕様 | Phase 2 の終了判定器で実装 |
| RFC-022 §1 仕様コメント構文 | `//!` 解析 + 仕様型 | Phase 4 で実装 |
| RFC-022 §3 検証メカニズム | VC 生成 + SMT 連携 | Phase 4 の VCGen + SMT モジュールで実装 |
| RFC-009 §所有権モデル | 純粋性分析の基盤 | Phase 1/2 で所有権情報を再利用 |

---

## 参考文献

- [RFC-010: 統一型構文](../design/rfc/accepted/010-unified-type-syntax.md)
- [RFC-011: ジェネリクスシステム設計](../design/rfc/accepted/011-generic-type-system.md)
- [RFC-022: ホーア論理静的検証](../design/rfc/draft/022-hoare-logic-static-verification.md)
- [RFC-009: 所有権モデル](../design/rfc/accepted/009-ownership-model.md)
- [Z3 Prover](https://github.com/Z3Prover/z3)
- [SMT-LIB Standard](https://smtlib.cs.uiowa.edu/)
- [Weakest Precondition Calculus (Dijkstra)](https://en.wikipedia.org/wiki/Predicate_transformer_semantics)