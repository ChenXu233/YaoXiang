```markdown
# 所有権チェックの既知の問題

> 最終更新：2026-06-16
> 実装場所：`src/frontend/core/typecheck/layers/ownership.rs`
> テスト場所：`src/frontend/core/typecheck/layers/tests/ownership.rs`
> 61 tests, 0 failures

## 正確性の欠陥

- [x] ### 1. ref のエイリアスが spawn に入るとエスケープを検出し損なう（P0）— 修正済み (2026-06-15)

**シナリオ**：
```yaoxiang
shared = ref x
alias = shared       // shared Move → alias
spawn { use(alias) } // alias ∉ ref_vars → エスケープの検出漏れ → Rc を選択（非アトミック、スレッド越えで安全でない）
```

**根本原因**：`OwnershipChecker` は `Expr::Ref` で直接代入された変数名（`ref_vars`）のみを追跡する。ref 変数が中間変数へ Move された場合、中間変数は `ref_vars` に追加されない。

**影響**：spawn を越えて使用される ref が誤って `RcNew` にコンパイルされる可能性があり、非アトミックな参照カウントはスレッド越えでデータ競合を引き起こす可能性がある。

**修正**：`StmtKind::Var` および `BinOp::Assign` のハンドラにおいて、右辺が `Expr::Var(name)` かつ `name ∈ ref_vars` の場合、左辺のターゲットを `ref_vars` に追加する（commit `9029d5b`）。

- [x] ### 2. spawn でキャプチャされた変数が Move された後も外側で使用可能（P1）— 修正済み (2026-06-16)

**シナリオ**：
```yaoxiang
shared = ref data
spawn { a = shared }  // spawn 本体を walk（save/restore）→ shared は本体内で Moved → 外側は復元
use(shared)            // 外側の shared は依然 Alive — 正しいが、spawn 本体内では shared は Move 済み
```

**根本原因**：`Expr::Spawn` は save/restore を使用し、spawn 本体内での所有権変更は外側に影響しない。これは正しい設計だが、spawn 本体内での `a = shared` の Move は spawn の「一時的 walk」内でのみ検出される。spawn 本体が `shared` の Move を実行した場合、save/restore により外側が復元されるが、**spawn 後に外側コードが `shared` の使用を続行することを阻止する仕組みがない**。

**影響**：spawn が実行時に `shared` を Move した場合（`a = shared` のように）、spawn 後も外側コードは `shared` にアクセスできる — YaoXiang の並行モデルではこれが正しい可能性がある（spawn は独立したコピーを取得する）が、意味論が明確に定義されていない。

**修正方針**：言語仕様を明確化する必要がある：spawn がキャプチャする Move の意味論が外側のスコープに影響するかどうか。「spawn が独立したコピーを取得する」の場合、現在の挙動は正しい。「spawn が所有権を消費する」の場合、save/restore を除去するか、クロージャのような Captures を導入する必要がある。

## 精度のトレードオフ

- [ ] ### 3. 分岐の排他性を保守的に競合と報告する（P1）

**シナリオ**：
```yaoxiang
if cond {
    a = &mut x   // 分岐 A
} else {
    b = &mut x   // 分岐 B
}
// 理論上：A と B は排他なので競合しないはず
// 実際：2 つの WriteToken が順次作成される → BorrowConflict を報告
```

**根本原因**：`NLL without fixpoint` アーキテクチャの制限 — 単一パスの AST walk ではパスの条件がモデル化されず、分岐の排他性か順次実行かを区別できない。

**修正方針**：CFG の SMT 低速経路の介入が必要である（現在 `smt_cut` は実装済みだが `while + path_condition` のシナリオでのみトリガーされる）。if/else 分岐への拡張には、path_condition を Borrow ハンドラへ伝播させる必要がある。

- [ ] ### 4. ref 型が Dup として認識されない（P1）

**シナリオ**：
```yaoxiang
shared = ref x
a = shared    // Move — しかし ref は理論上 Dup 型なのでコピー可能
b = shared    // move の後の使用 — 実際には許可されるべき
```

**根本原因**：所有権チェッカーは `ref T` が Dup 型（コピー可能な参照カウントハンドル）であることを認識していない。`StmtKind::Var` の Move ロジックはすべての型を区別なく扱う。

**影響**：ref 値の意味論は予想より厳格であり、RFC-009 で設計された「自由にコピー」のように振る舞わない。

**修正方針**：`TypeEnvironment` から変数の型を問い合わせ、Dup 型には Move ロジックをスキップする必要がある。これは `clone()` の明示的な呼び出しを要求する全体的な設計と一貫している — 現在の保守的な挙動は正しい意味論より寛容ではない。

## インフラ

- [ ] ### 5. エラーコード形式が統一されていない（P2）

**説明**：フロントエンドの所有権チェッカーは `DisproofModel.into_diagnostic()` を使用し、エラーコードは E2014-E2020。Middle 層に残る `lifetime/error.rs` は独立した `ValueState` + `Checker` trait を使用する。2 つのシステムが現在併存している。

**修正方針**：Middle 層 `error.rs` の `ValueState` と `Checker` trait を削除する（参照は 2 つのみ：`lifecycle.rs` と `cycle_check` テスト）、フロントエンドのエラーコードシステムに統一する。

- [ ] ### 6. パラメータ付きネスト関数が解析されない（P2）

**説明**：`StmtKind::Binding` は `params.is_empty() && !body.is_empty()` のクロージャのみキャプチャ解析を行う。パラメータ付きネスト関数は `vec![]` を返す（`check_module` が body を独立してチェックするが、キャプチャ意味論は解析しない）。

**影響**：パラメータ付きネスト関数の body 内の所有権エラーは検出されず（現在は直接 skip）、キャプチャ情報も生成されない。パラメータ付きネスト関数が外側の変数を使用した場合、所有権意味論が解析されない。

**修正方針**：パラメータあり/なしの両方の Binding を統一的に処理し、body に対して `check_function` + キャプチャ解析を同時に行う。
```