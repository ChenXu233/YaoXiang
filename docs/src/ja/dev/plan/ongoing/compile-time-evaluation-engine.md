# コンパイル時計算エンジン（CTE Engine）とホーア論理静的検証 — 実装計画

> **タスク**：コンパイル時評価エンジン + ホーア論理静的検証通道を実装し、値依存型とコンパイル時次元検証を支える
> **ベース**：RFC-010（統一型構文）、RFC-011（ジェネリックシステム/値依存型）、RFC-022（ホーア論理静的検証）
> **日付**：2026-05-10
> **状態**：設計中
> **目標マイルストーン**：
> - M1：定数畳み込み + 純度分析スケルトン
> - M2：純粋関数のコンパイル時評価 + 終了チェック
> - M3：型レベル計算（`If`/`Assert`/`match` 型族）
> - M4：ホーア論理検証（`//!` 仕様解析 → VC 生成 → SMT 連携）

---

## 摘要

YaoXiang の値依存型（RFC-011）は、型がコンパイル時に既知の値に依存できることを要求する（例：`Vec(factorial(5))` → `Vec(120)`）。また、ホーア論理静的検証（RFC-022）は純粋関数に対するコンパイル時仕様チェックを要求する。両者に共通する中核的な要求は、**コンパイル時に純粋関数を安全に実行/分析する**ことである。

本計画は**統一コンパイル時計算エンジン（CTE Engine）**を提案し、純度分析、終了チェック、式評価を共通インフラストラクチャとして抽象化し、型レベル評価とホーア論理検証という2つのコンシューマにそれぞれサービスを提供する。

---

## 中核的設計原則

1. **所有権システムを使った純度分析の复用**：YaoXiang の `&mut` は副作用のマークアップである——借用チェックにより、何が変更されるかがすでにわかる
2. **純粋関数 = コンパイル時評価可能**：`const fn` キーワード不要、コンパイラが純度を自動推測
3. **終了証明 = 型安全の基盤**：型位置の評価は終了を証明する必要がある（`decreases` 仕様）、そうでなければ型システムは決定不能
4. **部分的評価优于全量評価**：いくつのパラメータが既知かによって評価し、計算できないものは実行時に残す
5. **コンパイル時評価とホーア論理はインタープリタを共有**：同じ式評価コアが2つのコンシューマを支える
6. **双方向評価モード**：具体評価（既知パラメータ → `CTValue` 出力）と記号評価（未知パラメータ → `SMTExpr` 出力）が同じインタープリタフレームワークを共有し、評価環境のみ異なる

---

## アーキテクチャ概要

```
                             ソースファイル + //! 仕様
                                    ↓
                            ┌──────────────┐
                            │   パーサー     │
                            │ (//! コメント識別)│
                            └──────┬───────┘
                                   ↓
                            ┌──────────────┐
                            │  型チェッカー  │
                            │ • 仕様収集     │
                            │ • 値依存発見   │
                            └──────┬───────┘
                                   ↓
              ┌────────────────────┴────────────────────┐
              ↓                                         ↓
   ┌──────────────────────┐                ┌──────────────────────┐
   │   CTE エンジン        │                │  ホーア論理検証器     │
   │                      │                │                      │
   │  ┌────────────────┐  │                │  1. //! 仕様収集      │
   │  │  純度分析器      │  │                │  2. 検証条件(VC)生成  │
   │  │  (所有権ベース)  │  │                │  3. SMT ソルバ(Z3)    │
   │  └───────┬────────┘  │                │  4. 反例レポート      │
   │  ┌───────┴────────┐  │                └──────────┬───────────┘
   │  │  終了チェッカー  │  │                           │
   │  │  (decreases)    │  │                           ↓
   │  └───────┬────────┘  │                ┌──────────────────────┐
   │  ┌───────┴────────┐  │                │  検証結果            │
   │  │  AST インタープリタ│ │                │  • 通過 → キャッシュ  │
   │  │                │  │                │  • 失敗 → ブロック   │
   │  │ ┌────────────┐ │  │                │      release        │
   │  │ │ 具体評価    │ │  │                └──────────────────────┘
   │  │ │ env: 全既知 │ │  │                       ↑
   │  │ │ → CTValue  │ │  │                       │
   │  │ └────────────┘ │  │  共有インタープリタ     │
   │  │ ┌────────────┐ │  │  (AST走査/インライン/   │
   │  │ │ 記号評価    │─┼──┼───────────────────────┘
   │  │ │ env: 部分既知│ │  │  (ホーア論理コンシューマ)
   │  │ │ → SMTExpr  │ │  │
   │  │ └────────────┘ │  │
   │  └───────┬────────┘  │
   │          ↓           │
   │  結果埋め込み型/単態化 │
   └──────────────────────┘
```

