# 所有権チェック 既知の問題

> 最終更新：2026-06-16
> 実装場所：`src/frontend/core/typecheck/layers/ownership.rs`
> テスト場所：`src/frontend/core/typecheck/layers/tests/ownership.rs`
> 61 tests、0 failures

## 正確性の欠陥

- [x] ### 1. ref のエイリアスが spawn に入りエスケープを漏れ検出（P0）— 修正済み (2026-06-15)

**シナリオ**：
```yaoxiang
shared = ref x
alias = shared       // shared Move → alias
spawn { use(alias) } // alias ∉ ref_vars → エスケープを漏れ検出 → Rc を選択（原子ではない、スレッド間安全でない）
```

**根本原因**：`OwnershipChecker` は `Expr::Ref` で直接代入された変数名（`ref_vars`）のみを追跡する。ref 変数が中間変数に Move された後、中間変数は `ref_vars` を変更しない。

**影響**：spawn をまたいで使用される ref が誤って `RcNew` にコンパイルされる可能性があり、原子ではない参照カウントはスレッド間でデータ競合を引き起こす可能性がある。

**修正**：`StmtKind::Var` および `BinOp::Assign` ハンドラにおいて、右辺が `Expr::Var(name)` で `name ∈ ref_vars` の場合、左辺のターゲットを `ref_vars` に追加する（commit `9029d5b`）。

- [x] ### 2. spawn キャプチャ変数の Move 後も外層が引き続き使用可能（P1）— 修正済み (2026-06-16)

**シナリオ**：
```yaoxiang
shared = ref data
spawn { a = shared }  // spawn 本体 walk（save/restore）→ shared が本体内で Moved → 外層は復元
use(shared)            // 外層の shared は依然 Alive——正しいが、spawn 本体内で shared は既に Move
```

**根本原因**：`Expr::Spawn` は save/restore を使用し、spawn 本体内での所有権変更は外層に影響しない。これは正しい設計だが、spawn 本体内での `a = shared` の Move は spawn の「一時的な walk」内でのみ検出される。spawn 本体が `shared` の Move を実行した場合、save/restore によって外層は復元されるが、**spawn 後に外層が `shared` を引き続き使用することを阻止するものが何もない**。

**影響**：spawn が実際に実行時に `shared` を Move した場合（例：`a = shared`）、外層コードは spawn 後も `shared` にアクセスできる——これは YaoXiang の並行モデルでは正しい可能性がある（spawn が独立したコピーを取得する）が、意味は明確に定義されていない。

**修正方針**：言語仕様を明確化する必要がある：spawn がキャプチャする Move の意味が外層スコープに影響するかどうか。「spawn が独立したコピーを取得する」の場合、現在の動作は正しい。「spawn が所有権を消費する」の場合、save/restore を削除するか、クロージャのような Captures を導入する必要がある。

## 精度のトレードオフ

- [x] ### 3. 分岐の排他性が保守的に衝突を報告（P1）— 修正済み (2026-06-16)

**シナリオ**：
```yaoxiang
if cond {
    a = &mut x   // 分岐 A
} else {
    b = &mut x   // 分岐 B
}
// 理論上：A と B は排他であり、衝突すべきでない
// 実際：2 つの WriteToken が順次作成される → BorrowConflict を報告
```

**根本原因**：`NLL without fixpoint` アーキテクチャの制限——単一パスの AST walk ではパス条件をモデル化できず、分岐の排他性と逐次実行を区別できない。

**修正方針**：CFG の SMT スローパスの介入が必要（現在 `smt_cut` は実装済みだが、`while + path_condition` のシナリオでのみトリガーされる）。if/else 分岐への拡張には、path_condition を Borrow ハンドラに伝播する必要がある。

- [ ] ### 4. ref 型が Dup を認識しない（P1）

**シナリオ**：
```yaoxiang
shared = ref x
a = shared    // Move——しかし ref は理論上 Dup 型であり、コピー可能であるべき
b = shared    // use after move——実際には許可されるべき
```

**根本原因**：所有権チェッカは `ref T` が Dup 型（コピー可能な参照カウントハンドル）であることを認識しない。`StmtKind::Var` の Move ロジックはすべての型を区別なく扱う。

**影響**：ref 値のセマンティクスが予想より厳格になる——RFC-009 の設計のように「自由にコピー」することができない。

**修正方針**：`TypeEnvironment` から変数の型を照会し、Dup 型に対して Move ロジックをスキップする必要がある。これは `clone()` の明示的呼び出しを要求する全体設計と一致する——現在の保守的な動作は正しいセマンティクスより緩くはない。

## インフラ

- [ ] ### 5. エラーコード形式が未統一（P2）

**説明**：フロントエンド所有権チェッカは `DisproofModel.into_diagnostic()` を使用し、エラーコード E2014-E2020 を生成する。Middle 層に残っている `lifetime/error.rs` は独立した `ValueState` + `Checker` trait を使用する。2 つのシステムが現在共存している。

**修正方針**：middle 層 `error.rs` の `ValueState` と `Checker` trait を削除し（参照は 2 つのみ：`lifecycle.rs` と `cycle_check` テスト）、フロントエンドエラーコードシステムに統一する。

- [ ] ### 6. ネスト関数の引数付き形式が解析されない（P2）

**説明**：`StmtKind::Binding` は `params.is_empty() && !body.is_empty()` のクロージャに対してのみキャプチャ解析を行う。引数付きネスト関数は `vec![]` を返す（`check_module` が body を独立にチェックするが、キャプチャセマンティクスは解析しない）。

**影響**：引数付きネスト関数本体内での所有権エラーが検出されず（現在は直接 skip）、キャプチャ情報も生成されない。引数付きネスト関数が外層変数を使用した場合、所有権セマンティクスが解析されない。

**修正方針**：引数付き/引数なしの Binding を統一的に処理し、その body に対して `check_function` とキャプチャ解析を同時に行う。