---

## 一、コンパイル時値（CTValue）

コンパイル時計算の中核データ型。コンパイラ内部 IR 値として設計され、実行時値と混同しない。

```rust
/// コンパイル時評価結果
enum CTValue {
    /// 整数（Bool のコンパイル時の用途をすべて含む）
    Int(i64),

    /// 浮動小数点数
    Float(f64),

    /// 文字列（エラーメッセージ、型名など）
    String(SmolStr),

    /// 型参照——型レベル計算の中核
    /// YaoXiang の型自身は Type1 層の「値」
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

    /// 未評価の関数参照（部分的評価時に保持）
    /// すべてのパラメータが既知の場合はインライン評価、
    /// そうでなければ実行時呼び出しとして保持
    Thunk {
        func: FunctionId,
        known_args: Vec<CTValue>,
        unknown_params: Vec<ParamId>,
    },
}
```

**重要な設計**：`CTValue::Type(TypeId)` により、型は一等コンパイル時値となる。`If(C, T, E)` の C を評価すると `CTValue::Bool` となり、T/E を評価すると `CTValue::Type` となる。

---

## 二、サブシステム 1：純度分析器

### 2.1 設計思路

YaoXiang の所有権システム（RFC-009）を再利用し、副作用は型署名で自然に表現される：

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
    // 1. 高速パス：すでにアノテーション済み
    if ctx.has_purity_annotation(func):
        return ctx.get_annotation(func)

    // 2. 直接副作用をチェック
    for op in func.body.operations():
        match op:
            Call(callee, _) if is_io_operation(callee):
                return Impure("I/O operation")
            Call(callee, args) where has_mut_arg(args):
                if arg_escapes_function(args):
                    return Impure("&mut を通じた外部状態の変更")
            Call(callee, _):
                // 推移性：呼び出し先も純粋関数である必要がある
                if analyze_purity(callee, ctx).is_impure():
                    return Impure("impure 関数呼び出し: {callee}")

    // 3. デフォルトで純粋関数と推測
    return Pure
```

### 2.3 明示的な純度アノテーションは提供しない

**設計判断：`//! pure` などの明示的アノテーションは提供しない。**

所有権システム（RFC-009）はすでに型署名を通じて副作用情報を表現している——`&mut T` は変更、`I/O` 操作は外部副作用である。コンパイラは純度を自動推測できる。

コンパイラが純粋関数を不純と誤判定した場合、それはコンパイラのバグであり、ユーザーがパッチを打つのではなく、コンパイラを修正すべきである。「この関数は純粋だ、信じてくれ」というアノテーションを提供，只会掩盖真实问题，只会带来兼容コード、回退、临时、备用、特定モード生效の代码的产生。

> *「兼容、回退、临时、备用、特定モード生效の代码。让问题直接暴露.」*

### 2.4 RFC-022 との関係

純度分析器は同時に以下に使用される：

- **CTE**：非純粋関数は型位置で使用不可
- **ホーア論理**：仕様式（`requires`/`ensures` の右辺）は純粋関数呼び出しである必要がある

---

## 三、サブシステム 2：終了チェッカー

### 3.1 設計思路

型位置のコンパイル時評価は終了を保証する必要がある、さもなくば型システムは決定不能となる。YaoXiang は `//! decreases` 仕様で終了を証明する。

```
//! decreases: <expr>
```

ここで `<expr>` は下限を持つ整列値（通常是自然数などの `Int` 型）である。

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

            // 3. 下限を検証
            if !has_lower_bound(decreases):
                return TermError::NoLowerBound

            return TermOk
```

### 3.3 自動推測

明らかな終了状況はアノテーション不要：

```yaoxiang
// decreases 不要——コンパイラはループが既知の上界 n を持つことを確認
sum: (arr: Array(Int, n)) -> Int = {
    s = 0; i = 0
    while i < n { s += arr[i]; i += 1 }
    return s
}
```

アノテーションが必要な状況：
```yaoxiang
// decreases アノテーション必須——再帰呼び出し n-1
factorial: (n: Int) -> Int = {
    //! decreases: n
    if n <= 1 { return 1 }
    return n * factorial(n - 1)
}
```

### 3.4 RFC-022 との関係

終了チェッカーは同時に以下に使用される：

- **CTE**：decreases はコンパイル時評価の入場料
- **ホーア論理**：ループ不変式の decreases 変種（`/*! decreases: n - i !*/`）も終了チェッカーで検証

---

## 四、サブシステム 3：AST インタープリタ

### 4.1 設計思路

インタープリタは AST 走査に基づき、評価環境（変数名 → CTValue マッピング）を維持する。中核的能力は**部分的評価**：既知パラメータは計算し、未知パラメータは保持する。

```
eval(expr: &Expr, env: &mut EvalEnv) -> EvalResult<CTValue>:
    match expr:
        // リテラル → 直接変換
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
                // 部分既知 → 部分的評価（単相化コードを生成）
                partial_eval(func, known_args, env)
            else:
                // すべて未知 → Thunk
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

すべてのパラメータが既知の場合、インタープリタは関数体を評価コンテキストにインライン化する：

```
inline_and_eval(func, args, env):
    // 1. 純度をチェック
    purity.check(func)?

    // 2. キャッシュをチェック
    if let Some(cached) = cache.get(func, args):
        return cached

    // 3. インライン環境を作成
    let mut inline_env = env.child()
    for (param, arg) in func.params.zip(args):
        inline_env.bind(param.name, arg)

    // 4. 関数体を評価
    let result = eval(&func.body, &mut inline_env)?

    // 5. 結果をキャッシュ
    cache.insert(func, args, result.clone())
    result
```

### 4.3 ステップ数制限

コンパイル時評価には硬性制限が必要であり、`decreases` があっても意図せずタイムアウトするシナリオを防ぐ：

```
const MAX_EVAL_STEPS: u64 = 1_000_000;  // 100万ステップの硬性上限

struct EvalEnv {
    variables: HashMap<SmolStr, CTValue>,
    step_count: u64,
    step_limit: u64,
}
```

### 4.4 双方向評価モード：具体 vs 記号

インタープリタの中核フレームワーク（AST 走査、インライン展開、パターンマッチ）は統一しているが、**評価環境**により2つのモードが決まる：

#### 4.4.1 具体評価（Concrete Evaluation）

**コンシューマ**：CTE エンジン → 型レベル評価、単態化

**特徴**：
- 環境内のすべての変数に具体的な `CTValue` がある
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

**コンシューマ**：ホーア論理検証器 → SMT ソルバ

**特徴**：
- 環境に**記号変数**が存在する（関数パラメータ `n`、`arr` など、コンパイル時に未知）
- 既知のサブ式は具体値として評価し、未知部分は SMT 記号として保持
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

#### 4.4.3 2つのモードの主な違い

| 次元 | 具体評価 | 記号評価 |
|------|----------|----------|
| 環境 | `HashMap<Name, CTValue>` | `HashMap<Name, SMTTerm>` |
| 変数が未知の場合 | エラー | 記号として保持 |
| 関数呼び出し | インライン化 + 関数体実行 | 論理定義に展開（実行しない） |
| ループ | 実際の反復（ステップ数制限あり） | ループ不変式 VC に変換 |
| 出力型 | `Result<CTValue, CTError>` | `Result<SMTExpr, SMError>` |
| 失敗のセマンティクス | コンパイルエラー | 検証失敗（Runtime Check に降級可能） |
| パフォーマンス特性 | 高速（直接計算） | 低速（SMT 求解） |

#### 4.4.4 共有インタープリタフレームワーク

2つのモードは同一の AST 走査スケルトンを共有：

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
        // ... 其余 AST ノード同理
    }
}
```

**重要な洞察**：具体評価と記号評価の AST 走査ロジックは完全に同一であり、違いは次の点のみである：

- **値をどう表現するか**（`CTValue` vs `SMTExpr`）
- **関数呼び出しをどう処理するか**（インライン実行 vs 論理展開）
- **未知変数をどう処理するか**（エラー vs 記号保持）

---

## 五、CTE エンジンと他のサブシステムの連携

### 5.1 型チェック器との連携

CTE は型チェック器の以下の位置で使用される：

1. 型アノテーション位置
   `Vec(factorial(5))` → CTE::eval(factorial(5)) → CTValue::Int(120)
   型置換为 Vec(120)

2. ジェネリック値パラメータ
   `Array(Int, factorial(3))` → CTE::eval(factorial(3)) → CTValue::Int(6)
   インスタンス化 `Array(Int, 6)`

3. Assert 型
   `Assert(N > 0)` → CTE::eval(N > 0) → CTValue::Bool(true/false)
   True → Void, False → compile_error("N must be > 0")

4. If 条件型
   `If(C, T, E)` → CTE::eval(C) → CTValue::Bool(b)
   True → T, False → E

5. Match 型族
   `AsString(Int)` → match Int { Int => String, ... } → String

### 5.2 単態化との連携

```
単態化は CTE 結果を以下の位置で使用する：

1. 既知のジェネリック値パラメータ → 具体的なインスタンスを生成
   List(Int) の push メソッド → push_List_Int を生成

2. 既知の値依存型 → 具体的な型に展開
   Matrix(Float, 3, 3).data → Array(Array(Float, 3), 3)

3. 部分的評価 → 単相化コードを生成
   map(Int, String) → map_Int_String を生成、T=Int, R=String はすでに固定
```

### 5.3 ホーア論理検証器との連携

```
検証器は CTE を以下の位置で使用する：

1. 仕様式の部分的評価
   //! requires: n > 0 && factorial(n) < MAX
   CTE::eval(factorial(n)) → n がコンパイル時に既知の場合 → 定数
                            → n が未知の場合 → 記号として保持、SMT に渡す

2. 仕様条件の簡略化
   //! ensures: result >= 0 && result < n
   CTE は既知のサブ式を簡略化 пыта，减轻 SMT 求解负担

3. 仕様型インスタンス化
   NonEmpty(n) = n > 0
   CTE は仕様型をブール式に展開
```

---

## 六、ホーア論理静的検証（RFC-022 実装設計）

### 6.1 仕様解析

`//!` と `/*! ... !*/` はパーサーによって特殊コメントノードとして識別され、AST に附加される：

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

最弱前置条件（Weakest Precondition）計算を使用：

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
        // ループに入る前に成立
        vcs.push(VC::InvariantEntry { loop_, invariant })
        // 各反復で保持
        vcs.push(VC::InvariantPreservation { loop_, invariant })
        // 退出後に後置条件を蕴含
        vcs.push(VC::InvariantExit { loop_, invariant, post: ensures })

    vcs
```

### 6.3 SMT ソルバ統合

```
┌─────────────┐     SMT-LIB 形式      ┌───────────┐
│  VC 生成器   │ ──────────────────→ │  Z3 ソルバ │
└─────────────┘                       └─────┬─────┘
                                            │
                          ┌─────────────────┴──────────────┐
                          ↓                                ↓
                      unsat                            sat
                          ↓                                ↓
                    ┌──────────┐                   ┌──────────────┐
                    │ 検証通過  │                   │ 反例モデル抽出│
                    │ 結果キャッシュ│                   │ 読み取り可能な│
                    └──────────┘                   │ 形式に変換   │
                                                  └──────┬───────┘
                                                         ↓
                                                  ┌──────────────┐
                                                  │ コンパイルエラー│
                                                  │ レポート        │
                                                  │ • 入力値       │
                                                  │ • 違反した仕様   │
                                                  └──────────────┘
```

### 6.4 コンパイルモード

| モード | 動作 | CLI |
|------|------|-----|
| **Debug Build** | 仕様を解析し、VC を生成し、Z3 で証明；検証通過後に Release Build 可能 | `yaoxiangc --debug` |
| **Release Build** | すべての `//!` コメントを無視、ゼロオーバーヘッド、検証キャッシュをクリア | `yaoxiangc --release` |
| **Runtime Checks** | 仕様を `assert` 文に降級、違反時に panic | `yaoxiangc --runtime-checks` |

---

## 七、実装フェーズ

### Phase 1：定数畳み込み + 純度分析スケルトン

**目標**：CTE インフラストラクチャを確立し、基本的なコンパイル時評価をサポート

**内容**：
- [ ] `CTValue` enum と `EvalEnv` 構造体を定義
- [ ] `eval()` の基本パス实现：リテラル、変数、二項演算、条件、コードブロック
- [ ] 純度分析器の第一版実装：I/O 呼び出しを不純として識別し、他はデフォルトで純粋
- [ ] 型チェッカーに CTE 呼び出しポイントを挿入（型アノテーション位置）
- [ ] 定数畳み込み：`1 + 2 * 3` をコンパイル時に `7` として計算
- [ ] デッドブランチ除去：`if true { ... } else { ... }` → then ブランチを直接使用
- [ ] ユニットテスト：リテラル評価、简单な式、定数畳み込み

**成果物**：`src/middle/cte/` モジュール、`value.rs`、`eval.rs`、`purity.rs` を含む

### Phase 2：純粋関数のコンパイル時評価 + 終了チェック

**目標**：純粋関数のコンパイル時完全な評価をサポート

**内容**：
- [ ] 関数インライン評価実装：すべてのパラメータが既知 → 関数体を展開して評価
- [ ] `//! decreases` 解析と終了検証を実装
- [ ] 再帰関数のコンパイル時評価実装（ステップ数制限あり）
- [ ] 評価結果キャッシュ（Memoization）実装
- [ ] 純度分析器を完善：所有権情報を活用して `&mut` 副作用を識別
- [ ] 部分的評価：一部パラメータが既知の場合のコード生成最適化
- [ ] 統合テスト：`factorial(5)` が型位置で `120` として評価される

**成果物**：`src/middle/cte/interpreter.rs`、`src/middle/cte/termination.rs`

### Phase 3：型レベル計算

**目標**：`If`/`Assert`/`match` 型族をサポート

**内容**：
- [ ] `CTValue::Type(TypeId)` の型レベル操作を実装
- [ ] `If: (C: Bool, T: Type, E: Type) -> Type` の条件型評価を実装
- [ ] `Assert(C)` → `True → Void, False → compile_error` を実装
- [ ] 型レベル `match` を実装：`AsString: (T: Type) -> Type = match T { ... }`
- [ ] 値依存型の完全なインスタンス化：`Matrix(Float, 3, 3)` → 具体的な型
- [ ] コンパイル時次元検証：行列積の次元不一致 → コンパイルエラー
- [ ] 単態化（mono pass）との統合

**成果物**：`src/middle/cte/type_level.rs`、`src/middle/passes/mono/` の更新

### Phase 4：ホーア論理静的検証

**目標**：完全な仕様解析、VC 生成、SMT 検証通道

**内容**：
- [ ] パーサー拡張：`//!` と `/*! ... !*/` を仕様ノードとして識別
- [ ] 仕様型定義（`NonEmpty`、`Sorted`、`GreaterOrEqual` などの標準ライブラリ仕様型）
- [ ] ユーザー定義仕様型サポート
- [ ] VC 生成器：最弱前置条件計算
- [ ] Z3 SMT ソルバ統合（`z3` crate を使用）
- [ ] SMT-LIB 形式翻訳
- [ ] 反例抽出と読み取り可能なレポート
- [ ] Debug/Release/RuntimeChecks の3つのコンパイルモード切り替え
- [ ] 統合テスト：`max`、`binary_search` などの関数の仕様を検証

**成果物**：`src/middle/verification/` モジュール

---

## 八、モジュール計画

```
src/middle/
├── cte/                          # コンパイル時計算エンジン
│   ├── mod.rs                    # CTE 入口、3サブシステムの調整
│   ├── value.rs                  # CTValue 定義 + 基本操作
│   ├── eval.rs                   # 統一 AST 走査器（Interpreter trait + eval_ast）
│   ├── concrete.rs               # 具体評価実装（ConcreteInterpreter → CTValue）
│   ├── symbolic.rs               # 記号評価実装（SymbolicInterpreter → SMTExpr）
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
│   └── counterexample.rs         # 反例フォーマットの問題
│
└── passes/
    └── mono/                     # 既存の単態化（CTE 統合強化）
        └── ...                   # CTE 結果を使用してインスタンス化
```

---

## 九、主要設計判断記録

| 判断 | オプション | 選択 | 理由 |
|------|------|------|------|
| 純度判定方式 | 明示的アノテーション vs 自動推測 vs 両者結合 | **自動推測** | 所有権システムは十分な情報を提供；明示的アノテーションの逃げ道を提供しない |
| コンパイル時評価器 | 制限付きサブセット vs 完全言語 | **完全言語（ステップ数制限付き）** | 統一型構文の「すべては `name: type = value`」と一致 |
| 終了証明 | 強制アノテーション vs 自動推測 | **型位置は強制、他は自動** | 型位置が決定不能 = コンパイルエラー、他は緩くても可 |
| VC 生成 | WP 計算 vs SP 計算 | **WP 計算** | WP はより简单で直接、エラー位置が明確 |
| SMT ソルバ | Z3 vs CVC5 vs 自前開発 | **Z3** | 最も成熟、Rust binding が最も完善、社区最大 |
| キャッシュ戦略 | キャッシュなし vs クロスモジュールキャッシュ | **LRU キャッシュ + 増分失效** | コンパイル時評価結果は确定的純粋関数であり、キャッシュに天生的に適している |
| コンパイルモード | 統一モード vs Debug/Release 分離 | **Debug 検証 → Release ゼロオーバーヘッド** | 検証コストは高く、Release は負担するべきではない |

---

## 十、リスクと 완화

| リスク | 影響 | 緩和 |
|------|------|------|
| Z3 統合が複雑 | Phase 4 遅延 | 成熟した `z3` crate を使用；まず簡単な算術をサポートし、少しずつ拡張 |
| コンパイル時評価タイムアウト | ユーザー体験が悪い | ステップ数制限 + 明確なタイムアウトエラーメッセージ + 式の簡略化を推奨 |
| 純度誤判定 | コンパイル時評価結果と実行時が不一致 | 所有権システムが強力な保証を提供；誤判定した場合はコンパイラのバグであり、コンパイラを修正すべき |
| SMT 検証失敗のデバッグが難しい | ユーザーがなぜ仕様が成立しないかわからない | 反例抽出 + 具体的な入力値表示 + 実行パスハイライト |
| コンパイル時間が著しく増加 | CI が遅くなる | 増分検証 + モジュールレベルキャッシュ + 検証結果ファイル（`.o` ファイル类似） |

---

## 十一、既存 RFC への相互参照

| RFC | 関係 | 本計画による対応 |
|------|------|---------------|
| RFC-010 §統一構文 | CTValue はすべての型式をサポートする必要がある | `CTValue::Type(TypeId)` + `CTValue::Struct` で対応 |
| RFC-011 §4.2 コンパイル時計算 | 値依存型の中核メカニズム | Phase 2/3 で実装 |
| RFC-011 §6 型レベル計算 | `If`/`Assert`/`match` 型族 | Phase 3 で実装 |
| RFC-011 §終了チェックメカニズム | decreases 仕様 | Phase 2 の終了チェッカーで実装 |
| RFC-022 §1 仕様コメント構文 | `//!` 解析 + 仕様型 | Phase 4 で実装 |
| RFC-022 §3 検証メカニズム | VC 生成 + SMT 連携 | Phase 4 の VCGen + SMT モジュールで実装 |
| RFC-009 §所有権モデル | 純度分析の基盤 | Phase 1/2 で所有権情報を再利用 |

---

## 参考文献

- [RFC-010: 統一型構文](../design/rfc/accepted/010-unified-type-syntax.md)
- [RFC-011: ジェネリックシステム設計](../design/rfc/accepted/011-generic-type-system.md)
- [RFC-022: ホーア論理静的検証](../design/rfc/draft/022-hoare-logic-static-verification.md)
- [RFC-009: 所有権モデル](../design/rfc/accepted/009-ownership-model.md)
- [Z3 Prover](https://github.com/Z3Prover/z3)
- [SMT-LIB Standard](https://smtlib.cs.uiowa.edu/)
- [Weakest Precondition Calculus (Dijkstra)](https://en.wikipedia.org/wiki/Predicate_transformer_semantics)